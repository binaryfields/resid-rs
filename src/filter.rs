// This file is part of resid-rs.
// Copyright (c) 2017-2019 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use core::f64;

use super::data::{SPLINE6581_F0, SPLINE8580_F0};
use super::spline;
use super::ChipModel;

const MIXER_DC: i32 = (-0xfff * 0xff / 18) >> 7;

/// The SID filter is modeled with a two-integrator-loop biquadratic filter,
/// which has been confirmed by Bob Yannes to be the actual circuit used in
/// the SID chip.
///
/// Measurements show that excellent emulation of the SID filter is achieved,
/// except when high resonance is combined with high sustain levels.
/// In this case the SID op-amps are performing less than ideally and are
/// causing some peculiar behavior of the SID filter. This however seems to
/// have more effect on the overall amplitude than on the color of the sound.
///
/// The theory for the filter circuit can be found in "Microelectric Circuits"
/// by Adel S. Sedra and Kenneth C. Smith.
/// The circuit is modeled based on the explanation found there except that
/// an additional inverter is used in the feedback from the bandpass output,
/// allowing the summer op-amp to operate in single-ended mode. This yields
/// inverted filter outputs with levels independent of Q, which corresponds with
/// the results obtained from a real SID.
///
/// We have been able to model the summer and the two integrators of the circuit
/// to form components of an IIR filter.
/// Vhp is the output of the summer, Vbp is the output of the first integrator,
/// and Vlp is the output of the second integrator in the filter circuit.
///
/// According to Bob Yannes, the active stages of the SID filter are not really
/// op-amps. Rather, simple NMOS inverters are used. By biasing an inverter
/// into its region of quasi-linear operation using a feedback resistor from
/// input to output, a MOS inverter can be made to act like an op-amp for
/// small signals centered around the switching threshold.
#[derive(Clone, Copy)]
pub struct Filter {
    // Configuration
    enabled: bool,
    fc: u16,
    filt: u8,
    res: u8,
    // Mode
    voice3_off: bool,
    hp_bp_lp: u8,
    vol: u8,
    // Runtime State
    pub vhp: i32,
    pub vbp: i32,
    pub vlp: i32,
    pub vnf: i32,
    // Cutoff Freq/Res
    mixer_dc: i32,
    q_1024_div: i32,
    w0: i32,
    w0_ceil_1: i32,
    w0_ceil_dt: i32,
    // Cutoff Freq Tables
    f0: &'static [i32; 2048],
}

impl Filter {
    pub fn new(chip_model: ChipModel) -> Self {
        let f0 = match chip_model {
            ChipModel::Mos6581 => &SPLINE6581_F0,
            ChipModel::Mos8580 => &SPLINE8580_F0,
        };
        let mut filter = Filter {
            enabled: true,
            fc: 0,
            filt: 0,
            res: 0,
            voice3_off: false,
            hp_bp_lp: 0,
            vol: 0,
            vhp: 0,
            vbp: 0,
            vlp: 0,
            vnf: 0,
            mixer_dc: MIXER_DC,
            q_1024_div: 0,
            w0: 0,
            w0_ceil_1: 0,
            w0_ceil_dt: 0,
            f0,
        };
        filter.set_q();
        filter.set_w0();
        filter
    }

    pub fn get_fc_hi(&self) -> u8 {
        (self.fc >> 3) as u8
    }

    pub fn get_fc_lo(&self) -> u8 {
        (self.fc & 0x007) as u8
    }

    pub fn get_mode_vol(&self) -> u8 {
        let value = if self.voice3_off { 0x80 } else { 0 };
        value | (self.hp_bp_lp << 4) | (self.vol & 0x0f)
    }

