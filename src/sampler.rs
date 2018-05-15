/*
 * Copyright (c) 2017 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
 * Portions (c) 2004 Dag Lem <resid@nimrod.no>
 *
 * This file is part of resid-rs.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::cmp;

use super::Sid;

// Resampling constants.
// The error in interpolated lookup is bounded by 1.234/L^2,
// while the error in non-interpolated lookup is bounded by
// 0.7854/L + 0.4113/L^2, see
// http://www-ccrma.stanford.edu/~jos/resample/Choice_Table_Size.html
// For a resolution of 16 bits this yields L >= 285 and L >= 51473,
// respectively.
const FIR_RES_FAST: i32 = 51473;
const FIR_RES_INTERPOLATE: i32 = 285;
const FIR_SHIFT: i32 = 15;
const RINGSIZE: usize = 16384;

const FIXP_SHIFT: i32 = 16;
const FIXP_MASK: i32 = 0xffff;

#[derive(Clone, Copy, PartialEq)]
pub enum SamplingMethod {
    Fast,
    Interpolate,
    Resample,
    ResampleFast,
}

pub struct Sampler {
    // Configuration
    cycles_per_sample: u32,
    fir: Vec<i16>,
    fir_n: i32,
    fir_res: i32,
    sampling_method: SamplingMethod,
    // Runtime State
    sample_buffer: [i16; RINGSIZE * 2],
    sample_index: usize,
    sample_offset: i32,
    sample_prev: i16,
}

impl Sampler {
    pub fn new() -> Sampler {
        Sampler {
            cycles_per_sample: 0,
            fir: Vec::new(),
            fir_n: 0,
            fir_res: 0,
            sampling_method: SamplingMethod::Fast,
            sample_buffer: [0; RINGSIZE * 2],
            sample_index: 0,
            sample_offset: 0,
            sample_prev: 0,
        }
    }

    pub fn set_parameters(&mut self, method: SamplingMethod, clock_freq: u32, sample_freq: u32) {
        self.cycles_per_sample =
            (clock_freq as f64 / sample_freq as f64 * (1 << FIXP_SHIFT) as f64 + 0.5) as u32;
        self.sampling_method = method;
        if self.sampling_method == SamplingMethod::Resample
            || self.sampling_method == SamplingMethod::ResampleFast
        {
            self.init_fir(clock_freq as f64, sample_freq as f64, -1.0, 0.97);
        }
        // Clear state
        for j in 0..RINGSIZE * 2 {
            self.sample_buffer[j] = 0;
        }
        self.sample_index = 0;
        self.sample_offset = 0;
        self.sample_prev = 0;
    }

    pub fn reset(&mut self) {
        self.sample_index = 0;
        self.sample_offset = 0;
        self.sample_prev = 0;
    }

    #[inline]
    pub fn sample(
        &mut self,
        sid: &mut Sid,
        delta: u32,
        buffer: &mut [i16],
        n: usize,
        interleave: usize,
    ) -> (usize, u32) {
        match self.sampling_method {
            SamplingMethod::Fast => {
                self.clock_fast(sid, delta, buffer, n, interleave)
            },
            SamplingMethod::Interpolate => {
                self.clock_interpolate(sid, delta, buffer, n, interleave)
            }
            SamplingMethod::Resample => {
                self.clock_resample_interpolate(sid, delta, buffer, n, interleave)
            }
            SamplingMethod::ResampleFast => {
                self.clock_resample_fast(sid, delta, buffer, n, interleave)
            }
        }
    }

    // ----------------------------------------------------------------------------
    // SID clocking with audio sampling - delta clocking picking nearest sample.
    // ----------------------------------------------------------------------------
    #[inline]
    fn clock_fast(
        &mut self,
        sid: &mut Sid,
        mut delta: u32,
        buffer: &mut [i16],
        n: usize,
        interleave: usize,
    ) -> (usize, u32) {
        let mut index = 0;
        loop {
            let next_sample_offset = self.get_next_sample_offset();
            let delta_sample = (next_sample_offset >> FIXP_SHIFT) as u32;
            if delta_sample > delta || index >= n {
                break;
            }
            sid.clock_delta(delta_sample);
            delta -= delta_sample;
            buffer[(index * interleave) as usize] = sid.output();
            index += 1;
            self.update_sample_offset(next_sample_offset);
        }
        if delta > 0 && index < n {
            sid.clock_delta(delta);
            self.sample_offset -= (delta as i32) << FIXP_SHIFT;
            (index, 0)
        } else {
            (index, delta)
        }
    }

    #[inline]
    fn clock_interpolate(
        &mut self,
        sid: &mut Sid,
        mut delta: u32,
        buffer: &mut [i16],
        n: usize,
        interleave: usize,
    ) -> (usize, u32) {
        let mut index = 0;
        loop {
            let next_sample_offset = self.get_next_sample_offset();
            let delta_sample = (next_sample_offset >> FIXP_SHIFT) as u32;
            if delta_sample > delta || index >= n {
                break;
            }
            for _i in 0..(delta_sample - 1) {
                self.sample_prev = sid.output();
                sid.clock();
            }
            delta -= delta_sample;
            let sample_now = sid.output();
            buffer[index * interleave] = self.sample_prev
                + ((self.sample_offset * (sample_now - self.sample_prev) as i32) >> FIXP_SHIFT)
                    as i16;
            index += 1;
            self.sample_prev = sample_now;
            self.update_sample_offset(next_sample_offset);
        }
        if delta > 0 && index < n {
            for _i in 0..(delta - 1) {
                sid.clock();
            }
            self.sample_offset -= (delta as i32) << FIXP_SHIFT;
            (index, 0)
        } else {
            (index, delta)
        }
    }

    // ----------------------------------------------------------------------------
    // SID clocking with audio sampling - cycle based with audio resampling.
    //
    // This is the theoretically correct (and computationally intensive) audio
    // sample generation. The samples are generated by resampling to the specified
    // sampling frequency. The work rate is inversely proportional to the
    // percentage of the bandwidth allocated to the filter transition band.
    //
    // This implementation is based on the paper "A Flexible Sampling-Rate
    // Conversion Method", by J. O. Smith and P. Gosset, or rather on the
    // expanded tutorial on the "Digital Audio Resampling Home Page":
    // http://www-ccrma.stanford.edu/~jos/resample/
    //
    // By building shifted FIR tables with samples according to the
    // sampling frequency, this implementation dramatically reduces the
    // computational effort in the filter convolutions, without any loss
    // of accuracy. The filter convolutions are also vectorizable on
    // current hardware.
    //
    // Further possible optimizations are:
    // * An equiripple filter design could yield a lower filter order, see
    //   http://www.mwrf.com/Articles/ArticleID/7229/7229.html
    // * The Convolution Theorem could be used to bring the complexity of
    //   convolution down from O(n*n) to O(n*log(n)) using the Fast Fourier
    //   Transform, see http://en.wikipedia.org/wiki/Convolution_theorem
    // * Simply resampling in two steps can also yield computational
    //   savings, since the transition band will be wider in the first step
    //   and the required filter order is thus lower in this step.
    //   Laurent Ganier has found the optimal intermediate sampling frequency
    //   to be (via derivation of sum of two steps):
    //     2 * pass_freq + sqrt [ 2 * pass_freq * orig_sample_freq
    //       * (dest_sample_freq - 2 * pass_freq) / dest_sample_freq ]
    //
    // NB! the result of right shifting negative numbers is really
    // implementation dependent in the C++ standard.
    // ----------------------------------------------------------------------------
    #[inline]
    fn clock_resample_interpolate(
        &mut self,
        sid: &mut Sid,
        mut delta: u32,
        buffer: &mut [i16],
        n: usize,
        interleave: usize,
    ) -> (usize, u32) {
        let mut index = 0;
        let half = 1i32 << 15;
        loop {
            let next_sample_offset = self.get_next_sample_offset2();
            let delta_sample = (next_sample_offset >> FIXP_SHIFT) as u32;
            if delta_sample > delta || index >= n {
                break;
            }

            for _i in 0..delta_sample {
                sid.clock();
                let output = sid.output();
                self.sample_buffer[self.sample_index] = output;
                self.sample_buffer[self.sample_index + RINGSIZE] = output;
                self.sample_index += 1;
                self.sample_index &= 0x3fff;
            }
            delta -= delta_sample;
            self.update_sample_offset2(next_sample_offset);

            let fir_offset_1 = (self.sample_offset * self.fir_res) >> FIXP_SHIFT;
            let fir_offset_rmd = (self.sample_offset * self.fir_res) & FIXP_MASK;
            let fir_start_1 = (fir_offset_1 * self.fir_n) as usize;
            let fir_end_1 = fir_start_1 + self.fir_n as usize;
            let sample_start_1 = (self.sample_index as i32 - self.fir_n + RINGSIZE as i32) as usize;
            let sample_end_1 = sample_start_1 + self.fir_n as usize;

            // Convolution with filter impulse response.
            let v1 = self.compute_convolution_fir(
                &self.sample_buffer[sample_start_1..sample_end_1],
                &self.fir[fir_start_1..fir_end_1],
            );

            // Use next FIR table, wrap around to first FIR table using
            // previous sample.
            let mut fir_offset_2 = fir_offset_1 + 1;
            let mut sample_start_2 = sample_start_1;
            if fir_offset_2 == self.fir_res {
                fir_offset_2 = 0;
                sample_start_2 -= 1;
            }
            let fir_start_2 = (fir_offset_2 * self.fir_n) as usize;
            let fir_end_2 = fir_start_2 + self.fir_n as usize;
            let sample_end_2 = sample_start_2 + self.fir_n as usize;

            let v2 = self.compute_convolution_fir(
                &self.sample_buffer[sample_start_2..sample_end_2],
                &self.fir[fir_start_2..fir_end_2],
            );

            // Linear interpolation.
            // fir_offset_rmd is equal for all samples, it can thus be factorized out:
            // sum(v1 + rmd*(v2 - v1)) = sum(v1) + rmd*(sum(v2) - sum(v1))
            let mut v = v1 + (fir_offset_rmd * (v2 - v1) >> FIXP_SHIFT);
            v >>= FIR_SHIFT;

            // Saturated arithmetics to guard against 16 bit sample overflow.
            if v >= half {
                v = half - 1;
            } else if v < -half {
                v = -half;
            }

            buffer[index * interleave] = v as i16;
            index += 1;
        }
        if delta > 0 && index < n {
            for _i in 0..delta {
                sid.clock();
                let output = sid.output();
                self.sample_buffer[self.sample_index] = output;
                self.sample_buffer[self.sample_index + RINGSIZE] = output;
                self.sample_index += 1;
                self.sample_index &= 0x3fff;
            }
            self.sample_offset -= (delta as i32) << FIXP_SHIFT;
            (index, 0)
        } else {
            (index, delta)
        }
    }

    // ----------------------------------------------------------------------------
    // SID clocking with audio sampling - cycle based with audio resampling.
    // ----------------------------------------------------------------------------
    #[inline]
    fn clock_resample_fast(
        &mut self,
        sid: &mut Sid,
        mut delta: u32,
        buffer: &mut [i16],
        n: usize,
        interleave: usize,
    ) -> (usize, u32) {
        let mut index = 0;
        let half = 1i32 << 15;
        loop {
            let next_sample_offset = self.get_next_sample_offset2();
            let delta_sample = (next_sample_offset >> FIXP_SHIFT) as u32;
            if delta_sample > delta || index >= n {
                break;
            }

            for _i in 0..delta_sample {
                sid.clock();
                let output = sid.output();
                self.sample_buffer[self.sample_index] = output;
                self.sample_buffer[self.sample_index + RINGSIZE] = output;
                self.sample_index += 1;
                self.sample_index &= 0x3fff;
            }
            delta -= delta_sample;
            self.update_sample_offset2(next_sample_offset);

            let fir_offset = (self.sample_offset * self.fir_res) >> FIXP_SHIFT;
            let fir_start = (fir_offset * self.fir_n) as usize;
            let fir_end = fir_start + self.fir_n as usize;
            let sample_start = (self.sample_index as i32 - self.fir_n + RINGSIZE as i32) as usize;
            let sample_end = sample_start + self.fir_n as usize;

            // Convolution with filter impulse response.
            let mut v = self.compute_convolution_fir(
                &self.sample_buffer[sample_start..sample_end],
                &self.fir[fir_start..fir_end],
            );
            v >>= FIR_SHIFT;

            // Saturated arithmetics to guard against 16 bit sample overflow.
            if v >= half {
                v = half - 1;
            } else if v < -half {
                v = -half;
            }

            buffer[index * interleave] = v as i16;
            index += 1;
        }
        if delta > 0 && index < n {
            for _i in 0..delta {
                sid.clock();
                let output = sid.output();
                self.sample_buffer[self.sample_index] = output;
                self.sample_buffer[self.sample_index + RINGSIZE] = output;
                self.sample_index += 1;
                self.sample_index &= 0x3fff;
            }
            self.sample_offset -= (delta as i32) << FIXP_SHIFT;
            (index, 0)
        } else {
            (index, delta)
        }
    }

    #[inline]
    fn compute_convolution_fir(&self, sample: &[i16], fir: &[i16]) -> i32 {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { self.compute_convolution_fir_avx2(sample, fir) };
            }
        }
        self.compute_convolution_fir_fallback(sample, fir)
    }

    #[target_feature(enable = "avx2")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe fn compute_convolution_fir_avx2(&self, sample: &[i16], fir: &[i16]) -> i32 {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::*;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::*;

        // Convolution with filter impulse response.
        let len = cmp::min(sample.len(), fir.len());
        let mut fs = &fir[..len];
        let mut ss = &sample[..len];
        let mut va = [0i32; 8];
        let mut v = _mm256_set1_epi32(0);
        while fs.len() >= 64 {
            let sv1 = _mm256_loadu_si256(ss.as_ptr() as *const _);
            let sv2 = _mm256_loadu_si256((&ss[16..]).as_ptr() as *const _);
            let sv3 = _mm256_loadu_si256((&ss[32..]).as_ptr() as *const _);
            let sv4 = _mm256_loadu_si256((&ss[48..]).as_ptr() as *const _);
            let fv1 = _mm256_loadu_si256(fs.as_ptr() as *const _);
            let prod = _mm256_madd_epi16(sv1, fv1);
            v = _mm256_add_epi32(v, prod);
            let fv2 = _mm256_loadu_si256((&fs[16..]).as_ptr() as *const _);
            let prod = _mm256_madd_epi16(sv2, fv2);
            v = _mm256_add_epi32(v, prod);
            let fv3 = _mm256_loadu_si256((&fs[32..]).as_ptr() as *const _);
            let prod = _mm256_madd_epi16(sv3, fv3);
            v = _mm256_add_epi32(v, prod);
            let fv4 = _mm256_loadu_si256((&fs[48..]).as_ptr() as *const _);
            let prod = _mm256_madd_epi16(sv4, fv4);
            v = _mm256_add_epi32(v, prod);
            fs = &fs[64..];
            ss = &ss[64..];
        }
        _mm256_storeu_si256(va[..].as_mut_ptr() as *mut _, v);
        let mut v = va[0] + va[1] + va[2] + va[3] + va[4] + va[5] + va[6] + va[7];
        for i in 0..fs.len() {
            v += ss[i] as i32 * fs[i] as i32;
        }
        v
    }

    #[inline]
    fn compute_convolution_fir_fallback(&self, sample: &[i16], fir: &[i16]) -> i32 {
        if sample.len() < fir.len() {
            sample.iter().zip(fir.iter())
                .fold(0, |sum, (&s, &f)| sum + (s as i32 * f as i32))
        } else {
            fir.iter().zip(sample.iter())
                .fold(0, |sum, (&f, &s)| sum + (f as i32 * s as i32))
        }
    }

    #[inline]
    fn get_next_sample_offset(&self) -> i32 {
        self.sample_offset + self.cycles_per_sample as i32 + (1 << (FIXP_SHIFT - 1))
    }

    #[inline]
    fn get_next_sample_offset2(&self) -> i32 {
        self.sample_offset + self.cycles_per_sample as i32
    }

    #[inline]
    fn update_sample_offset(&mut self, next_sample_offset: i32) {
        self.sample_offset = (next_sample_offset & FIXP_MASK) - (1 << (FIXP_SHIFT - 1));
    }

    #[inline]
    fn update_sample_offset2(&mut self, next_sample_offset: i32) {
        self.sample_offset = next_sample_offset & FIXP_MASK;
    }

    fn init_fir(
        &mut self,
        clock_freq: f64,
        sample_freq: f64,
        mut pass_freq: f64,
        filter_scale: f64,
    ) {
        let pi = 3.1415926535897932385;
        let samples_per_cycle = sample_freq / clock_freq;
        let cycles_per_sample = clock_freq / sample_freq;

        // The default passband limit is 0.9*sample_freq/2 for sample
        // frequencies below ~ 44.1kHz, and 20kHz for higher sample frequencies.
        if pass_freq < 0.0 {
            pass_freq = 20000.0;
            if 2.0 * pass_freq / sample_freq >= 0.9 {
                pass_freq = 0.9 * sample_freq / 2.0;
            }
        }

        // 16 bits -> -96dB stopband attenuation.
        let atten = -20.0f64 * (1.0 / (1i32 << 16) as f64).log10();
        // A fraction of the bandwidth is allocated to the transition band,
        let dw = (1.0f64 - 2.0 * pass_freq / sample_freq) * pi;
        // The cutoff frequency is midway through the transition band.
        let wc = (2.0f64 * pass_freq / sample_freq + 1.0) * pi / 2.0;

        // For calculation of beta and N see the reference for the kaiserord
        // function in the MATLAB Signal Processing Toolbox:
        // http://www.mathworks.com/access/helpdesk/help/toolbox/signal/kaiserord.html
        let beta = 0.1102f64 * (atten - 8.7);
        let io_beta = self.i0(beta);

        // The filter order will maximally be 124 with the current constraints.
        // N >= (96.33 - 7.95)/(2.285*0.1*pi) -> N >= 123
        // The filter order is equal to the number of zero crossings, i.e.
        // it should be an even number (sinc is symmetric about x = 0).
        let mut n_cap = ((atten - 7.95) / (2.285 * dw) + 0.5) as i32;
        n_cap += n_cap & 1;

        // The filter length is equal to the filter order + 1.
        // The filter length must be an odd number (sinc is symmetric about x = 0).
        self.fir_n = (n_cap as f64 * cycles_per_sample) as i32 + 1;
        self.fir_n |= 1;

        // We clamp the filter table resolution to 2^n, making the fixpoint
        // sample_offset a whole multiple of the filter table resolution.
        let res = if self.sampling_method == SamplingMethod::Resample {
            FIR_RES_INTERPOLATE
        } else {
            FIR_RES_FAST
        };
        let n = ((res as f64 / cycles_per_sample).ln() / (2.0f64).ln()).ceil() as i32;
        self.fir_res = 1 << n;

        self.fir.clear();
        self.fir.resize((self.fir_n * self.fir_res) as usize, 0);

        // Calculate fir_RES FIR tables for linear interpolation.
        for i in 0..self.fir_res {
            let fir_offset = i * self.fir_n + self.fir_n / 2;
            let j_offset = i as f64 / self.fir_res as f64;
            // Calculate FIR table. This is the sinc function, weighted by the
            // Kaiser window.
            let fir_n_div2 = self.fir_n / 2;
            for j in -fir_n_div2..fir_n_div2 + 1 {
                let jx = j as f64 - j_offset;
                let wt = wc * jx / cycles_per_sample;
                let temp = jx / fir_n_div2 as f64;
                let kaiser = if temp.abs() <= 1.0 {
                    self.i0(beta * (1.0 - temp * temp).sqrt()) / io_beta
                } else {
                    0f64
                };
                let sincwt = if wt.abs() >= 1e-6 { wt.sin() / wt } else { 1.0 };
                let val = (1i32 << FIR_SHIFT) as f64 * filter_scale * samples_per_cycle * wc / pi
                    * sincwt * kaiser;
                self.fir[(fir_offset + j) as usize] = (val + 0.5) as i16;
            }
        }
    }

    fn i0(&self, x: f64) -> f64 {
        // Max error acceptable in I0.
        let i0e = 1e-6;
        let halfx = x / 2.0;
        let mut sum = 1.0;
        let mut u = 1.0;
        let mut n = 1;
        loop {
            let temp = halfx / n as f64;
            n += 1;
            u *= temp * temp;
            sum += u;
            if !(u >= i0e * sum) {
                break;
            }
        }
        sum
    }
}
