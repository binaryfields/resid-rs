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

use std::cell::RefCell;
use std::rc::Rc;

use super::ChipModel;
use super::data;

const ACC_MASK: u32 = 0x00ffffff;
const ACC_BIT19_MASK: u32 = 0x080000;
const ACC_MSB_MASK: u32 = 0x800000;
const SHIFT_MASK: u32 = 0x7fffff;
const OUTPUT_MASK: u16 = 0x0fff;

#[inline(always)]
fn bit_test(value: u8, bit: u8) -> bool {
    value & (1 << bit) != 0
}

// ----------------------------------------------------------------------------
// A 24 bit accumulator is the basis for waveform generation. FREQ is added to
// the lower 16 bits of the accumulator each cycle.
// The accumulator is set to zero when TEST is set, and starts counting
// when TEST is cleared.
// The noise waveform is taken from intermediate bits of a 23 bit shift
// register. This register is clocked by bit 19 of the accumulator.
// ----------------------------------------------------------------------------

pub struct WaveformGenerator {
    // Dependencies
    sync_source: Option<Rc<RefCell<WaveformGenerator>>>,
    sync_dest: Option<Rc<RefCell<WaveformGenerator>>>,
    // Configuration
    frequency: u16,
    pulse_width: u16,
    // Control
    waveform: u8,
    ring: bool,
    sync: bool,
    test: bool,
    // Runtime State
    acc: u32,
    shift: u32,
    msb_rising: bool,
    // Static Data
    wave_ps: &'static [u8; 4096],
    wave_pst: &'static [u8; 4096],
    wave_pt: &'static [u8; 4096],
    wave_st: &'static [u8; 4096],
}

impl WaveformGenerator {
    pub fn new(chip_model: ChipModel) -> WaveformGenerator {
        let (wave_ps, wave_pst, wave_pt, wave_st) = match chip_model {
            ChipModel::Mos6581 => {
                (&data::WAVE6581_PS, &data::WAVE6581_PST, &data::WAVE6581_PT, &data::WAVE6581_ST)
            }
            ChipModel::Mos8580 => {
                (&data::WAVE8580_PS, &data::WAVE8580_PST, &data::WAVE8580_PT, &data::WAVE8580_ST)
            }
        };
        let mut waveform = WaveformGenerator {
            sync_source: None,
            sync_dest: None,
            frequency: 0,
            pulse_width: 0,
            waveform: 0,
            ring: false,
            sync: false,
            test: false,
            acc: 0,
            shift: 0,
            msb_rising: false,
            wave_ps: wave_ps,
            wave_pst: wave_pst,
            wave_pt: wave_pt,
            wave_st: wave_st,
        };
        waveform.reset();
        waveform
    }

    pub fn get_acc(&self) -> u32 {
        self.acc
    }

    pub fn get_frequency(&self) -> u16 {
        self.frequency
    }

    pub fn get_sync_dest_sync(&self) -> bool {
        if let Some(ref sync_dest) = self.sync_dest {
            sync_dest.borrow().sync
        } else {
            false
        }
    }

    pub fn get_sync_source_acc(&self) -> u32 {
        if let Some(ref sync_source) = self.sync_source {
            sync_source.borrow().acc
        } else {
            0
        }
    }

    pub fn is_msb_rising(&self) -> bool {
        self.msb_rising
    }

    pub fn set_acc(&mut self, value: u32) {
        self.acc = value;
    }

    pub fn set_control(&mut self, value: u8) {
        self.waveform = (value >> 4) & 0x0f;
        self.sync = bit_test(value, 1);
        self.ring = bit_test(value, 2);
        let test = bit_test(value, 3);
        if test {
            // Test bit set.
            // The accumulator and the shift register are both cleared.
            // NB! The shift register is not really cleared immediately. It seems like
            // the individual bits in the shift register start to fade down towards
            // zero when test is set. All bits reach zero within approximately
            // $2000 - $4000 cycles.
            // This is not modeled. There should fortunately be little audible output
            // from this peculiar behavior.
            self.acc = 0;
            self.shift = 0;
        } else if self.test {
            // Test bit cleared.
            // The accumulator starts counting, and the shift register is reset to
            // the value 0x7ffff8.
            // NB! The shift register will not actually be set to this exact value if the
            // shift register bits have not had time to fade to zero.
            // This is not modeled.
            self.shift = 0x7ffff8;
        }
        self.test = test;
    }

    pub fn set_frequency_hi(&mut self, value: u8) {
        let value = (self.frequency & 0x00ff) | ((value as u16) << 8);
        self.frequency = value;
    }

