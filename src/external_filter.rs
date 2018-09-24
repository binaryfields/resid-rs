// This file is part of resid-rs.
// Copyright (c) 2017-2018 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use super::ChipModel;

/// Maximum mixer DC output level; to be removed if the external
/// filter is turned off: ((wave DC + voice DC)*voices + mixer DC)*volume
/// See voice.cc and filter.cc for an explanation of the values.
const MIXER_DC_6581: i32 = ((((0x800 - 0x380) + 0x800) * 0xff * 3 - 0xfff * 0xff / 18) >> 7) * 0x0f;

/// Low-pass:  R = 10kOhm, C = 1000pF; w0l = 1/RC = 1/(1e4*1e-9) = 100000
/// High-pass: R =  1kOhm, C =   10uF; w0h = 1/RC = 1/(1e3*1e-5) =    100
/// Multiply with 1.048576 to facilitate division by 1 000 000 by right-
/// shifting 20 times (2 ^ 20 = 1048576).
const W0_LP: i32 = 104858;
const W0_HP: i32 = 105;

/// The audio output stage in a Commodore 64 consists of two STC networks,
/// a low-pass filter with 3-dB frequency 16kHz followed by a high-pass
/// filter with 3-dB frequency 16Hz (the latter provided an audio equipment
/// input impedance of 1kOhm).
/// The STC networks are connected with a BJT supposedly meant to act as
/// a unity gain buffer, which is not really how it works. A more elaborate
/// model would include the BJT, however DC circuit analysis yields BJT
/// base-emitter and emitter-base impedances sufficiently low to produce
/// additional low-pass and high-pass 3dB-frequencies in the order of hundreds
/// of kHz. This calls for a sampling frequency of several MHz, which is far
/// too high for practical use.
pub struct ExternalFilter {
    // Configuration
    enabled: bool,
    mixer_dc: i32,
    w0lp: i32,
    w0hp: i32,
    // Runtime State
    vlp: i32,
    vhp: i32,
    vo: i32,
}

impl ExternalFilter {
    pub fn new(chip_model: ChipModel) -> Self {
        let mixer_dc = match chip_model {
            ChipModel::Mos6581 => MIXER_DC_6581,
            ChipModel::Mos8580 => 0,
        };
        let mut filter = ExternalFilter {
            enabled: true,
            mixer_dc,
            w0lp: W0_LP,
            w0hp: W0_HP,
            vlp: 0,
            vhp: 0,
            vo: 0,
        };
        filter.reset();
        filter
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    #[inline]
    pub fn clock(&mut self, vi: i32) {
        // delta_t is converted to seconds given a 1MHz clock by dividing
        // with 1 000 000.
        // Calculate filter outputs.
        // Vo  = Vlp - Vhp;
        // Vlp = Vlp + w0lp*(Vi - Vlp)*delta_t;
        // Vhp = Vhp + w0hp*(Vlp - Vhp)*delta_t;
        if self.enabled {
            let dvlp = ((self.w0lp >> 8) * (vi - self.vlp)) >> 12;
            let dvhp = (self.w0hp * (self.vlp - self.vhp)) >> 20;
            self.vo = self.vlp - self.vhp;
            self.vlp += dvlp;
            self.vhp += dvhp;
        } else {
            self.vlp = 0;
            self.vhp = 0;
            self.vo = vi - self.mixer_dc;
        }
    }

    #[inline]
    pub fn clock_delta(&mut self, mut delta: u32, vi: i32) {
        if self.enabled {
            // Maximum delta cycles for the external filter to work satisfactorily
            // is approximately 8.
            let mut delta_flt: u32 = 8;
            while delta != 0 {
                if delta < delta_flt {
                    delta_flt = delta;
                }
                // delta_t is converted to seconds given a 1MHz clock by dividing
                // with 1 000 000.
                // Calculate filter outputs.
                // Vo  = Vlp - Vhp;
                // Vlp = Vlp + w0lp*(Vi - Vlp)*delta_t;
                // Vhp = Vhp + w0hp*(Vlp - Vhp)*delta_t;
                let dvlp = (((self.w0lp * delta_flt as i32) >> 8) * (vi - self.vlp)) >> 12;
                let dvhp = (self.w0hp * delta_flt as i32 * (self.vlp - self.vhp)) >> 20;
                self.vo = self.vlp - self.vhp;
                self.vlp += dvlp;
                self.vhp += dvhp;
                delta -= delta_flt;
            }
        } else {
            self.vlp = 0;
            self.vhp = 0;
            self.vo = vi - self.mixer_dc;
        }
    }

    #[inline]
    pub fn output(&self) -> i32 {
        self.vo
    }

    pub fn reset(&mut self) {
        self.vlp = 0;
        self.vhp = 0;
        self.vo = 0;
    }
}
