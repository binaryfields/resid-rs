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

const RATE_COUNTER_MASK: u16 = 0x7fff;
const RATE_COUNTER_MSB_MASK: u16 = 0x8000;

// Rate counter periods are calculated from the Envelope Rates table in
// the Programmer's Reference Guide. The rate counter period is the number of
// cycles between each increment of the envelope counter.
// The rates have been verified by sampling ENV3.
//
// The rate counter is a 16 bit register which is incremented each cycle.
// When the counter reaches a specific comparison value, the envelope counter
// is incremented (attack) or decremented (decay/release) and the
// counter is zeroed.
//
// NB! Sampling ENV3 shows that the calculated values are not exact.
// It may seem like most calculated values have been rounded (.5 is rounded
// down) and 1 has beed added to the result. A possible explanation for this
// is that the SID designers have used the calculated values directly
// as rate counter comparison values, not considering a one cycle delay to
// zero the counter. This would yield an actual period of comparison value + 1.
//
// The time of the first envelope count can not be exactly controlled, except
// possibly by resetting the chip. Because of this we cannot do cycle exact
// sampling and must devise another method to calculate the rate counter
// periods.
//
// The exact rate counter periods can be determined e.g. by counting the number
// of cycles from envelope level 1 to envelope level 129, and dividing the
// number of cycles by 128. CIA1 timer A and B in linked mode can perform
// the cycle count. This is the method used to find the rates below.
//
// To avoid the ADSR delay bug, sampling of ENV3 should be done using
// sustain = release = 0. This ensures that the attack state will not lower
// the current rate counter period.
//
// The ENV3 sampling code below yields a maximum timing error of 14 cycles.
//     lda #$01
// l1: cmp $d41c
//     bne l1
//     ...
//     lda #$ff
// l2: cmp $d41c
//     bne l2
//
// This yields a maximum error for the calculated rate period of 14/128 cycles.
// The described method is thus sufficient for exact calculation of the rate
// periods.
//
static RATE_COUNTER_PERIOD: [u16; 16] = [
    9, // 2ms*1.0MHz/256 = 7.81
    32, // 8ms*1.0MHz/256 = 31.25
    63, // 16ms*1.0MHz/256 = 62.50
    95, // 24ms*1.0MHz/256 = 93.75
    149, // 38ms*1.0MHz/256 = 148.44
    220, // 56ms*1.0MHz/256 = 218.75
    267, // 68ms*1.0MHz/256 = 265.63
    313, // 80ms*1.0MHz/256 = 312.50
    392, // 100ms*1.0MHz/256 = 390.63
    977, // 250ms*1.0MHz/256 = 976.56
    1954, // 500ms*1.0MHz/256 = 1953.13
    3126, // 800ms*1.0MHz/256 = 3125.00
    3907, // 1 s*1.0MHz/256 =  3906.25
    11720, // 3 s*1.0MHz/256 = 11718.75
    19532, // 5 s*1.0MHz/256 = 19531.25
    31251, // 8 s*1.0MHz/256 = 31250.00
];

// From the sustain levels it follows that both the low and high 4 bits of the
// envelope counter are compared to the 4-bit sustain value.
// This has been verified by sampling ENV3.
//
static SUSTAIN_LEVEL: [u8; 16] = [
    0x00,
    0x11,
    0x22,
    0x33,
    0x44,
    0x55,
    0x66,
    0x77,
    0x88,
    0x99,
    0xaa,
    0xbb,
    0xcc,
    0xdd,
    0xee,
    0xff,
];

#[inline(always)]
pub fn bit_test(value: u8, bit: u8) -> bool {
    value & (1 << bit) != 0
}

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Attack,
    DecaySustain,
    Release,
}

// ----------------------------------------------------------------------------
// A 15 bit counter is used to implement the envelope rates, in effect
// dividing the clock to the envelope counter by the currently selected rate
// period.
// In addition, another counter is used to implement the exponential envelope
// decay, in effect further dividing the clock to the envelope counter.
// The period of this counter is set to 1, 2, 4, 8, 16, 30 at the envelope
// counter values 255, 93, 54, 26, 14, 6, respectively.
// ----------------------------------------------------------------------------