    pub fn set_frequency_lo(&mut self, value: u8) {
        let value = (self.frequency & 0xff00) | (value as u16);
        self.frequency = value;
    }

    pub fn set_pulse_width_hi(&mut self, value: u8) {
        let value = (self.pulse_width & 0x00ff) | ((value as u16) << 8);
        self.pulse_width = value;
    }

    pub fn set_pulse_width_lo(&mut self, value: u8) {
        let value = (self.pulse_width & 0xff00) | (value as u16);
        self.pulse_width = value;
    }

    pub fn set_sync_dest(&mut self, dest: Rc<RefCell<WaveformGenerator>>) {
        self.sync_dest = Some(dest);
    }

    pub fn set_sync_source(&mut self, source: Rc<RefCell<WaveformGenerator>>) {
        self.sync_source = Some(source);
    }

    pub fn clock(&mut self) {
        // No operation if test bit is set.
        if !self.test {
            let acc_prev = self.acc;
            // Calculate new accumulator value;
            self.acc = (self.acc + self.frequency as u32) & ACC_MASK;
            // Check whether the MSB is set high. This is used for synchronization.
            self.msb_rising = (acc_prev & ACC_MSB_MASK) == 0 && (self.acc & ACC_MSB_MASK) != 0;
            if (acc_prev & ACC_BIT19_MASK) == 0 && (self.acc & ACC_BIT19_MASK) != 0 {
                // Shift noise register once for each time accumulator bit 19 is set high.
                let bit0 = ((self.shift >> 22) ^ (self.shift >> 17)) & 0x01;
                self.shift = ((self.shift << 1) & SHIFT_MASK) | bit0;
            }
        }
    }

    pub fn clock_delta(&mut self, delta: u32) {
        if !self.test {
            let acc_prev = self.acc;
            // Calculate new accumulator value;
            let mut delta_acc = delta * self.frequency as u32;
            self.acc = (self.acc + delta_acc) & ACC_MASK;
            // Check whether the MSB is set high. This is used for synchronization.
            self.msb_rising = (acc_prev & ACC_MSB_MASK) == 0 && (self.acc & ACC_MSB_MASK) != 0;
            // Shift noise register once for each time accumulator bit 19 is set high.
            // Bit 19 is set high each time 2^20 (0x100000) is added to the accumulator.
            let mut shift_period = 0x100000;
            while delta_acc != 0 {
                if delta_acc < shift_period {
                    shift_period = delta_acc;
                    // Determine whether bit 19 is set on the last period.
                    // NB! Requires two's complement integer.
                    if shift_period <= 0x080000 {
                        // Check for flip from 0 to 1.
                        if ((self.acc as i32 - shift_period as i32) & ACC_BIT19_MASK as i32) != 0 || (self.acc & ACC_BIT19_MASK) == 0 {
                            break;
                        }
                    } else {
                        // Check for flip from 0 (to 1 or via 1 to 0) or from 1 via 0 to 1.
                        if ((self.acc as i32 - shift_period as i32) & ACC_BIT19_MASK as i32) != 0 && (self.acc & ACC_BIT19_MASK) == 0 {
                            break;
                        }
                    }
                }
                // Shift the noise/random register.
                let bit0 = ((self.shift >> 22) ^ (self.shift >> 17)) & 0x01;
                self.shift = (self.shift << 1) & SHIFT_MASK | bit0;
                delta_acc -= shift_period;
            }
        }
    }

    /* 12-bit waveform output. */
    pub fn output(&self) -> u16 {
        match self.waveform {
            0x0 => 0,
            0x1 => self.output_t(),
            0x2 => self.output_s(),
            0x3 => self.output_st(),
            0x4 => self.output_p(),
            0x5 => self.output_pt(),
            0x6 => self.output_ps(),
            0x7 => self.output_pst(),
            0x8 => self.output_n(),
            0x9 => 0,
            0xa => 0,
            0xb => 0,
            0xc => 0,
            0xd => 0,
            0xe => 0,
            0xf => 0,
            _ => panic!("invalid waveform {}", self.waveform),
        }
    }

    pub fn read_osc(&self) -> u8 {
        (self.output() >> 4) as u8
    }

    pub fn reset(&mut self) {
        self.frequency = 0;
        self.pulse_width = 0;
        self.waveform = 0; // NOTE this is not in orig resid
        self.ring = false;
        self.sync = false;
        self.test = false;
        self.acc = 0;
        self.shift = 0x7ffff8;
        self.msb_rising = false;
    }

