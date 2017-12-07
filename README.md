# resid-rs

### Overview

Port of reSID, a MOS6581 SID emulator engine, to Rust

### Status

| Component      | Status  | clock()  | clock_delta() |
|----------------|---------|----------|---------------|
| Envelope       | Done    | Pass     | TBD           |
| ExternalFilter | Done    | Pass     | Pass          |
| Filter         | Done    | TBD      | TBD           |
| Spline         | Done    | n/a      | n/a           |
| Wave           | Done    | Pass     | Pass          |
| Sid            | Done    | Pass     | Pass          |
