### Task 2: Fix Decoder to Reconstruct Raw BGRA

**Files:**
- Modify: `crates/ob-codec/src/decoder.rs:42-70`

**Interfaces:**
- Consumes: `EncodedFrame` with header + raw BGRA pixels
- Produces: `DecodedFrame` with full BGRA pixel buffer

The current `decode_h264` reconstructs grayscale from subsampled data. Replace it to read the header and return the original pixel buffer.

- [ ] **Step 1: Replace `decode_h264` method**

In `crates/ob-codec/src/decoder.rs`, replace lines 42-70 with:

```rust
    fn decode_h264(&self, encoded: &EncodedFrame) -> Result<Vec<u8>> {
        if encoded.data.len() < 20 {
            return Err(anyhow::anyhow!("Encoded frame too small"));
        }

        let _width = u32::from_le_bytes(encoded.data[0..4].try_into()?);
        let _height = u32::from_le_bytes(encoded.data[4..8].try_into()?);
        let _timestamp = u64::from_le_bytes(encoded.data[8..16].try_into()?);
        let pixel_data_len = u32::from_le_bytes(encoded.data[16..20].try_into()?) as usize;

        if encoded.data.len() < 20 + pixel_data_len {
            return Err(anyhow::anyhow!("Pixel data truncated"));
        }

        Ok(encoded.data[20..20 + pixel_data_len].to_vec())
    }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-codec`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-codec/src/decoder.rs
git commit -m "fix(codec): reconstruct raw BGRA in stub decoder"
```

---