pub struct EnvelopeGenerator {
    // Configuration
    attack: u8,
    decay: u8,
    sustain: u8,
    release: u8,
    // Control
    gate: bool,
    // Runtime State
    state: State,
    envelope_counter: u8,
    exponential_counter: u8,
    exponential_counter_period: u8,
    hold_zero: bool,
    rate_counter: u16,
    rate_period: u16,
}

impl EnvelopeGenerator {
    pub fn new() -> EnvelopeGenerator {
        let mut envelope = EnvelopeGenerator {
            attack: 0,
            decay: 0,
            sustain: 0,
            release: 0,
            gate: false,
            state: State::Release,
            envelope_counter: 0,
            exponential_counter: 0,
            exponential_counter_period: 0,
            hold_zero: false,
            rate_counter: 0,
            rate_period: 0,
        };
        envelope.reset();
        envelope
    }

    pub fn set_attack_decay(&mut self, value: u8) {
        self.attack = (value >> 4) & 0x0f;
        self.decay = value & 0x0f;
        match self.state {
            State::Attack => self.rate_period = RATE_COUNTER_PERIOD[self.attack as usize],
            State::DecaySustain => self.rate_period = RATE_COUNTER_PERIOD[self.decay as usize],
            _ => {},
        }
    }

    pub fn set_control(&mut self, value: u8) {
        let gate = bit_test(value, 0);
        if !self.gate && gate {
            // Gate bit on: Start attack, decay, sustain.
            self.state = State::Attack;
            self.rate_period = RATE_COUNTER_PERIOD[self.attack as usize];
            // Switching to attack state unlocks the zero freeze.
            self.hold_zero = false;
        } else if self.gate && !gate {
            // Gate bit off: Start release.
            self.state = State::Release;
            self.rate_period = RATE_COUNTER_PERIOD[self.release as usize];
        }
        self.gate = gate;
    }

    pub fn set_sustain_release(&mut self, value: u8) {
        self.sustain = (value >> 4) & 0x0f;
        self.release = value & 0x0f;
        match self.state {
            State::Release => self.rate_period = RATE_COUNTER_PERIOD[self.release as usize],
            _ => {},
        }
    }

    pub fn clock(&mut self) {
        // Check for ADSR delay bug.
        // If the rate counter comparison value is set below the current value of the
        // rate counter, the counter will continue counting up until it wraps around
        // to zero at 2^15 = 0x8000, and then count rate_period - 1 before the
        // envelope can finally be stepped.
        // This has been verified by sampling ENV3.
        self.rate_counter += 1;
        if self.rate_counter & RATE_COUNTER_MSB_MASK != 0 {
            self.rate_counter += 1;
            self.rate_counter &= RATE_COUNTER_MASK;
        }
        if self.rate_counter == self.rate_period {
            self.rate_counter = 0;
            // The first envelope step in the attack state also resets the exponential
            // counter. This has been verified by sampling ENV3.
            self.exponential_counter += 1;
            if self.state == State::Attack ||
                self.exponential_counter == self.exponential_counter_period {
                self.exponential_counter = 0;
                // Check whether the envelope counter is frozen at zero.
                if self.hold_zero {
                    return;
                }
                match self.state {
                    State::Attack => {
                        // The envelope counter can flip from 0xff to 0x00 by changing state to
                        // release, then to attack. The envelope counter is then frozen at
                        // zero; to unlock this situation the state must be changed to release,
                        // then to attack. This has been verified by sampling ENV3.
                        self.envelope_counter += 1;
                        if self.envelope_counter == 0xff {
                            self.state = State::DecaySustain;
                            self.rate_period = RATE_COUNTER_PERIOD[self.decay as usize];
                        }
                    },
                    State::DecaySustain => {
                        if self.envelope_counter != SUSTAIN_LEVEL[self.sustain as usize] {
                            self.envelope_counter -= 1;
                        }
                    },
                    State::Release => {
                        // The envelope counter can flip from 0x00 to 0xff by changing state to
                        // attack, then to release. The envelope counter will then continue
                        // counting down in the release state.
                        // This has been verified by sampling ENV3.
                        // NB! The operation below requires two's complement integer.
                        self.envelope_counter -= 1;
                    },
                }
                // Check for change of exponential counter period.
                match self.envelope_counter {
                    0xff => self.exponential_counter_period = 1,
                    0x5d => self.exponential_counter_period = 2,
                    0x36 => self.exponential_counter_period = 4,
                    0x1a => self.exponential_counter_period = 8,
                    0x0e => self.exponential_counter_period = 16,
                    0x06 => self.exponential_counter_period = 30,
                    0x00 => {
                        self.exponential_counter_period = 1;
                        // When the envelope counter is changed to zero, it is frozen at zero.
                        // This has been verified by sampling ENV3.
                        self.hold_zero = true;
                    },
                    _ => {},
                }
            }
        }
    }

