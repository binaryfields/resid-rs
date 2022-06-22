# resid-rs

[![Build Status](https://travis-ci.org/binaryfields/resid-rs.svg?branch=master)](https://travis-ci.org/binaryfields/resid-rs)
[![Crates.io](https://img.shields.io/crates/v/resid-rs.svg?maxAge=2592000)](https://crates.io/crates/resid-rs)

### Overview

Port of reSID, a MOS6581 SID emulator engine, to Rust.

### Story

This project originated from zinc64, a Commodore 64 emulator, around Jan 2017.
It evolved to the point where it can be useful to others working on C64 sound/emulation
so it is packaged and shipped as a standalone crate.

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

### Changelog

- 0.3 - compliance with the original resid
- 0.4 - full sampler support 
- 0.5 - closed performance gap largely due to resampling 
- 0.6 - SIMD optimization 
- 0.7 - continuous integration and GPLv3 
- 0.8 - documentation and api refinements/internal cleanup
- 0.9 - migration to Rust 2018
- 1.0 - no_std support
- 1.1 - more idiomatic implementation, removes interior mutability and improves support for async rust

## Credits

- Thanks to Dag Lem for his reSID implementation
- Thanks to Daniel Collin for motivating me to put this out and helping out with code optimization
- Commodore folks for building an iconic 8-bit machine
- Rust developers for providing an incredible language to develop in
