// This file is part of resid-rs.
// Copyright (c) 2017-2019 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use super::envelope::State as EnvState;
use super::sampler::{Sampler, SamplingMethod};
use super::synth::Synth;
use super::ChipModel;

pub mod reg {
    pub const FREQLO1: u8 = 0x00;
    pub const FREQHI1: u8 = 0x01;
    pub const PWLO1: u8 = 0x02;
    pub const PWHI1: u8 = 0x03;
    pub const CR1: u8 = 0x04;
    pub const AD1: u8 = 0x05;
    pub const SR1: u8 = 0x06;
    pub const FREQLO2: u8 = 0x07;
    pub const FREQHI2: u8 = 0x08;
    pub const PWLO2: u8 = 0x09;
    pub const PWHI2: u8 = 0x0a;
    pub const CR2: u8 = 0x0b;
    pub const AD2: u8 = 0x0c;
    pub const SR2: u8 = 0x0d;
    pub const FREQLO3: u8 = 0x0e;
    pub const FREQHI3: u8 = 0x0f;
    pub const PWLO3: u8 = 0x10;
    pub const PWHI3: u8 = 0x11;
    pub const CR3: u8 = 0x12;
    pub const AD3: u8 = 0x13;
    pub const SR3: u8 = 0x14;
    pub const FCLO: u8 = 0x15;
    pub const FCHI: u8 = 0x16;
    pub const RESFILT: u8 = 0x17;
    pub const MODVOL: u8 = 0x18;
    pub const POTX: u8 = 0x19;
    pub const POTY: u8 = 0x1a;
    pub const OSC3: u8 = 0x1b;
    pub const ENV3: u8 = 0x1c;
}

#[derive(Debug)]
pub struct State {
    // Sid
    pub sid_register: [u8; 32],
    pub bus_value: u8,
    pub bus_value_ttl: u32,
    pub ext_in: i32,
    // Wave
    pub accumulator: [u32; 3],
    pub shift_register: [u32; 3],
    // Envelope
    pub envelope_state: [u8; 3],
    pub envelope_counter: [u8; 3],
    pub exponential_counter: [u8; 3],
    pub exponential_counter_period: [u8; 3],
    pub hold_zero: [u8; 3],
    pub rate_counter: [u16; 3],
    pub rate_counter_period: [u16; 3],
}

#[derive(Clone)]
pub struct Sid {
    // Functional Units
    sampler: Sampler,
    // Runtime State
    bus_value: u8,
    bus_value_ttl: u32,
}

impl Sid {
    pub fn new(chip_model: ChipModel) -> Self {
        let synth = Synth::new(chip_model);
        let mut sid = Sid {
            sampler: Sampler::new(synth),
            bus_value: 0,
            bus_value_ttl: 0,
        };
        sid.set_sampling_parameters(SamplingMethod::Fast, 985_248, 44100);
        sid
    }

    pub fn set_sampling_parameters(
        &mut self,
        method: SamplingMethod,
        clock_freq: u32,
        sample_freq: u32,
    ) {
        self.sampler.set_parameters(method, clock_freq, sample_freq);
    }

    pub fn clock(&mut self) {
        // Age bus value.
        if self.bus_value_ttl > 0 {
            self.bus_value_ttl -= 1;
            if self.bus_value_ttl == 0 {
                self.bus_value = 0;
            }
        }
        // Clock synthesizer.
        self.sampler.synth.clock();
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
        // Clock synthesizer.
        self.sampler.synth.clock_delta(delta);
    }

    pub fn enable_external_filter(&mut self, enabled: bool) {
        self.sampler.synth.ext_filter.set_enabled(enabled);
    }

    pub fn enable_filter(&mut self, enabled: bool) {
        self.sampler.synth.filter.set_enabled(enabled);
    }

    pub fn input(&mut self, sample: i32) {
        // Voice outputs are 20 bits. Scale up to match three voices in order
        // to facilitate simulation of the MOS8580 "digi boost" hardware hack.
        self.sampler.synth.ext_in = (sample << 4) * 3;
    }

    pub fn output(&self) -> i16 {
        self.sampler.synth.output()
    }

    pub fn reset(&mut self) {
        self.sampler.reset();
        self.bus_value = 0;
        self.bus_value_ttl = 0;
    }

