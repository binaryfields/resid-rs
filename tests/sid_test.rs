extern crate resid;

mod data;

use resid::{ChipModel, Sid};

#[cfg_attr(rustfmt, rustfmt_skip)]
static SID_DATA: [u16; 51] = [
    25, 177, 250, 28, 214, 250,
    25, 177, 250, 25, 177, 250,
    25, 177, 125, 28, 214, 125,
    32, 94, 750, 25, 177, 250,
    28, 214, 250, 19, 63, 250,
    19, 63, 250, 19, 63, 250,
    21, 154, 63, 24, 63, 63,
    25, 177, 250, 24, 63, 125,
    19, 63, 250,
];

#[test]
fn clock_delta() {
    let mut sid = Sid::new(ChipModel::Mos6581);
    sid.write(0x05, 0x09); // AD1
    sid.write(0x06, 0x00); // SR1
    sid.write(0x18, 0x0f); // MODVOL
    let mut i = 0;
    let mut index = 0usize;
    while i < SID_DATA.len() {
        sid.write(0x01, SID_DATA[i + 0] as u8); // FREQHI1
        sid.write(0x00, SID_DATA[i + 1] as u8); // FREQLO1
        sid.write(0x00, 0x21); // CR1
        for _j in 0..SID_DATA[i + 2] {
            sid.clock_delta(22);
            assert_eq!(sid.output(), data::sid_output::RESID_OUTPUT[index]);
            index += 1;
        }
        sid.write(0x00, 0x20); // CR1
        for _j in 0..50 {
            sid.clock_delta(22);
            assert_eq!(sid.output(), data::sid_output::RESID_OUTPUT[index]);
            index += 1;
        }
        i += 3;
    }
}
