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

use super::ChipModel;
use super::external_filter::ExternalFilter;
use super::filter::Filter;
use super::voice::Voice;

const FIXP_SHIFT: i32 = 16;
const FIXP_MASK: i32 = 0xffff;
const OUTPUT_RANGE: u32 = 1 << 16;
const OUTPUT_HALF: i32 = (OUTPUT_RANGE >> 1) as i32;
const SAMPLES_PER_OUTPUT: u32 = (((4095 * 255) >> 7) * 3 * 15 * 2 / OUTPUT_RANGE);

#[derive(Clone, Copy)]
pub enum Reg {
    FREQLO1,
    FREQHI1,
    PWLO1,
    PWHI1,
    CR1,
    AD1,
    SR1,
    FREQLO2,
    FREQHI2,
    PWLO2,
    PWHI2,
    CR2,
    AD2,
    SR2,
    FREQLO3,
    FREQHI3,
    PWLO3,
    PWHI3,
    CR3,
    AD3,
    SR3,
    FCLO,
    FCHI,
    RESFILT,
    MODVOL,
    POTX,
    POTY,
    OSC3,
    ENV3,
}

impl Reg {
    pub fn from(reg: u8) -> Reg {
        match reg {
            0x00 => Reg::FREQLO1,
            0x01 => Reg::FREQHI1,
            0x02 => Reg::PWLO1,
            0x03 => Reg::PWHI1,
            0x04 => Reg::CR1,
            0x05 => Reg::AD1,
            0x06 => Reg::SR1,
            0x07 => Reg::FREQLO2,
            0x08 => Reg::FREQHI2,
            0x09 => Reg::PWLO2,
            0x0a => Reg::PWHI2,
            0x0b => Reg::CR2,
            0x0c => Reg::AD2,
            0x0d => Reg::SR2,
            0x0e => Reg::FREQLO3,
            0x0f => Reg::FREQHI3,
            0x10 => Reg::PWLO3,
            0x11 => Reg::PWHI3,
            0x12 => Reg::CR3,
            0x13 => Reg::AD3,
            0x14 => Reg::SR3,
            0x15 => Reg::FCLO,
            0x16 => Reg::FCHI,
            0x17 => Reg::RESFILT,
            0x18 => Reg::MODVOL,
            0x19 => Reg::POTX,
            0x1a => Reg::POTY,
            0x1b => Reg::OSC3,
            0x1c => Reg::ENV3,
            _ => panic!("invalid reg {}", reg),
        }
    }

    #[allow(dead_code)]
    pub fn addr(&self) -> u8 {
        *self as u8
    }
}

#[derive(Debug)]
pub struct State {
    // Sid
    sid_register: [u8; 32],
    bus_value: u8,
    bus_value_ttl: u32,
    ext_in: i32,
    // Wave
    accumulator: [u32; 3],
    shift_register: [u32; 3],
    // Envelope
    envelope_counter: [u8; 3],
    exponential_counter: [u8; 3],
    exponential_counter_period: [u8; 3],
    hold_zero: [u8; 3],
    rate_counter: [u16; 3],
    rate_counter_period: [u16; 3],
}

pub struct Sid {
    // Functional Units
    ext_filter: ExternalFilter,
    filter: Filter,
    voices: [Voice; 3],
    // Runtime State
    bus_value: u8,
    bus_value_ttl: u32,
    ext_in: i32,
    // Sampling State
    cycles_per_sample: u32,
    sample_offset: i32,
    sample_prev: i16,
}

impl Sid {
    pub fn new(chip_model: ChipModel) -> Sid {
        let mut voice1 = Voice::new(chip_model);
        let mut voice2 = Voice::new(chip_model);
        let mut voice3 = Voice::new(chip_model);
        voice1.set_sync_source(&mut voice3);
        voice2.set_sync_source(&mut voice1);
        voice3.set_sync_source(&mut voice2);
        let mut sid = Sid {
            ext_filter: ExternalFilter::new(chip_model),
            filter: Filter::new(chip_model),
            voices: [voice1, voice2, voice3],
            bus_value: 0,
            bus_value_ttl: 0,
            ext_in: 0,
            cycles_per_sample: 0,
            sample_offset: 0,
            sample_prev: 0,
        };
        sid.set_sampling_parameters(985248, 44100);
        sid
    }

