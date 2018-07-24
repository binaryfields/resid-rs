// This file is part of resid-rs.
// Copyright (c) 2017-2018 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

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
