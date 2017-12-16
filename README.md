# resid-rs

### Overview

Port of reSID, a MOS6581 SID emulator engine, to Rust.

### Story

This project originated from zinc64, a Commodore 64 emulator, around Jan 2017.
It evolved to the point where it can be useful to others working on C64 sound/emulation
so it is packaged and shipped as a standalone crate.

### Status

The port has reached compliance with original resid implementation as of version 0.3.
The output has been tested and verified to match original implementation.

### Roadmap

v0.4 - resampling support

### Components

| Component         | Status      |
|-------------------|-------------|
| Envelope          | Done        |
| ExternalFilter    | Done        |
| Filter            | Done        |
| Sampler           | Done        |
| Spline            | Done        |
| Wave              | Done        |
| Sid               | Done        |

## Credits

- Thanks to Dag Lem for his reSID implementation
- Commodore folks for building an iconic 8-bit machine
- Rust developers for providing an incredible language to develop in
