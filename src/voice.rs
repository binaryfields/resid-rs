// This file is part of resid-rs.
// Copyright (c) 2017-2019 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]

use super::envelope::EnvelopeGenerator;
use super::wave::{Syncable, WaveformGenerator};
use super::ChipModel;

/// The waveform output range is 0x000 to 0xfff, so the "zero"
/// level should ideally have been 0x800. In the measured chip, the
/// waveform output "zero" level was found to be 0x380 (i.e. $d41b
/// = 0x38) at 5.94V.
const WAVE_ZERO: i32 = 0x0380;

/// The envelope multiplying D/A converter introduces another DC
/// offset. This is isolated by the following measurements:
///
/// * The "zero" output level of the mixer at full volume is 5.44V.
/// * Routing one voice to the mixer at full volume yields
///     6.75V at maximum voice output (wave = 0xfff, sustain = 0xf)
///     5.94V at "zero" voice output  (wave = any,   sustain = 0x0)
///     5.70V at minimum voice output (wave = 0x000, sustain = 0xf)
/// * The DC offset of one voice is (5.94V - 5.44V) = 0.50V
/// * The dynamic range of one voice is |6.75V - 5.70V| = 1.05V
/// * The DC offset is thus 0.50V/1.05V ~ 1/2 of the dynamic range.
///
/// Note that by removing the DC offset, we get the following ranges for
/// one voice:
///     y > 0: (6.75V - 5.44V) - 0.50V =  0.81V
///     y < 0: (5.70V - 5.44V) - 0.50V = -0.24V
/// The scaling of the voice amplitude is not symmetric about y = 0;
/// this follows from the DC level in the waveform output.
const VOICE_DC: i32 = 0x800 * 0xff;

#[derive(Clone, Copy)]
pub struct Voice {
    // Configuration
    wave_zero: i32,
    voice_dc: i32,
    // Generators
    pub envelope: EnvelopeGenerator,
    pub wave: WaveformGenerator,
}

impl Voice {
    pub fn new(chip_model: ChipModel) -> Self {
        match chip_model {
            ChipModel::Mos6581 => Voice {
                wave_zero: WAVE_ZERO,
                voice_dc: VOICE_DC,
                envelope: EnvelopeGenerator::default(),
                wave: WaveformGenerator::new(chip_model),
            },
            ChipModel::Mos8580 => Voice {
                // No DC offsets in the MOS8580.
                wave_zero: 0x800,
                voice_dc: 0,
                envelope: EnvelopeGenerator::default(),
                wave: WaveformGenerator::new(chip_model),
            },
        }
    }

    pub fn set_control(&mut self, value: u8) {
        self.envelope.set_control(value);
        self.wave.set_control(value);
    }

    /// Amplitude modulated 20-bit waveform output.
    /// Range [-2048*255, 2047*255].
    #[inline]
    pub fn output(&self, sync_source: Option<&WaveformGenerator>) -> i32 {
        // Multiply oscillator output with envelope output.
        (self.wave.output(sync_source) as i32 - self.wave_zero) * self.envelope.output() as i32
            + self.voice_dc
    }

    pub fn reset(&mut self) {
        self.envelope.reset();
        self.wave.reset();
    }
}

impl Syncable<&'_ Voice> {
    pub fn output(&self) -> i32 {
        self.main.output(Some(&self.sync_source.wave))
    }
}

impl<'a> Syncable<&'a Voice> {
    pub fn wave(self) -> Syncable<&'a WaveformGenerator> {
        Syncable {
            main: &self.main.wave,
            sync_dest: &self.sync_dest.wave,
            sync_source: &self.sync_source.wave,
        }
    }
}

impl<'a> Syncable<&'a mut Voice> {
    pub fn wave(self) -> Syncable<&'a mut WaveformGenerator> {
        Syncable {
            main: &mut self.main.wave,
            sync_dest: &mut self.sync_dest.wave,
            sync_source: &mut self.sync_source.wave,
        }
    }
}
