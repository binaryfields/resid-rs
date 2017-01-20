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

extern crate resid;

use resid::{ChipModel, Sid};

const CPU_FREQ: u32 = 985248;
const SAMPLE_FREQ: u32 = 44100;
const SAMPLE_COUNT: usize = 128;
const CYCLES_PER_SAMPLE: u32 = CPU_FREQ / SAMPLE_FREQ;

fn dump(sid: &mut Sid, name: &str, samples: usize) -> Vec<i16> {
    let mut buffer = vec![0; samples];
    let (read, delta) = sid.sample(samples as u32 * CYCLES_PER_SAMPLE,
                                   &mut buffer,
                                   samples,
                                   1);
    buffer
}

fn setup(sid: &mut Sid, voice: u8, waveform: u8, freq: u16, pw: u16, vol: u8) {
    let offset = voice * 7;
    let control = (waveform << 4) | 1;
    sid.write(offset + 0x00, (freq & 0x00ff) as u8); // FREQ_LO
    sid.write(offset + 0x01, (freq >> 8) as u8); // FREQ_HI
    sid.write(offset + 0x02, (pw & 0x00ff) as u8); // PW_LO
    sid.write(offset + 0x03, (pw >> 8) as u8); // PW_HI
    sid.write(offset + 0x04, control); // CONTROL
    sid.write(offset + 0x05, 9); // ATTACK_DECAY
    sid.write(offset + 0x06, 0); // SUSTAIN_RELEASE
    sid.write(0x18, vol); // MODE_VOL
}

#[test]
fn extfilter_off() {
    let expected: Vec<i16> = vec![
        -21189, -21155, -21147, -21139, -20987, -20956, -20909, -20878, -20831, -20800, -20753, -20721, -20675, -20643, -20166, -20112, -20030, -19975, -19893, -19839, -19757, -19703, -19621, -19566, -19484, -19430, -19348, -19293, -19212, -19157, -19075, -19021, -18966, -18884, -18830, -18748, -18693, -18612, -18557, -18475, -18421, -18339, -18284, -18203, -18148, -18066, -18012, -17930, -17875, -17793, -19205, -17658, -17604, -17522, -17467, -17385, -17331, -17249, -17195, -17140, -17058, -17004, -16922, -16867, -16785, -16731, -16649, -16595, -16513, -16458, -18424, -18392, -18345, -18314, -16104, -16049, -15967, -15913, -15831, -15776, -15695, -15640, -15558, -15504, -15422, -15367, -15285, -15231, -15176, -15095, -15040, -14958, -14904, -14822, -14767, -14685, -14631, -14549, -14495, -14413, -14358, -17220, -14222, -14222, -14222, -14222, -14222, -14222, -14222, -14222, -14222, -14222, -14222, -14222, -17188, -17188, -17188, -17188, -17188, -17188, -17188, -17188, -17188, -17188, -17188, -17188, 0, 0];
    let mut sid = Sid::new(ChipModel::Mos6581);
    sid.enable_external_filter(false);
    sid.enable_filter(false);
    setup(&mut sid, 0, 4, 0x19b1, 0x0200, 4);
    setup(&mut sid, 1, 4, 0x29b1, 0x0100, 4);
    setup(&mut sid, 2, 4, 0x39b1, 0x0050, 4);
    let res = dump(&mut sid, "extfilter_off", SAMPLE_COUNT);
    assert_eq!(res, expected);
}

#[test]
fn extfilter_on() {
    let expected: Vec<i16> = vec![
        4096, 4289, 4291, 4290, 4426, 4452, 4488, 4511, 4547, 4568, 4605, 4626, 4662, 4684, 5132, 5192, 5262, 5306, 5375, 5419, 5487, 5531, 5599, 5642, 5710, 5753, 5821, 5864, 5931, 5974, 6042, 6083, 6125, 6192, 6234, 6301, 6343, 6409, 6450, 6517, 6558, 6624, 6665, 6731, 6772, 6838, 6878, 6943, 6984, 7048, 5682, 7098, 7195, 7260, 7300, 7364, 7404, 7468, 7507, 7545, 7609, 7647, 7711, 7750, 7813, 7852, 7914, 7953, 8016, 8053, 6152, 6092, 6123, 6141, 8250, 8371, 8434, 8471, 8533, 8569, 8631, 8667, 8729, 8765, 8826, 8862, 8924, 8959, 8994, 9055, 9090, 9150, 9186, 9246, 9281, 9342, 9376, 9436, 9471, 9530, 9565, 6801, 9548, 9644, 9625, 9603, 9582, 9560, 9539, 9518, 9496, 9475, 9455, 9433, 6567, 6435, 6419, 6405, 6391, 6376, 6362, 6348, 6334, 6320, 6307, 6292, 0, 0];
    let mut sid = Sid::new(ChipModel::Mos6581);
    sid.enable_external_filter(true);
    sid.enable_filter(false);
    setup(&mut sid, 0, 4, 0x19b1, 0x0200, 4);
    setup(&mut sid, 1, 4, 0x29b1, 0x0100, 4);
    setup(&mut sid, 2, 4, 0x39b1, 0x0050, 4);
    let res = dump(&mut sid, "extfilter_on", SAMPLE_COUNT);
    assert_eq!(res, expected);
}
