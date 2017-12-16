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

mod data;

use resid::ChipModel;
use resid::wave::WaveformGenerator;

fn setup(wave: &mut WaveformGenerator, waveform: u8, freq: u16, pw: u16) {
    wave.set_control((waveform & 0x0f) << 4 | 0x00);
    wave.set_frequency_hi((freq >> 8) as u8);
    wave.set_frequency_lo((freq & 0xff) as u8);
    wave.set_pulse_width_hi((pw >> 8) as u8);
    wave.set_pulse_width_lo((pw & 0xff) as u8);
}

#[test]
fn waveform_1() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 1, 32000, 100);
    for i in 0..500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE1_OUTPUT[i]);
    }
}

#[test]
fn waveform_2() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 2, 16000, 100);
    for i in 0..500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE2_OUTPUT[i]);
    }
}

#[test]
fn waveform_3() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 3, 32000, 100);
    for i in 0..500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE3_OUTPUT[i]);
    }
}

#[test]
fn waveform_4() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 4, 16000, 1000);
    for i in 0..1500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE4_OUTPUT[i]);
    }
}

#[test]
fn waveform_5() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 5, 16000, 1000);
    for i in 0..1500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE5_OUTPUT[i]);
    }
}

#[test]
fn waveform_6() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 6, 16000, 1000);
    for i in 0..1500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE6_OUTPUT[i]);
    }
}

#[test]
fn waveform_7() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 7, 16000, 1000);
    for i in 0..1500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE7_OUTPUT[i]);
    }
}

#[test]
fn waveform_8() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 8, 16000, 1000);
    for i in 0..1500 {
        wave.clock();
        assert_eq!(wave.output(), data::wave_output::RESID_WAVE8_OUTPUT[i]);
    }
}

#[test]
fn waveform_delta_1() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 1, 32000, 100);
    for i in 0..500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE1_OUTPUT[i]
        );
    }
}

#[test]
fn waveform_delta_2() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 2, 16000, 100);
    for i in 0..500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE2_OUTPUT[i]
        );
    }
}

#[test]
fn waveform_delta_3() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 3, 32000, 100);
    for i in 0..500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE3_OUTPUT[i]
        );
    }
}

#[test]
fn waveform_delta_4() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 4, 16000, 1000);
    for i in 0..1500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE4_OUTPUT[i]
        );
    }
}

#[test]
fn waveform_delta_5() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 5, 16000, 1000);
    for i in 0..1500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE5_OUTPUT[i]
        );
    }
}

#[test]
fn waveform_delta_6() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 6, 16000, 1000);
    for i in 0..1500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE6_OUTPUT[i]
        );
    }
}

#[test]
fn waveform_delta_7() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 7, 16000, 1000);
    for i in 0..1500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE7_OUTPUT[i]
        );
    }
}

#[test]
fn waveform_delta_8() {
    let mut wave = WaveformGenerator::new(ChipModel::Mos6581);
    setup(&mut wave, 8, 16000, 1000);
    for i in 0..1500 {
        wave.clock_delta(25);
        assert_eq!(
            wave.output(),
            data::wave_delta_output::RESID_WAVE8_OUTPUT[i]
        );
    }
}
