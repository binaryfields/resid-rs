// This file is part of resid-rs.
// Copyright (c) 2017-2019 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use super::external_filter::ExternalFilter;
use super::filter::Filter;
use super::voice::Voice;
use super::ChipModel;

const OUTPUT_RANGE: u32 = 1 << 16;
const OUTPUT_HALF: i32 = (OUTPUT_RANGE >> 1) as i32;
const SAMPLES_PER_OUTPUT: u32 = (((4095 * 255) >> 7) * 3 * 15 * 2 / OUTPUT_RANGE);

pub struct Synth {
    pub ext_filter: ExternalFilter,
    pub filter: Filter,
    pub voices: [Voice; 3],
    pub ext_in: i32,
}

impl Synth {
    pub fn new(chip_model: ChipModel) -> Self {
        let mut voice1 = Voice::new(chip_model);
        let mut voice2 = Voice::new(chip_model);
        let mut voice3 = Voice::new(chip_model);
        voice1.set_sync_source(&mut voice3);
        voice2.set_sync_source(&mut voice1);
        voice3.set_sync_source(&mut voice2);
        Synth {
            ext_filter: ExternalFilter::new(chip_model),
            filter: Filter::new(chip_model),
            voices: [voice1, voice2, voice3],
            ext_in: 0,
        }
    }

    pub fn clock(&mut self) {
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
        self.filter.clock(
            self.voices[0].output(),
            self.voices[1].output(),
            self.voices[2].output(),
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
                let wave = &self.voices[i].wave;
                // It is only necessary to clock on the MSB of an oscillator that is
                // a sync source and has freq != 0.
                if !(wave.borrow().get_sync_dest_sync() && wave.borrow().get_frequency() != 0) {
                    continue;
                }
                let freq = wave.borrow().get_frequency() as u32;
                let acc = wave.borrow().get_acc();
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
                self.voices[i].wave.borrow_mut().clock_delta(delta_min);
            }
            // Synchronize oscillators.
            for i in 0..3 {
                self.voices[i].wave.borrow_mut().synchronize();
            }
            delta_osc -= delta_min;
        }
        // Clock filter.
        self.filter.clock_delta(
            delta,
            self.voices[0].output(),
            self.voices[1].output(),
            self.voices[2].output(),
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
}
