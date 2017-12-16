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

extern crate bit_field;

mod data;
pub mod envelope;
pub mod external_filter;
pub mod filter;
pub mod sampler;
mod sid;
pub mod spline;
pub mod voice;
pub mod wave;

#[derive(Clone, Copy)]
pub enum ChipModel {
    Mos6581,
    Mos8580,
}

pub use self::sid::Sid;