    /// SID clocking with audio sampling.
    /// Fixpoint arithmetics is used.
    ///
    /// The example below shows how to clock the SID a specified amount of cycles
    /// while producing audio output:
    /// ``` ignore,
    /// let mut buffer = [0i16; 8192];
    /// while delta > 0 {
    ///     let (samples, next_delta) = self.resid.sample(delta, &mut buffer[..], 1);
    ///     let mut output = self.sound_buffer.lock().unwrap();
    ///     for i in 0..samples {
    ///         output.write(buffer[i]);
    ///     }
    ///     delta = next_delta;
    /// }
    /// ```
    pub fn sample(&mut self, delta: u32, buffer: &mut [i16], interleave: usize) -> (usize, u32) {
        self.sampler.clock(delta, buffer, interleave)
    }

    // -- Device I/O

    pub fn read(&self, reg: u8) -> u8 {
        self.sampler.synth.read(reg, self.bus_value)
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        self.bus_value = value;
        self.bus_value_ttl = 0x2000;
        self.sampler.synth.write(reg, value);
    }

    // -- State

    pub fn read_state(&self) -> State {
        let mut state = State {
            sid_register: [0; 32],
            bus_value: 0,
            bus_value_ttl: 0,
            ext_in: 0,
            accumulator: [0; 3],
            shift_register: [0; 3],
            envelope_state: [0; 3],
            envelope_counter: [0; 3],
            exponential_counter: [0; 3],
            exponential_counter_period: [0; 3],
            hold_zero: [0; 3],
            rate_counter: [0; 3],
            rate_counter_period: [0; 3],
        };
        for i in 0..3 {
            let j = i * 7;
            let wave = &self.sampler.synth.voices[i].wave;
            let envelope = &self.sampler.synth.voices[i].envelope;
            state.sid_register[j] = wave.get_frequency_lo();
            state.sid_register[j + 1] = wave.get_frequency_hi();
            state.sid_register[j + 2] = wave.get_pulse_width_lo();
            state.sid_register[j + 3] = wave.get_pulse_width_hi();
            state.sid_register[j + 4] = wave.get_control() | envelope.get_control();
            state.sid_register[j + 5] = envelope.get_attack_decay();
            state.sid_register[j + 6] = envelope.get_sustain_release();
        }
        let filter = &self.sampler.synth.filter;
        state.sid_register[0x15] = filter.get_fc_lo();
        state.sid_register[0x16] = filter.get_fc_hi();
        state.sid_register[0x17] = filter.get_res_filt();
        state.sid_register[0x18] = filter.get_mode_vol();
        for i in 0x19..0x1d {
            state.sid_register[i] = self.read(i as u8);
        }
        for i in 0x1d..0x20 {
            state.sid_register[i] = 0;
        }
        state.bus_value = self.bus_value;
        state.bus_value_ttl = self.bus_value_ttl;
        state.ext_in = self.sampler.synth.ext_in;
        for i in 0..3 {
            let wave = &self.sampler.synth.voices[i].wave;
            let envelope = &self.sampler.synth.voices[i].envelope;
            state.accumulator[i] = wave.get_acc();
            state.shift_register[i] = wave.get_shift();
            state.envelope_state[i] = envelope.state as u8;
            state.envelope_counter[i] = envelope.envelope_counter;
            state.exponential_counter[i] = envelope.exponential_counter;
            state.exponential_counter_period[i] = envelope.exponential_counter_period;
            state.hold_zero[i] = if envelope.hold_zero { 1 } else { 0 };
            state.rate_counter[i] = envelope.rate_counter;
            state.rate_counter_period[i] = envelope.rate_counter_period;
        }
        state
    }

    pub fn write_state(&mut self, state: &State) {
        for i in 0..0x19 {
            self.write(i, state.sid_register[i as usize]);
        }
        self.bus_value = state.bus_value;
        self.bus_value_ttl = state.bus_value_ttl;
        self.sampler.synth.ext_in = state.ext_in;
        for i in 0..3 {
            let envelope = &mut self.sampler.synth.voices[i].envelope;
            self.sampler.synth.voices[i].wave.acc = state.accumulator[i];
            self.sampler.synth.voices[i].wave.shift = state.shift_register[i];
            envelope.state = match state.envelope_state[i] {
                0 => EnvState::Attack,
                1 => EnvState::DecaySustain,
                2 => EnvState::Release,
                _ => panic!("invalid envelope state"),
            };
            envelope.envelope_counter = state.envelope_counter[i];
            envelope.exponential_counter = state.exponential_counter[i];
            envelope.exponential_counter_period = state.exponential_counter_period[i];
            envelope.hold_zero = state.hold_zero[i] != 0;
            envelope.rate_counter = state.rate_counter[i];
            envelope.rate_counter_period = state.rate_counter_period[i];
        }
    }
}
