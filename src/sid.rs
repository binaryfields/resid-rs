// This file is part of resid-rs.
// Copyright (c) 2017-2018 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use super::ChipModel;
use super::envelope::State as EnvState;
use super::sampler::{Sampler, SamplingMethod};
use super::synth::Synth;

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
        sid.set_sampling_parameters(SamplingMethod::Fast, 985248, 44100);
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
    ///     while (delta_t) {
    ///       bufindex += sid.clock(delta_t, buf + bufindex, buflength - bufindex);
    ///       write(dsp, buf, bufindex*2);
    ///       bufindex = 0;
    ///     }
    /// ```
    pub fn sample(
        &mut self,
        delta: u32,
        buffer: &mut [i16],
        n: usize,
        interleave: usize,
    ) -> (usize, u32) {
       self.sampler.clock(delta, buffer, n, interleave)
    }

    // -- Device I/O

    pub fn read(&self, reg: u8) -> u8 {
        match Reg::from(reg) {
            Reg::POTX => 0,
            Reg::POTY => 0,
            Reg::OSC3 => self.sampler.synth.voices[2].wave.borrow().read_osc(),
            Reg::ENV3 => self.sampler.synth.voices[2].envelope.read_env(),
            _ => self.bus_value,
        }
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        self.bus_value = value;
        self.bus_value_ttl = 0x2000;
        match Reg::from(reg) {
            Reg::FREQLO1 => {
                self.sampler.synth.voices[0].wave.borrow_mut().set_frequency_lo(value);
            }
            Reg::FREQHI1 => {
                self.sampler.synth.voices[0].wave.borrow_mut().set_frequency_hi(value);
            }
            Reg::PWLO1 => {
                self.sampler.synth.voices[0].wave.borrow_mut().set_pulse_width_lo(value);
            }
            Reg::PWHI1 => {
                self.sampler.synth.voices[0].wave.borrow_mut().set_pulse_width_hi(value);
            }
            Reg::CR1 => {
                self.sampler.synth.voices[0].set_control(value);
            }
            Reg::AD1 => {
                self.sampler.synth.voices[0].envelope.set_attack_decay(value);
            }
            Reg::SR1 => {
                self.sampler.synth.voices[0].envelope.set_sustain_release(value);
            }
            Reg::FREQLO2 => {
                self.sampler.synth.voices[1].wave.borrow_mut().set_frequency_lo(value);
            }
            Reg::FREQHI2 => {
                self.sampler.synth.voices[1].wave.borrow_mut().set_frequency_hi(value);
            }
            Reg::PWLO2 => {
                self.sampler.synth.voices[1].wave.borrow_mut().set_pulse_width_lo(value);
            }
            Reg::PWHI2 => {
                self.sampler.synth.voices[1].wave.borrow_mut().set_pulse_width_hi(value);
            }
            Reg::CR2 => {
                self.sampler.synth.voices[1].set_control(value);
            }
            Reg::AD2 => {
                self.sampler.synth.voices[1].envelope.set_attack_decay(value);
            }
            Reg::SR2 => {
                self.sampler.synth.voices[1].envelope.set_sustain_release(value);
            }
            Reg::FREQLO3 => {
                self.sampler.synth.voices[2].wave.borrow_mut().set_frequency_lo(value);
            }
            Reg::FREQHI3 => {
                self.sampler.synth.voices[2].wave.borrow_mut().set_frequency_hi(value);
            }
            Reg::PWLO3 => {
                self.sampler.synth.voices[2].wave.borrow_mut().set_pulse_width_lo(value);
            }
            Reg::PWHI3 => {
                self.sampler.synth.voices[2].wave.borrow_mut().set_pulse_width_hi(value);
            }
            Reg::CR3 => {
                self.sampler.synth.voices[2].set_control(value);
            }
            Reg::AD3 => {
                self.sampler.synth.voices[2].envelope.set_attack_decay(value);
            }
            Reg::SR3 => {
                self.sampler.synth.voices[2].envelope.set_sustain_release(value);
            }
            Reg::FCLO => {
                self.sampler.synth.filter.set_fc_lo(value);
            }
            Reg::FCHI => {
                self.sampler.synth.filter.set_fc_hi(value);
            }
            Reg::RESFILT => {
                self.sampler.synth.filter.set_res_filt(value);
            }
            Reg::MODVOL => {
                self.sampler.synth.filter.set_mode_vol(value);
            }
            _ => {}
        }
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
            let wave = self.sampler.synth.voices[i].wave.borrow();
            let envelope = &self.sampler.synth.voices[i].envelope;
            state.sid_register[j + 0] = wave.get_frequency_lo();
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
            let wave = self.sampler.synth.voices[i].wave.borrow();
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

    pub fn write_state(&mut self, state: State) {
        for i in 0..0x19 {
            self.write(i, state.sid_register[i as usize]);
        }
        self.bus_value = state.bus_value;
        self.bus_value_ttl = state.bus_value_ttl;
        self.sampler.synth.ext_in = state.ext_in;
        for i in 0..3 {
            let envelope = &mut self.sampler.synth.voices[i].envelope;
            self.sampler.synth.voices[i].wave.borrow_mut().acc = state.accumulator[i];
            self.sampler.synth.voices[i].wave.borrow_mut().shift = state.shift_register[i];
            envelope.state = match state.envelope_state[i] {
                0 => EnvState::Attack,
                1 => EnvState::DecaySustain,
                2 => EnvState::Release,
                _ => panic!("invalid envelope state"),
            };
            envelope.envelope_counter = state.envelope_counter[i];
            envelope.exponential_counter = state.exponential_counter[i];
            envelope.exponential_counter_period = state.exponential_counter_period[i];
            envelope.hold_zero = if state.hold_zero[i] != 0 { true } else { false };
            envelope.rate_counter = state.rate_counter[i];
            envelope.rate_counter_period = state.rate_counter_period[i];
        }
    }
}
