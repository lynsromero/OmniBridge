### Task 2: Fix Decoder to Reconstruct Raw BGRA

- **Status:** DONE
- **Commit:** `167e147` fix(codec): reconstruct raw BGRA in stub decoder
- **Test:** `cargo check -p ob-codec` passed (1 pre-existing warning, no errors)
- **Concerns:** None. The new decoder reads the 20-byte header (width, height, timestamp, pixel_len), validates bounds, and returns the raw BGRA slice. No synthetic grayscale reconstruction.