    pub fn clock_delta(&mut self, mut delta: u32) {
        let mut rate_step = self.rate_period - self.rate_counter;
        if rate_step <= 0 {
            rate_step += 0x7fff;
        }
        while delta != 0 {
            if delta < rate_step as u32 {
                self.rate_counter += delta as u16;
                if self.rate_counter & RATE_COUNTER_MSB_MASK != 0 {
                    self.rate_counter += 1;
                    self.rate_counter &= RATE_COUNTER_MASK;
                }
                return;
            }
            self.rate_counter = 0;
            delta -= rate_step as u32;
            // The first envelope step in the attack state also resets the exponential
            // counter. This has been verified by sampling ENV3.
            self.exponential_counter += 1;
            if self.state == State::Attack ||
                self.exponential_counter == self.exponential_counter_period {
                self.exponential_counter = 0;
                // Check whether the envelope counter is frozen at zero.
                if self.hold_zero {
                    rate_step = self.rate_period;
                    continue;
                }
                match self.state {
                    State::Attack => {
                        // The envelope counter can flip from 0xff to 0x00 by changing state to
                        // release, then to attack. The envelope counter is then frozen at
                        // zero; to unlock this situation the state must be changed to release,
                        // then to attack. This has been verified by sampling ENV3.
                        self.envelope_counter += 1;
                        if self.envelope_counter == 0xff {
                            self.state = State::DecaySustain;
                            self.rate_period = RATE_COUNTER_PERIOD[self.decay as usize];
                        }
                    },
                    State::DecaySustain => {
                        if self.envelope_counter != SUSTAIN_LEVEL[self.sustain as usize] {
                            self.envelope_counter -= 1;
                        }
                    },
                    State::Release => {
                        // The envelope counter can flip from 0x00 to 0xff by changing state to
                        // attack, then to release. The envelope counter will then continue
                        // counting down in the release state.
                        // This has been verified by sampling ENV3.
                        // NB! The operation below requires two's complement integer.
                        self.envelope_counter -= 1;
                    },
                }
                // Check for change of exponential counter period.
                match self.envelope_counter {
                    0xff => self.exponential_counter_period = 1,
                    0x5d => self.exponential_counter_period = 2,
                    0x36 => self.exponential_counter_period = 4,
                    0x1a => self.exponential_counter_period = 8,
                    0x0e => self.exponential_counter_period = 16,
                    0x06 => self.exponential_counter_period = 30,
                    0x00 => {
                        self.exponential_counter_period = 1;
                        // When the envelope counter is changed to zero, it is frozen at zero.
                        // This has been verified by sampling ENV3.
                        self.hold_zero = true;
                    },
                    _ => {},
                }
            }
            rate_step = self.rate_period;
        }
    }

    pub fn output(&self) -> u8 {
        self.envelope_counter
    }

    pub fn read_env(&self) -> u8 {
        self.output()
    }

    pub fn reset(&mut self) {
        self.attack = 0;
        self.decay = 0;
        self.sustain = 0;
        self.release = 0;
        self.gate = false;
        self.state = State::Release;
        self.envelope_counter = 0;
        self.exponential_counter = 0;
        self.exponential_counter_period = 1;
        self.hold_zero = true;
        self.rate_counter = 0;
        self.rate_period = RATE_COUNTER_PERIOD[self.release as usize];
    }
}

