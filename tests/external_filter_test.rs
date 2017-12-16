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

use resid::ChipModel;
use resid::external_filter::ExternalFilter;

#[cfg_attr(rustfmt, rustfmt_skip)]
static RESID_OUTPUT: [i32; 41] = [
    0, -100, -184, -255, -314, -362, -400, -429,
    -450, -464, -471, -472, -468, -460, -447, -431,
    -411, -388, -362, -334, -303, -270, -236, -200,
    -162, -123, -83, -42, 0, 42, 85, 129,
    173, 218, 263, 309, 355, 402, 449, 496,
    544
];

#[cfg_attr(rustfmt, rustfmt_skip)]
static RESID_DELTA_OUTPUT: [i32; 41] = [
    -989, -927, -864, -801, -738, -675, -612, -549,
    -486, -423, -360, -297, -234, -171, -108, -45,
    8, 58, 108, 158, 208, 258, 308, 358,
    408, 458, 508, 558, 608, 658, 708, 758,
    808, 858, 908, 958, 1008, 1058, 1108, 1158,
    1208
];

#[test]
fn clock() {
    let mut ext_filter = ExternalFilter::new(ChipModel::Mos6581);
    let mut index = 0usize;
    let mut vi = -1000;
    while vi <= 1000 {
        ext_filter.clock(vi);
        let output = ext_filter.output();
        assert_eq!(output, RESID_OUTPUT[index]);
        vi += 50;
        index += 1;
    }
}

#[test]
fn clock_delta() {
    let mut ext_filter = ExternalFilter::new(ChipModel::Mos6581);
    let mut index = 0usize;
    let mut vi = -1000;
    while vi <= 1000 {
        ext_filter.clock_delta(100, vi);
        let output = ext_filter.output();
        assert_eq!(output, RESID_DELTA_OUTPUT[index]);
        vi += 50;
        index += 1;
    }
}
