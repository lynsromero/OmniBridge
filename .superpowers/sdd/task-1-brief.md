### Task 1: Fix Encoder to Pass Through Raw BGRA

**Files:**
- Modify: `crates/ob-codec/src/encoder.rs:68-86`

**Interfaces:**
- Consumes: `CapturedFrame` (from `ob-capture::frame`)
- Produces: `EncodedFrame` with `data` containing full BGRA pixels (header + raw data)

The current `encode_software_h264` subsamples pixels down to 64KB grayscale. Replace it with a pass-through that stores the full pixel buffer with a header, so the decoder can reconstruct the original image.

- [ ] **Step 1: Replace `encode_software_h264` method**

In `crates/ob-codec/src/encoder.rs`, replace lines 68-86 with:

```rust
    fn encode_software_h264(&self, frame: &CapturedFrame) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(20 + frame.pixels.len());

        output.extend_from_slice(&frame.metadata.width.to_le_bytes());
        output.extend_from_slice(&frame.metadata.height.to_le_bytes());
        output.extend_from_slice(&frame.metadata.timestamp_us.to_le_bytes());
        output.extend_from_slice(&(frame.pixels.len() as u32).to_le_bytes());

        output.extend_from_slice(&frame.pixels);

        Ok(output)
    }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-codec`
Expected: OK (warnings acceptable)

- [ ] **Step 3: Commit**

```bash
git add crates/ob-codec/src/encoder.rs
git commit -m "fix(codec): pass through raw BGRA in stub encoder"
```

---