    /*
    Synchronize oscillators.
    This must be done after all the oscillators have been clock()'ed since the
    oscillators operate in parallel.
    Note that the oscillators must be clocked exactly on the cycle when the
    MSB is set high for hard sync to operate correctly. See SID::clock().
    */
    pub fn synchronize(&mut self) {
        // A special case occurs when a sync source is synced itself on the same
        // cycle as when its MSB is set high. In this case the destination will
        // not be synced. This has been verified by sampling OSC3.
        if self.is_msb_rising() {
            let dest_sync = if let Some(ref dest) = self.sync_dest {
                dest.borrow().sync
            } else {
                false
            };
            if dest_sync {
                let source_rising = if let Some(ref source) = self.sync_source {
                    source.borrow().is_msb_rising()
                } else {
                    false
                };
                if !(self.sync && source_rising) {
                    if let Some(ref dest) = self.sync_dest {
                        dest.borrow_mut().set_acc(0);
                    }
                }
            }
        }
    }

    // -- Output Functions

    // Noise:
    // The noise output is taken from intermediate bits of a 23-bit shift register
    // which is clocked by bit 19 of the accumulator.
    // NB! The output is actually delayed 2 cycles after bit 19 is set high.
    // This is not modeled.
    //
    // Operation: Calculate EOR result, shift register, set bit 0 = result.
    //
    //                        ----------------------->---------------------
    //                        |                                            |
    //                   ----EOR----                                       |
    //                   |         |                                       |
    //                   2 2 2 1 1 1 1 1 1 1 1 1 1                         |
    // Register bits:    2 1 0 9 8 7 6 5 4 3 2 1 0 9 8 7 6 5 4 3 2 1 0 <---
    //                   |   |       |     |   |       |     |   |
    // OSC3 bits  :      7   6       5     4   3       2     1   0
    //
    // Since waveform output is 12 bits the output is left-shifted 4 times.
    //
    fn output_n(&self) -> u16 {
        (((self.shift & 0x400000) >> 11) |
            ((self.shift & 0x100000) >> 10) |
            ((self.shift & 0x010000) >> 7) |
            ((self.shift & 0x002000) >> 5) |
            ((self.shift & 0x000800) >> 4) |
            ((self.shift & 0x000080) >> 1) |
            ((self.shift & 0x000010) << 1) |
            ((self.shift & 0x000004) << 2)) as u16
    }

    // Pulse:
    // The upper 12 bits of the accumulator are used.
    // These bits are compared to the pulse width register by a 12 bit digital
    // comparator; output is either all one or all zero bits.
    // NB! The output is actually delayed one cycle after the compare.
    // This is not modeled.
    //
    // The test bit, when set to one, holds the pulse waveform output at 0xfff
    // regardless of the pulse width setting.
    //
    fn output_p(&self) -> u16 {
        if self.test || ((self.acc >> 12) as u16 >= self.pulse_width) { 0x0fff } else { 0x0000 }
    }

    // Sawtooth:
    // The output is identical to the upper 12 bits of the accumulator.
    //
    fn output_s(&self) -> u16 {
        (self.acc >> 12) as u16
    }

    // Triangle:
    // The upper 12 bits of the accumulator are used.
    // The MSB is used to create the falling edge of the triangle by inverting
    // the lower 11 bits. The MSB is thrown away and the lower 11 bits are
    // left-shifted (half the resolution, full amplitude).
    // Ring modulation substitutes the MSB with MSB EOR sync_source MSB.
    //
    fn output_t(&self) -> u16 {
        let acc = if self.ring { self.acc ^ self.get_sync_source_acc() } else { self.acc };
        let msb = acc & ACC_MSB_MASK;
        let output = if msb != 0 { !self.acc } else { self.acc }; // TODO check w/ ref impl
        (output >> 11) as u16 & OUTPUT_MASK
    }

    // -- Combined Waveforms

    fn output_ps(&self) -> u16 {
        ((self.wave_ps[self.output_s() as usize] as u16) << 4) & self.output_p()
    }

    fn output_pst(&self) -> u16 {
        ((self.wave_pst[self.output_s() as usize] as u16) << 4) & self.output_p()
    }

    fn output_pt(&self) -> u16 {
        ((self.wave_pt[(self.output_t() >> 1) as usize] as u16) << 4) & self.output_p()
    }

    fn output_st(&self) -> u16 {
        (self.wave_st[self.output_s() as usize] as u16) << 4
    }
}
