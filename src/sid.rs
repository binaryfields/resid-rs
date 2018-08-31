// This file is part of resid-rs.
// Copyright (c) 2017-2018 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use super::ChipModel;
use super::envelope::State as EnvState;
use super::external_filter::ExternalFilter;
use super::filter::Filter;
use super::sampler::{Sampler, SamplingMethod};
use super::voice::Voice;

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
    ext_filter: ExternalFilter,
    filter: Filter,
    sampler: Option<Sampler>,
    voices: [Voice; 3],
    // Runtime State
    bus_value: u8,
    bus_value_ttl: u32,
    ext_in: i32,
}

impl Sid {
    pub fn new(chip_model: ChipModel) -> Self {
        let mut voice1 = Voice::new(chip_model);
        let mut voice2 = Voice::new(chip_model);
        let mut voice3 = Voice::new(chip_model);
        voice1.set_sync_source(&mut voice3);
        voice2.set_sync_source(&mut voice1);
        voice3.set_sync_source(&mut voice2);
        let mut sid = Sid {
            ext_filter: ExternalFilter::new(chip_model),
            filter: Filter::new(chip_model),
            sampler: Some(Sampler::new()),
            voices: [voice1, voice2, voice3],
            bus_value: 0,
            bus_value_ttl: 0,
            ext_in: 0,
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
        if let Some(ref mut sampler) = self.sampler {
            sampler.set_parameters(method, clock_freq, sample_freq);
        }
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

    pub fn reset(&mut self) {
        self.ext_filter.reset();
        self.filter.reset();
        if let Some(ref mut sampler) = self.sampler {
            sampler.reset();
        }
        for i in 0..3 {
            self.voices[i].reset();
        }
        self.bus_value = 0;
        self.bus_value_ttl = 0;
        self.ext_in = 0;
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
        if let Some(mut sampler) = self.sampler.take() {
            let result = sampler.sample(self, delta, buffer, n, interleave);
            self.sampler = Some(sampler);
            result
        } else {
            panic!("invalid sampler")
        }
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
            let wave = self.voices[i].wave.borrow();
            let envelope = &self.voices[i].envelope;
            state.sid_register[j + 0] = wave.get_frequency_lo();
            state.sid_register[j + 1] = wave.get_frequency_hi();
            state.sid_register[j + 2] = wave.get_pulse_width_lo();
            state.sid_register[j + 3] = wave.get_pulse_width_hi();
            state.sid_register[j + 4] = wave.get_control() | envelope.get_control();
            state.sid_register[j + 5] = envelope.get_attack_decay();
            state.sid_register[j + 6] = envelope.get_sustain_release();
        }
        state.sid_register[0x15] = self.filter.get_fc_lo();
        state.sid_register[0x16] = self.filter.get_fc_hi();
        state.sid_register[0x17] = self.filter.get_res_filt();
        state.sid_register[0x18] = self.filter.get_mode_vol();
        for i in 0x19..0x1d {
            state.sid_register[i] = self.read(i as u8);
        }
        for i in 0x1d..0x20 {
            state.sid_register[i] = 0;
        }
        state.bus_value = self.bus_value;
        state.bus_value_ttl = self.bus_value_ttl;
        state.ext_in = self.ext_in;
        for i in 0..3 {
            let wave = self.voices[i].wave.borrow();
            let envelope = &self.voices[i].envelope;
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
        self.ext_in = state.ext_in;
        for i in 0..3 {
            let envelope = &mut self.voices[i].envelope;
            self.voices[i].wave.borrow_mut().acc = state.accumulator[i];
            self.voices[i].wave.borrow_mut().shift = state.shift_register[i];
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
