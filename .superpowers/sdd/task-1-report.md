# Task 1 Report: Encoder Pass-Through

## Status: DONE

## What was implemented
Replaced the `encode_software_h264` method in `crates/ob-codec/src/encoder.rs` with a pass-through that stores:
- 4 bytes: width (u32 LE)
- 4 bytes: height (u32 LE)
- 4 bytes: timestamp_us (u32 LE)
- 4 bytes: pixel data length (u32 LE)
- N bytes: raw BGRA pixel data

## Verification
- `cargo check -p ob-codec` passes with only dead-code warnings (expected)

## Commit
- `cb166b9` fix(codec): pass through raw BGRA in stub encoder

## Concerns
None
