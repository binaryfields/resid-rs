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

mod wave6581_ps;
mod wave6581_pst;
mod wave6581_pt;
mod wave6581_st;
mod wave8580_ps;
mod wave8580_pst;
mod wave8580_pt;
mod wave8580_st;

pub use self::wave6581_ps::WAVE6581_PS;
pub use self::wave6581_pst::WAVE6581_PST;
pub use self::wave6581_pt::WAVE6581_PT;
pub use self::wave6581_st::WAVE6581_ST;
pub use self::wave8580_ps::WAVE8580_PS;
pub use self::wave8580_pst::WAVE8580_PST;
pub use self::wave8580_pt::WAVE8580_PT;
pub use self::wave8580_st::WAVE8580_ST;
