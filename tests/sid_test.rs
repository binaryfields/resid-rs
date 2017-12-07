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

use resid::{ChipModel, Sid};

static SID_DATA: [u16; 51] = [
    25, 177, 250, 28, 214, 250,
    25, 177, 250, 25, 177, 250,
    25, 177, 125, 28, 214, 125,
    32, 94, 750, 25, 177, 250,
    28, 214, 250, 19, 63, 250,
    19, 63, 250, 19, 63, 250,
    21, 154, 63, 24, 63, 63,
    25, 177, 250, 24, 63, 125,
    19, 63, 250,
];

#[test]
fn clock_delta() {
    let mut sid = Sid::new(ChipModel::Mos6581);
    sid.write(0x05, 0x09); // AD1
    sid.write(0x06, 0x00); // SR1
    sid.write(0x18, 0x0f); // MODVOL
    let mut i = 0;
    let mut index = 0usize;
    while i < SID_DATA.len() {
        sid.write(0x01, SID_DATA[i + 0] as u8); // FREQHI1
        sid.write(0x00, SID_DATA[i + 1] as u8); // FREQLO1
        sid.write(0x00, 0x21); // CR1
        for _j in 0..SID_DATA[i + 2] {
            sid.clock_delta(22);
            assert_eq!(sid.output(), data::sid_output::RESID_OUTPUT[index]);
            index += 1;
        }
        sid.write(0x00, 0x20); // CR1
        for _j in 0..50 {
            sid.clock_delta(22);
            assert_eq!(sid.output(), data::sid_output::RESID_OUTPUT[index]);
            index += 1;
        }
        i += 3;
    }
}