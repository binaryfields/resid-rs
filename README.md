# resid-rs

[![Build Status](https://travis-ci.org/digitalstreamio/resid-rs.svg?branch=master)](https://travis-ci.org/digitalstreamio/resid-rs)
[![Crates.io](https://img.shields.io/crates/v/resid-rs.svg?maxAge=2592000)](https://crates.io/crates/resid-rs)

### Overview

Port of reSID, a MOS6581 SID emulator engine, to Rust.

### Story

This project originated from zinc64, a Commodore 64 emulator, around Jan 2017.
It evolved to the point where it can be useful to others working on C64 sound/emulation
so it is packaged and shipped as a standalone crate.

### Status

The port has reached compliance with the original resid implementation as of version 0.3.
Output has been tested and verified to match the original implementation.
Full sampler support has been added to v0.4 which reached feature parity
with resid. Performance gap largely due to resampling has been closed in v0.5.
SIMD optimization added in v0.6. Continuous integration and GPLv3 were brought to v0.7.
v0.8 changes include documentation and api refinements/internal cleanup.

### Usage

Once SID register read/writes are wired up to resid, all that is left to do
is to generate audio samples and push them to audio output buffer.

    while delta > 0 {
        let (samples, next_delta) = self.resid.sample(delta, &mut buffer[..], 1);
        let mut output = self.sound_buffer.lock().unwrap();
        for i in 0..samples {
            output.write(buffer[i]);
        }
        delta = next_delta;
    }

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
- Thanks to Daniel Collin for motivating me to put this out and helping out with code optimization
- Commodore folks for building an iconic 8-bit machine
- Rust developers for providing an incredible language to develop in
