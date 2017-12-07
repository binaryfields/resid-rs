/*
 * Copyright (c) 2017 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
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

extern crate resid;

use resid::envelope::EnvelopeGenerator;

static RESID_OUTPUT: &'static [u8] = include_bytes!("data/envelope_output.dat");

#[test]
fn clock() {
    let mut envelope = EnvelopeGenerator::new();
    let mut cycles = 0u32;
    // setup
    envelope.set_attack_decay(0x02 << 4 | 0x00);
    envelope.set_sustain_release(0x02 << 4 | 0x01);
    envelope.set_control(0x01);
    let attack_cycles = 63u16;
    let decay_cycles = 9u16;
    let sustain_level = 0x22u8;
    let release_cycles = 32u16;
    let mut last_output = 0u8;
    let mut exp_counter = 0u8;
    let mut exp_period = 1u8;
    // test attack
    for _j in 0..0xff {
        for _i in 0..attack_cycles {
            envelope.clock();
            cycles += 1;
        }
        let output = envelope.output();
        assert_eq!(output, last_output + 1);
        last_output = output;
    }
    // test decay
    last_output = envelope.output();
    while last_output != sustain_level {
        for _i in 0..decay_cycles {
            envelope.clock();
            cycles += 1;
        }
        exp_counter += 1;
        if exp_counter == exp_period {
            exp_counter = 0;
            let output = envelope.output();
            assert_eq!(output, last_output - 1);
            last_output = output;
            exp_period = match output {
                0xff => 1,
                0x5d => 2,
                0x36 => 4,
                0x1a => 8,
                0x0e => 16,
                0x06 => 30,
                0x00 => 1,
                _ => exp_period,
            }
        }
    }
    // test sustain
    for _i in 0..2 {
        for _i in 0..decay_cycles {
            envelope.clock();
            cycles += 1;
        }
        exp_counter += 1;
        if exp_counter == exp_period {
            exp_counter = 0;
            let output = envelope.output();
            assert_eq!(output, last_output);
        }
    }
    // test release
    assert_eq!(cycles, 18963);
    envelope.set_control(0x00);
    while last_output != 0x00 {
        for _i in 0..release_cycles {
            envelope.clock();
            cycles += 1;
        }
        exp_counter += 1;
        if exp_counter == exp_period {
            exp_counter = 0;
            let output = envelope.output();
            assert_eq!(output, last_output - 1);
            last_output = output;
            exp_period = match output {
                0xff => 1,
                0x5d => 2,
                0x36 => 4,
                0x1a => 8,
                0x0e => 16,
                0x06 => 30,
                0x00 => 1,
                _ => exp_period,
            }
        }
    }
    // test hold zero
    for _i in 0..2 {
        for _i in 0..release_cycles {
            envelope.clock();
            cycles += 1;
        }
        exp_counter += 1;
        if exp_counter == exp_period {
            exp_counter = 0;
            let output = envelope.output();
            assert_eq!(output, 0x00);
        }
    }
    // verify cycle count
    assert_eq!(cycles, 32915);
}

fn clock_delta() {
    let mut envelope = EnvelopeGenerator::new();
    envelope.set_attack_decay(0x02 << 4 | 0x00);
    envelope.set_sustain_release(0x02 << 4 | 0x01);
    envelope.set_control(0x01);
    let mut envelope2 = EnvelopeGenerator::new();
    envelope2.set_attack_decay(0x02 << 4 | 0x00);
    envelope2.set_sustain_release(0x02 << 4 | 0x01);
    envelope2.set_control(0x01);
    for i in 0..33000 {
        if i == 19000 {
            envelope.set_control(0x00);
            envelope2.set_control(0x00);
        }
        envelope.clock();
        if i % 100 == 0 {
            envelope2.clock_delta(100);
            assert_eq!(envelope2.output(), envelope.output());
        }
    }
}

#[test]
fn resid_output() {
    let mut envelope = EnvelopeGenerator::new();
    envelope.reset();
    // setup
    envelope.set_attack_decay(0x02 << 4 | 0x00);
    envelope.set_sustain_release(0x02 << 4 | 0x01);
    envelope.set_control(0x01);
    // generate
    let mut buffer: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < 18963 {
        envelope.clock();
        buffer.push(envelope.output());
        i += 1;
    }
    envelope.set_control(0x00);
    while i < 32914 {
        envelope.clock();
        buffer.push(envelope.output());
        i += 1;
    }
    // validate
    assert_eq!(buffer.len(), RESID_OUTPUT.len());
    assert_eq!(&*buffer, RESID_OUTPUT);
}
