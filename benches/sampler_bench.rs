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

#[macro_use]
extern crate criterion;
extern crate resid;

use criterion::Criterion;
use resid::sampler::Sampler;

fn bench_compute_convolution_fir(c: &mut Criterion) {
    c.bench_function("convolution_fir_avx2", |b| {
        let sampler = Sampler::new();
        let samples = [2i16; 1024];
        let fir = [5i16; 1024];
        b.iter(|| unsafe { sampler.compute_convolution_fir_avx2(&samples[..], &fir[..]) })
    });
    c.bench_function("convolution_fir_sse", |b| {
        let sampler = Sampler::new();
        let samples = [2i16; 1024];
        let fir = [5i16; 1024];
        b.iter(|| unsafe { sampler.compute_convolution_fir_sse(&samples[..], &fir[..]) })
    });
    c.bench_function("convolution_fir_fallback", |b| {
        let sampler = Sampler::new();
        let samples = [2i16; 1024];
        let fir = [5i16; 1024];
        b.iter(|| sampler.compute_convolution_fir_fallback(&samples[..], &fir[..]))
    });
}

criterion_group!(benches, bench_compute_convolution_fir);
criterion_main!(benches);
