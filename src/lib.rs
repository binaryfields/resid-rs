// This file is part of resid-rs.
// Copyright (c) 2017-2018 Sebastian Jastrzebski <sebby2k@gmail.com>. All rights reserved.
// Portions (c) 2004 Dag Lem <resid@nimrod.no>
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

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
pub use self::sampler::SamplingMethod;
