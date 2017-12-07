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

use resid::spline;

static FO_POINTS_6581: [(i32, i32); 31] =
    [
        //  FC      f         FCHI FCLO
        // ----------------------------
        (0, 220), // 0x00      - repeated end point
        (0, 220), // 0x00
        (128, 230), // 0x10
        (256, 250), // 0x20
        (384, 300), // 0x30
        (512, 420), // 0x40
        (640, 780), // 0x50
        (768, 1600), // 0x60
        (832, 2300), // 0x68
        (896, 3200), // 0x70
        (960, 4300), // 0x78
        (992, 5000), // 0x7c
        (1008, 5400), // 0x7e
        (1016, 5700), // 0x7f
        (1023, 6000), // 0x7f 0x07
        (1023, 6000), // 0x7f 0x07 - discontinuity
        (1024, 4600), // 0x80      -
        (1024, 4600), // 0x80
        (1032, 4800), // 0x81
        (1056, 5300), // 0x84
        (1088, 6000), // 0x88
        (1120, 6600), // 0x8c
        (1152, 7200), // 0x90
        (1280, 9500), // 0xa0
        (1408, 12000), // 0xb0
        (1536, 14500), // 0xc0
        (1664, 16000), // 0xd0
        (1792, 17100), // 0xe0
        (1920, 17700), // 0xf0
        (2047, 18000), // 0xff 0x07
        (2047, 18000)    // 0xff 0x07 - repeated end point
    ];

fn set_f0(f0: &mut [i32; 2048]) {
    let points = FO_POINTS_6581.into_iter()
        .map(|&pt| {
            spline::Point {
                x: pt.0 as f64,
                y: pt.1 as f64,
            }
        })
        .collect::<Vec<spline::Point>>();
    let mut plotter = spline::PointPlotter::new(2048);
    spline::interpolate(&points, &mut plotter, 1.0);
    let output = plotter.output();
    for i in 0..2048 {
        f0[i] = output[i];
    }
}

#[test]
fn interpolate() {
    let mut f0 = [0i32; 2048];
    set_f0(&mut f0);
    for i in 0..2048usize {
        assert_eq!(f0[i], data::spline_output::RESID_OUTPUT[i]);
    }
}