    pub fn clock(&mut self) {
        // Age bus value.
        if self.bus_value_ttl > 0 {
            self.bus_value_ttl -= 1;
            if self.bus_value_ttl == 0 {
                self.bus_value = 0;
            }
        }
        // Clock amplitude modulators.
        for i in 0..3 {
            self.voices[i].envelope.clock();
        }
        // Clock oscillators.
        for i in 0..3 {
            self.voices[i].wave.borrow_mut().clock();
        }
        // Synchronize oscillators.
        for i in 0..3 {
            self.voices[i].wave.borrow_mut().synchronize();
        }
        // Clock filter.
        self.filter.clock(self.voices[0].output(),
                          self.voices[1].output(),
                          self.voices[2].output(),
                          self.ext_in);
        // Clock external filter.
        self.ext_filter.clock(self.filter.output());
    }

    pub fn clock_delta(&mut self, delta: u32) {
        // Age bus value.
        if self.bus_value_ttl >= delta {
            self.bus_value_ttl -= delta;
        } else {
            self.bus_value_ttl = 0;
        }
        if self.bus_value_ttl == 0 {
            self.bus_value = 0;
        }
        // Clock amplitude modulators.
        for i in 0..3 {
            self.voices[i].envelope.clock_delta(delta);
        }
        let mut delta_osc = delta;
        while delta_osc != 0 {
            // Find minimum number of cycles to an oscillator accumulator MSB toggle.
            // We have to clock on each MSB on / MSB off for hard sync to operate
            // correctly.
            let mut delta_min = delta_osc;
            for i in 0..3 {
                let wave = &self.voices[i].wave;
                // It is only necessary to clock on the MSB of an oscillator that is
                // a sync source and has freq != 0.
                if !(wave.borrow().get_sync_dest_sync() && wave.borrow().get_frequency() != 0) {
                    continue;
                }
                let freq = wave.borrow().get_frequency() as u32;
                let acc = wave.borrow().get_acc();
                // Clock on MSB off if MSB is on, clock on MSB on if MSB is off.
                let delta_acc = if acc & 0x800000 != 0 {
                    0x1000000 - acc
                } else {
                    0x800000 - acc
                };
                let mut delta_next = delta_acc / freq;
                if delta_acc % freq != 0 {
                    delta_next += 1;
                }
                if delta_next < delta_min {
                    delta_min = delta_next;
                }
            }
            // Clock oscillators.
            for i in 0..3 {
                self.voices[i].wave.borrow_mut().clock_delta(delta_min);
            }
            // Synchronize oscillators.
            for i in 0..3 {
                self.voices[i].wave.borrow_mut().synchronize();
            }
            delta_osc -= delta_min;
        }
        // Clock filter.
        self.filter.clock_delta(delta,
                                self.voices[0].output(),
                                self.voices[1].output(),
                                self.voices[2].output(),
                                self.ext_in);
        // Clock external filter.
        self.ext_filter.clock_delta(delta, self.filter.output());
    }

    pub fn enable_external_filter(&mut self, enabled: bool) {
        self.ext_filter.set_enabled(enabled);
    }

    pub fn enable_filter(&mut self, enabled: bool) {
        self.filter.set_enabled(enabled);
    }

    pub fn input(&mut self, sample: i32) {
        // Voice outputs are 20 bits. Scale up to match three voices in order
        // to facilitate simulation of the MOS8580 "digi boost" hardware hack.
        self.ext_in = (sample << 4) * 3;
    }

    pub fn output(&self) -> i16 {
        // Read sample from audio output.
        let sample = self.ext_filter.output() / SAMPLES_PER_OUTPUT as i32;
        if sample >= OUTPUT_HALF {
            (OUTPUT_HALF - 1) as i16
        } else if sample < -OUTPUT_HALF {
            (-OUTPUT_HALF) as i16
        } else {
            sample as i16
        }
    }

