// This file is part of resid-rs.
// Copyright (c) 2017-2019 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use super::external_filter::ExternalFilter;
use super::filter::Filter;
use super::sid::reg;
use super::voice::Voice;
use super::wave::Syncable;
use super::ChipModel;

const OUTPUT_RANGE: u32 = 1 << 16;
const OUTPUT_HALF: i32 = (OUTPUT_RANGE >> 1) as i32;
const SAMPLES_PER_OUTPUT: u32 = ((4095 * 255) >> 7) * 3 * 15 * 2 / OUTPUT_RANGE;

#[derive(Clone, Copy)]
pub struct Synth {
    pub ext_filter: ExternalFilter,
    pub filter: Filter,
    pub voices: [Voice; 3],
    pub ext_in: i32,
}

// slice::rotate_left is inefficient for small arrays:
// https://github.com/rust-lang/rust/issues/89714
fn rotate3<T>([a, b, c]: [T; 3], i: usize) -> [T; 3] {
    match i {
        0 => [a, b, c],
        1 => [b, c, a],
        2 => [c, a, b],
        _ => panic!("index out of bounds"),
    }
}

impl Synth {
    pub fn new(chip_model: ChipModel) -> Self {
        Synth {
            ext_filter: ExternalFilter::new(chip_model),
            filter: Filter::new(chip_model),
            voices: [Voice::new(chip_model); 3],
            ext_in: 0,
        }
    }

    pub fn syncable_voice(&self, i: usize) -> Syncable<&'_ Voice> {
        let [a, b, c] = &self.voices;
        let [main, sync_dest, sync_source] = rotate3([a, b, c], i);
        Syncable {
            main,
            sync_dest,
            sync_source,
        }
    }

    pub fn syncable_voice_mut(&mut self, i: usize) -> Syncable<&'_ mut Voice> {
        let [a, b, c] = &mut self.voices;
        let [main, sync_dest, sync_source] = rotate3([a, b, c], i);
        Syncable {
            main,
            sync_dest,
            sync_source,
        }
    }

    pub fn clock(&mut self) {
        // Clock amplitude modulators.
        for i in 0..3 {
            self.voices[i].envelope.clock();
        }
        // Clock oscillators.
        for i in 0..3 {
            self.voices[i].wave.clock();
        }
        // Synchronize oscillators.
        for i in 0..3 {
            self.syncable_voice_mut(i).wave().synchronize();
        }
        // Clock filter.
        self.filter.clock(
            self.syncable_voice(0).output(),
            self.syncable_voice(1).output(),
            self.syncable_voice(2).output(),
            self.ext_in,
        );
        // Clock external filter.
        self.ext_filter.clock(self.filter.output());
    }

    pub fn clock_delta(&mut self, delta: u32) {
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
                let wave = self.syncable_voice(i).wave();
                // It is only necessary to clock on the MSB of an oscillator that is
                // a sync source and has freq != 0.
                if !(wave.sync_dest.get_sync() && wave.main.get_frequency() != 0) {
                    continue;
                }
                let freq = wave.main.get_frequency() as u32;
                let acc = wave.main.get_acc();
                // Clock on MSB off if MSB is on, clock on MSB on if MSB is off.
                let delta_acc = if acc & 0x0080_0000 != 0 {
                    0x0100_0000 - acc
                } else {
                    0x0080_0000 - acc
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
                self.voices[i].wave.clock_delta(delta_min);
            }
            // Synchronize oscillators.
            for i in 0..3 {
                self.syncable_voice_mut(i).wave().synchronize();
            }
            delta_osc -= delta_min;
        }
        // Clock filter.
        self.filter.clock_delta(
            delta,
            self.syncable_voice(0).output(),
            self.syncable_voice(1).output(),
            self.syncable_voice(2).output(),
            self.ext_in,
        );
        // Clock external filter.
        self.ext_filter.clock_delta(delta, self.filter.output());
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

    pub fn reset(&mut self) {
        self.ext_filter.reset();
        self.filter.reset();
        for i in 0..3 {
            self.voices[i].reset();
        }
        self.ext_in = 0;
    }

    pub fn read(&self, reg: u8, bus_value: u8) -> u8 {
        match reg {
            reg::POTX => 0xff,
            reg::POTY => 0xff,
            reg::OSC3 => self.syncable_voice(2).wave().read_osc(),
            reg::ENV3 => self.voices[2].envelope.read_env(),
            _ => bus_value,
        }
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        match reg {
            reg::FREQLO1 => self.voices[0].wave.set_frequency_lo(value),
            reg::FREQHI1 => self.voices[0].wave.set_frequency_hi(value),
            reg::PWLO1 => self.voices[0].wave.set_pulse_width_lo(value),
            reg::PWHI1 => self.voices[0].wave.set_pulse_width_hi(value),
            reg::CR1 => self.voices[0].set_control(value),
            reg::AD1 => self.voices[0].envelope.set_attack_decay(value),
            reg::SR1 => self.voices[0].envelope.set_sustain_release(value),
            reg::FREQLO2 => self.voices[1].wave.set_frequency_lo(value),
            reg::FREQHI2 => self.voices[1].wave.set_frequency_hi(value),
            reg::PWLO2 => self.voices[1].wave.set_pulse_width_lo(value),
            reg::PWHI2 => self.voices[1].wave.set_pulse_width_hi(value),
            reg::CR2 => self.voices[1].set_control(value),
            reg::AD2 => self.voices[1].envelope.set_attack_decay(value),
            reg::SR2 => self.voices[1].envelope.set_sustain_release(value),
            reg::FREQLO3 => self.voices[2].wave.set_frequency_lo(value),
            reg::FREQHI3 => self.voices[2].wave.set_frequency_hi(value),
            reg::PWLO3 => self.voices[2].wave.set_pulse_width_lo(value),
            reg::PWHI3 => self.voices[2].wave.set_pulse_width_hi(value),
            reg::CR3 => self.voices[2].set_control(value),
            reg::AD3 => self.voices[2].envelope.set_attack_decay(value),
            reg::SR3 => self.voices[2].envelope.set_sustain_release(value),
            reg::FCLO => self.filter.set_fc_lo(value),
            reg::FCHI => self.filter.set_fc_hi(value),
            reg::RESFILT => self.filter.set_res_filt(value),
            reg::MODVOL => self.filter.set_mode_vol(value),
            _ => {}
        }
    }
}
