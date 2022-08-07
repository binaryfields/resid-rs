use criterion::Criterion;
use resid::{ChipModel, Sid};

pub fn bench_sid(c: &mut Criterion) {
    c.bench_function("clock_delta", |b| {
        let mut sid = Sid::new(ChipModel::Mos6581);
        sid.write(0x05, 0x09); // AD1
        sid.write(0x06, 0x00); // SR1
        sid.write(0x18, 0x0f); // MODVOL
        sid.write(0x01, 25); // FREQHI1
        sid.write(0x00, 177); // FREQLO1
        sid.write(0x00, 0x21); // CR1
        b.iter(|| sid.clock_delta(22))
    });
}