    pub fn read_state(&self) -> State {
        let mut state = State {
            sid_register: [0; 32],
            bus_value: 0,
            bus_value_ttl: 0,
            ext_in: 0,
            accumulator: [0; 3],
            shift_register: [0; 3],
            envelope_counter: [0; 3],
            exponential_counter: [0; 3],
            exponential_counter_period: [0; 3],
            hold_zero: [0; 3],
            rate_counter: [0; 3],
            rate_counter_period: [0; 3],
        };
        for i in 0..32 {
            state.sid_register[i] = 0; // self.read(i as u8);
        }
        state.bus_value = self.bus_value;
        state.bus_value_ttl = self.bus_value_ttl;
        state.ext_in = self.ext_in;
        for i in 0..3 {
            state.accumulator[i] = self.voices[i].wave.borrow().get_acc();
            state.shift_register[i] = self.voices[i].wave.borrow().get_shift();
            state.envelope_counter[i] = self.voices[i].envelope.envelope_counter;
            state.exponential_counter[i] = self.voices[i].envelope.exponential_counter;
            state.exponential_counter_period[i] = self.voices[i].envelope.exponential_counter_period;
            state.hold_zero[i] = if self.voices[i].envelope.hold_zero { 1 } else { 0 };
            state.rate_counter[i] = self.voices[i].envelope.rate_counter;
            state.rate_counter_period[i] = self.voices[i].envelope.rate_period;
        }
        state
    }

    pub fn reset(&mut self) {
        self.ext_filter.reset();
        self.filter.reset();
        for i in 0..3 {
            self.voices[i].reset();
        }
        self.bus_value = 0;
        self.bus_value_ttl = 0;
        self.ext_in = 0;
        self.sample_offset = 0;
        self.sample_prev = 0;
    }

    // ----------------------------------------------------------------------------
    // SID clocking with audio sampling.
    // Fixpoint arithmetics is used.
    //
    // The example below shows how to clock the SID a specified amount of cycles
    // while producing audio output:
    //
    // while (delta_t) {
    //   bufindex += sid.clock(delta_t, buf + bufindex, buflength - bufindex);
    //   write(dsp, buf, bufindex*2);
    //   bufindex = 0;
    // }
    //
    // ----------------------------------------------------------------------------
    pub fn sample(&mut self,
                  delta: u32,
                  buffer: &mut [i16],
                  n: usize,
                  interleave: usize) -> (usize, u32) {
        self.sample_fast(delta, buffer, n, interleave)
    }

    // ----------------------------------------------------------------------------
    // SID clocking with audio sampling - delta clocking picking nearest sample.
    // ----------------------------------------------------------------------------
    fn sample_fast(&mut self,
                   mut delta: u32,
                   buffer: &mut [i16],
                   n: usize,
                   interleave: usize) -> (usize, u32) {
        let mut s = 0;
        loop {
            let next_sample_offset = self.sample_offset + self.cycles_per_sample as i32 + (1 << (FIXP_SHIFT - 1));
            let delta_sample = (next_sample_offset >> FIXP_SHIFT) as u32;
            if delta_sample > delta {
                break;
            }
            if s >= n {
                return (s, delta);
            }
            self.clock_delta(delta_sample);
            delta -= delta_sample;
            self.sample_offset = (next_sample_offset & FIXP_MASK) - (1 << (FIXP_SHIFT - 1));
            buffer[(s * interleave) as usize] = self.output();
            s += 1; // TODO check w/ ref impl
        }
        self.clock_delta(delta);
        self.sample_offset -= (delta as i32) << FIXP_SHIFT;
        delta = 0;
        (s, delta)
    }

    #[allow(dead_code)]
    fn sample_interpolate(&mut self,
                          mut delta_t: u32,
                          buffer: &mut [i16],
                          n: usize,
                          interleave: usize) -> (usize, u32) {
        let mut s = 0;
        loop {
            let next_sample_offset = self.sample_offset + self.cycles_per_sample as i32;
            let delta_t_sample = (next_sample_offset >> FIXP_SHIFT) as u32;
            if delta_t_sample > delta_t {
                break;
            }
            if s >= n {
                return (s, delta_t);
            }
            for _i in 0..(delta_t_sample - 1) {
                self.sample_prev = self.output();
                self.clock();
            }
            delta_t -= delta_t_sample;
            self.sample_offset = next_sample_offset & FIXP_MASK;
            let sample_now = self.output();
            buffer[s * interleave] = self.sample_prev + ((self.sample_offset * (sample_now - self.sample_prev) as i32) >> FIXP_SHIFT) as i16;
            s += 1; // TODO check w/ ref impl
            self.sample_prev = sample_now;
        }
        for _i in 0..(delta_t - 1) {
            self.clock();
        }
        self.sample_offset -= (delta_t as i32) << FIXP_SHIFT;
        delta_t = 0;
        (s, delta_t)
    }

