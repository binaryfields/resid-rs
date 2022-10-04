use criterion::Criterion;
use resid::sampler::Sampler;
use resid::synth::Synth;
use resid::ChipModel;

pub fn bench_compute_convolution_fir(c: &mut Criterion) {
    c.bench_function("convolution_fir", |b| {
        let sampler = Sampler::new(Synth::new(ChipModel::Mos6581));
        let samples = [2i16; 1024];
        let fir = [5i16; 1024];
        b.iter(|| sampler.compute_convolution_fir(&samples[..], &fir[..]))
    });
    #[target_feature(enable = "avx2")]
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    c.bench_function("convolution_fir_avx2", |b| {
        let sampler = Sampler::new(Synth::new(ChipModel::Mos6581));
        let samples = [2i16; 1024];
        let fir = [5i16; 1024];
        b.iter(|| unsafe { sampler.compute_convolution_fir_avx2(&samples[..], &fir[..]) })
    });
    #[target_feature(enable = "sse4.2")]
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    c.bench_function("convolution_fir_sse", |b| {
        let sampler = Sampler::new(Synth::new(ChipModel::Mos6581));
        let samples = [2i16; 1024];
        let fir = [5i16; 1024];
        b.iter(|| unsafe { sampler.compute_convolution_fir_sse(&samples[..], &fir[..]) })
    });
    c.bench_function("convolution_fir_fallback", |b| {
        let sampler = Sampler::new(Synth::new(ChipModel::Mos6581));
        let samples = [2i16; 1024];
        let fir = [5i16; 1024];
        b.iter(|| sampler.compute_convolution_fir_fallback(&samples[..], &fir[..]))
    });
}
