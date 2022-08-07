#[macro_use]
extern crate criterion;

mod sid_bench;
mod sampler_bench;

criterion_group!(benches, sid_bench::bench_sid, sampler_bench::bench_compute_convolution_fir);

criterion_main!(benches);