    // ----------------------------------------------------------------------------
    // Setting of SID sampling parameters.
    //
    // Use a clock freqency of 985248Hz for PAL C64, 1022730Hz for NTSC C64.
    // The default end of passband frequency is pass_freq = 0.9*sample_freq/2
    // for sample frequencies up to ~ 44.1kHz, and 20kHz for higher sample
    // frequencies.
    //
    // For resampling, the ratio between the clock frequency and the sample
    // frequency is limited as follows:
    //   125*clock_freq/sample_freq < 16384
    // E.g. provided a clock frequency of ~ 1MHz, the sample frequency can not
    // be set lower than ~ 8kHz. A lower sample frequency would make the
    // resampling code overfill its 16k sample ring buffer.
    //
    // The end of passband frequency is also limited:
    //   pass_freq <= 0.9*sample_freq/2
    //
    // E.g. for a 44.1kHz sampling rate the end of passband frequency is limited
    // to slightly below 20kHz. This constraint ensures that the FIR table is
    // not overfilled.
    // ----------------------------------------------------------------------------
    pub fn set_sampling_parameters(&mut self, clock_freq: u32, sample_freq: u32) {
        self.cycles_per_sample = (clock_freq as f64 / sample_freq as f64 * (1 << FIXP_SHIFT) as f64 + 0.5) as u32;
        self.sample_offset = 0;
        self.sample_prev = 0;
    }

    // -- Device I/O

    pub fn read(&self, reg: u8) -> u8 {
        match Reg::from(reg) {
            Reg::POTX => 0,
            Reg::POTY => 0,
            Reg::OSC3 => self.voices[2].wave.borrow().read_osc(),
            Reg::ENV3 => self.voices[2].envelope.read_env(),
            _ => self.bus_value,
        }
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        self.bus_value = value;
        self.bus_value_ttl = 0x2000;
        match Reg::from(reg) {
            Reg::FREQLO1 => {
                self.voices[0].wave.borrow_mut().set_frequency_lo(value);
            }
            Reg::FREQHI1 => {
                self.voices[0].wave.borrow_mut().set_frequency_hi(value);
            }
            Reg::PWLO1 => {
                self.voices[0].wave.borrow_mut().set_pulse_width_lo(value);
            }
            Reg::PWHI1 => {
                self.voices[0].wave.borrow_mut().set_pulse_width_hi(value);
            }
            Reg::CR1 => {
                self.voices[0].set_control(value);
            }
            Reg::AD1 => {
                self.voices[0].envelope.set_attack_decay(value);
            }
            Reg::SR1 => {
                self.voices[0].envelope.set_sustain_release(value);
            }
            Reg::FREQLO2 => {
                self.voices[1].wave.borrow_mut().set_frequency_lo(value);
            }
            Reg::FREQHI2 => {
                self.voices[1].wave.borrow_mut().set_frequency_hi(value);
            }
            Reg::PWLO2 => {
                self.voices[1].wave.borrow_mut().set_pulse_width_lo(value);
            }
            Reg::PWHI2 => {
                self.voices[1].wave.borrow_mut().set_pulse_width_hi(value);
            }
            Reg::CR2 => {
                self.voices[1].set_control(value);
            }
            Reg::AD2 => {
                self.voices[1].envelope.set_attack_decay(value);
            }
            Reg::SR2 => {
                self.voices[1].envelope.set_sustain_release(value);
            }
            Reg::FREQLO3 => {
                self.voices[2].wave.borrow_mut().set_frequency_lo(value);
            }
            Reg::FREQHI3 => {
                self.voices[2].wave.borrow_mut().set_frequency_hi(value);
            }
            Reg::PWLO3 => {
                self.voices[2].wave.borrow_mut().set_pulse_width_lo(value);
            }
            Reg::PWHI3 => {
                self.voices[2].wave.borrow_mut().set_pulse_width_hi(value);
            }
            Reg::CR3 => {
                self.voices[2].set_control(value);
            }
            Reg::AD3 => {
                self.voices[2].envelope.set_attack_decay(value);
            }
            Reg::SR3 => {
                self.voices[2].envelope.set_sustain_release(value);
            }
            Reg::FCLO => {
                self.filter.set_fc_lo(value);
            }
            Reg::FCHI => {
                self.filter.set_fc_hi(value);
            }
            Reg::RESFILT => {
                self.filter.set_res_filt(value);
            }
            Reg::MODVOL => {
                self.filter.set_mode_vol(value);
            }
            _ => {}
        }
    }
}