    pub fn get_res_filt(&self) -> u8 {
        (self.res << 4) | (self.filt & 0x0f)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_fc_hi(&mut self, value: u8) {
        let result = ((value as u16) << 3) & 0x7f8 | self.fc & 0x007;
        self.fc = result;
        self.set_w0();
    }

    pub fn set_fc_lo(&mut self, value: u8) {
        let result = self.fc & 0x7f8 | (value as u16) & 0x007;
        self.fc = result;
        self.set_w0();
    }

    pub fn set_mode_vol(&mut self, value: u8) {
        self.voice3_off = value & 0x80 != 0;
        self.hp_bp_lp = (value >> 4) & 0x07;
        self.vol = value & 0x0f;
    }

    pub fn set_res_filt(&mut self, value: u8) {
        self.res = (value >> 4) & 0x0f;
        self.filt = value & 0x0f;
        self.set_q();
    }

    #[inline]
    pub fn clock(&mut self, mut voice1: i32, mut voice2: i32, mut voice3: i32, mut ext_in: i32) {
        // Scale each voice down from 20 to 13 bits.
        voice1 >>= 7;
        voice2 >>= 7;
        // NB! Voice 3 is not silenced by voice3off if it is routed through
        // the filter.
        voice3 = if self.voice3_off && self.filt & 0x04 == 0 {
            0
        } else {
            voice3 >> 7
        };
        ext_in >>= 7;

        // This is handy for testing.
        if !self.enabled {
            self.vnf = voice1 + voice2 + voice3 + ext_in;
            self.vhp = 0;
            self.vbp = 0;
            self.vlp = 0;
            return;
        }

        // Route voices into or around filter.
        // The code below is expanded to a switch for faster execution.
        // (filt1 ? Vi : Vnf) += voice1;
        // (filt2 ? Vi : Vnf) += voice2;
        // (filt3 ? Vi : Vnf) += voice3;
        let vi = match self.filt {
            0x0 => {
                self.vnf = voice1 + voice2 + voice3 + ext_in;
                0
            }
            0x1 => {
                self.vnf = voice2 + voice3 + ext_in;
                voice1
            }
            0x2 => {
                self.vnf = voice1 + voice3 + ext_in;
                voice2
            }
            0x3 => {
                self.vnf = voice3 + ext_in;
                voice1 + voice2
            }
            0x4 => {
                self.vnf = voice1 + voice2 + ext_in;
                voice3
            }
            0x5 => {
                self.vnf = voice2 + ext_in;
                voice1 + voice3
            }
            0x6 => {
                self.vnf = voice1 + ext_in;
                voice2 + voice3
            }
            0x7 => {
                self.vnf = ext_in;
                voice1 + voice2 + voice3
            }
            0x8 => {
                self.vnf = voice1 + voice2 + voice3;
                ext_in
            }
            0x9 => {
                self.vnf = voice2 + voice3;
                voice1 + ext_in
            }
            0xa => {
                self.vnf = voice1 + voice3;
                voice2 + ext_in
            }
            0xb => {
                self.vnf = voice3;
                voice1 + voice2 + ext_in
            }
            0xc => {
                self.vnf = voice1 + voice2;
                voice3 + ext_in
            }
            0xd => {
                self.vnf = voice2;
                voice1 + voice3 + ext_in
            }
            0xe => {
                self.vnf = voice1;
                voice2 + voice3 + ext_in
            }
            0xf => {
                self.vnf = 0;
                voice1 + voice2 + voice3 + ext_in
            }
            _ => {
                self.vnf = voice1 + voice2 + voice3 + ext_in;
                0
            }
        };

        // delta_t = 1 is converted to seconds given a 1MHz clock by dividing
        // with 1 000 000.

        // Calculate filter outputs.
        // Vhp = Vbp/Q - Vlp - Vi;
        // dVbp = -w0*Vhp*dt;
        // dVlp = -w0*Vbp*dt;
        let dvbp = (self.w0_ceil_1 * self.vhp) >> 20;
        let dvlp = (self.w0_ceil_1 * self.vbp) >> 20;
        self.vbp -= dvbp;
        self.vlp -= dvlp;
        self.vhp = ((self.vbp * self.q_1024_div) >> 10) - self.vlp - vi;
    }

    #[inline]
    pub fn clock_delta(
        &mut self,
        mut delta: u32,
        mut voice1: i32,
        mut voice2: i32,
        mut voice3: i32,
        mut ext_in: i32,
    ) {
        // Scale each voice down from 20 to 13 bits.
        voice1 >>= 7;
        voice2 >>= 7;
        if self.voice3_off && self.filt & 0x04 == 0 {
            voice3 = 0;
        } else {
            voice3 >>= 7;
        }
        ext_in >>= 7;
        // Enable filter on/off.
        // This is not really part of SID, but is useful for testing.
        // On slow CPUs it may be necessary to bypass the filter to lower the CPU
        // load.
        if !self.enabled {
            self.vnf = voice1 + voice2 + voice3 + ext_in;
            self.vhp = 0;
            self.vbp = 0;
            self.vlp = 0;
            return;
        }

        // Route voices into or around filter.
        // The code below is expanded to a switch for faster execution.
        // (filt1 ? Vi : Vnf) += voice1;
        // (filt2 ? Vi : Vnf) += voice2;
        // (filt3 ? Vi : Vnf) += voice3;
        let vi = match self.filt {
            0x0 => {
                self.vnf = voice1 + voice2 + voice3 + ext_in;
                0
            }
            0x1 => {
                self.vnf = voice2 + voice3 + ext_in;
                voice1
            }
            0x2 => {
                self.vnf = voice1 + voice3 + ext_in;
                voice2
            }
            0x3 => {
                self.vnf = voice3 + ext_in;
                voice1 + voice2
            }
            0x4 => {
                self.vnf = voice1 + voice2 + ext_in;
                voice3
            }
            0x5 => {
                self.vnf = voice2 + ext_in;
                voice1 + voice3
            }
            0x6 => {
                self.vnf = voice1 + ext_in;
                voice2 + voice3
            }
            0x7 => {
                self.vnf = ext_in;
                voice1 + voice2 + voice3
            }
            0x8 => {
                self.vnf = voice1 + voice2 + voice3;
                ext_in
            }
            0x9 => {
                self.vnf = voice2 + voice3;
                voice1 + ext_in
            }
            0xa => {
                self.vnf = voice1 + voice3;
                voice2 + ext_in
            }
            0xb => {
                self.vnf = voice3;
                voice1 + voice2 + ext_in
            }
            0xc => {
                self.vnf = voice1 + voice2;
                voice3 + ext_in
            }
            0xd => {
                self.vnf = voice2;
                voice1 + voice3 + ext_in
            }
            0xe => {
                self.vnf = voice1;
                voice2 + voice3 + ext_in
            }
            0xf => {
                self.vnf = 0;
                voice1 + voice2 + voice3 + ext_in
            }
            _ => {
                self.vnf = voice1 + voice2 + voice3 + ext_in;
                0
            }
        };

        // Maximum delta cycles for the filter to work satisfactorily under current
        // cutoff frequency and resonance constraints is approximately 8.
        let mut delta_flt = 8;

        while delta != 0 {
            if delta < delta_flt {
                delta_flt = delta;
            }
            // delta_t is converted to seconds given a 1MHz clock by dividing
            // with 1 000 000. This is done in two operations to avoid integer
            // multiplication overflow.

            // Calculate filter outputs.
            // Vhp = Vbp/Q - Vlp - Vi;
            // dVbp = -w0*Vhp*dt;
            // dVlp = -w0*Vbp*dt;
            let w0_delta_t = (self.w0_ceil_dt * delta_flt as i32) >> 6;
            let dvbp = (w0_delta_t * self.vhp) >> 14;
            let dvlp = (w0_delta_t * self.vbp) >> 14;
            self.vbp -= dvbp;
            self.vlp -= dvlp;
            self.vhp = ((self.vbp * self.q_1024_div) >> 10) - self.vlp - vi;

            delta -= delta_flt;
        }
    }

    #[inline]
    pub fn output(&self) -> i32 {
        // This is handy for testing.
        if !self.enabled {
            (self.vnf + self.mixer_dc) * self.vol as i32
        } else {
            // Mix highpass, bandpass, and lowpass outputs. The sum is not
            // weighted, this can be confirmed by sampling sound output for
            // e.g. bandpass, lowpass, and bandpass+lowpass from a SID chip.
            // The code below is expanded to a switch for faster execution.
            // if (hp) Vf += Vhp;
            // if (bp) Vf += Vbp;
            // if (lp) Vf += Vlp;
            let vf = match self.hp_bp_lp {
                0x0 => 0,
                0x1 => self.vlp,
                0x2 => self.vbp,
                0x3 => self.vlp + self.vbp,
                0x4 => self.vhp,
                0x5 => self.vlp + self.vhp,
                0x6 => self.vbp + self.vhp,
                0x7 => self.vlp + self.vbp + self.vhp,
                _ => 0,
            };
            // Sum non-filtered and filtered output.
            // Multiply the sum with volume.
            (self.vnf + vf + self.mixer_dc) * self.vol as i32
        }
    }

    pub fn reset(&mut self) {
        self.fc = 0;
        self.filt = 0;
        self.res = 0;
        self.voice3_off = false;
        self.hp_bp_lp = 0;
        self.vol = 0;
        self.vhp = 0;
        self.vbp = 0;
        self.vlp = 0;
        self.vnf = 0;
        self.set_w0();
        self.set_q();
    }

    fn set_q(&mut self) {
        // Q is controlled linearly by res. Q has approximate range [0.707, 1.7].
        // As resonance is increased, the filter must be clocked more often to keep
        // stable.

        // The coefficient 1024 is dispensed of later by right-shifting 10 times
        // (2 ^ 10 = 1024).
        self.q_1024_div = (1024.0 / (0.707 + 1.0 * self.res as f64 / 15.0)) as i32;
    }

    fn set_w0(&mut self) {
        // Multiply with 1.048576 to facilitate division by 1 000 000 by right-
        // shifting 20 times (2 ^ 20 = 1048576).
        self.w0 = (2.0 * f64::consts::PI * self.f0[self.fc as usize] as f64 * 1.048_576) as i32;

        // Limit f0 to 16kHz to keep 1 cycle filter stable.
        let w0_max_1 = (2.0 * f64::consts::PI * 16000.0 * 1.048_576) as i32;
        self.w0_ceil_1 = if self.w0 <= w0_max_1 {
            self.w0
        } else {
            w0_max_1
        };

        // Limit f0 to 4kHz to keep delta_t cycle filter stable.
        let w0_max_dt = (2.0 * f64::consts::PI * 4000.0 * 1.048_576) as i32;
        self.w0_ceil_dt = if self.w0 <= w0_max_dt {
            self.w0
        } else {
            w0_max_dt
        };
    }
}
