# OmniBridge: Wire Full Pipeline Design

**Date:** 2026-07-11
**Goal:** Connect all stages so a screen capture on the primary device appears on the secondary device's overlay window in real-time.

## Problem

The codebase has networking, screen capture, encoding, and display components, but none are connected. The server never sends video frames. The client never receives or displays them. The overlay window is never created.

## Approach

Extend the existing `tokio::select!` loops in `server.rs` and `client.rs` with timer-driven capture and frame-receive branches. Use the existing stub codec (no real H.264 yet). Create a real Win32 overlay window for display.

## Changes

### 1. Protocol Extension (`ob-core/src/protocol.rs`)

Add `VideoFrame` to the `MessageType` enum:

```rust
pub enum MessageType {
    Handshake,
    HandshakeAck,
    InputEvent,
    VideoFrame,    // NEW
    WindowTransfer,
}
```

The `VideoFrame` message payload is a serialized `VideoFrameData` struct:

```rust
pub struct VideoFrameData {
    pub source_device: DeviceId,
    pub target_device: DeviceId,
    pub width: u32,
    pub height: u32,
    pub timestamp_us: u64,
    pub is_keyframe: bool,
    pub pixels: Vec<u8>,  // raw BGRA pixels (stub codec output)
}
```

### 2. Server Pipeline (`src/server.rs`)

Add a capture-and-stream loop as a new `tokio::spawn` task:

```
every 33ms (30fps):
  1. screen_capturer.capture_frame()
  2. video_encoder.encode_frame(&frame)
  3. build VideoFrameData from encoded.data
  4. serialize to Message with MessageType::VideoFrame
  5. send_to_all connected clients via UDP
```

The server needs:
- A `ScreenCapturer` initialized with the first detected screen
- A `VideoEncoder` initialized with screen dimensions
- A `connected_clients` clone (Arc<RwLock<>>) shared with the forwarding task

### 3. Client Pipeline (`src/client.rs`)

Add a frame-receive branch to the `tokio::select!` loop:

```
on receive VideoFrame message:
  1. deserialize VideoFrameData
  2. video_decoder.decode_frame() -> DecodedFrame
  3. overlay_window.update_from_decoded_frame(&decoded)
  4. overlay_window.render_pixels(&decoded.pixels)
```

The client needs:
- A `VideoDecoder` instance
- An `OverlayWindow` instance (newly created)

### 4. Overlay Window (`ob-display/src/overlay.rs`)

Replace the broken `FindWindowA` approach with actual Win32 window creation:

**Window creation** (called once at startup):
- `RegisterClassExA` with a custom window procedure
- `CreateWindowExA` with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT`
- Window class name: `"OmniBridgeOverlay"`
- Initial size: 800x600, positioned at (0,0)

**Pixel rendering** (called every frame):
- Create a DIB section from the decoded pixel buffer
- Use `StretchDIBits` to blit pixels to the window DC
- Call `SetWindowPos` to reposition if needed
- Call `SetLayeredWindowAttributes` for alpha blending

**Window cleanup:**
- `DestroyWindow` on shutdown

The overlay window struct gains a `hwnd` field stored after creation, and new methods:
- `create_window()` -> sets up the Win32 window
- `render_pixels(&self, pixels: &[u8], width: u32, height: u32)` -> blits pixels
- `destroy_window()` -> cleanup

### 5. Encoder/Decoder Adjustments (`ob-codec/`)

The stub encoder currently outputs a truncated subsampled buffer. For the pipeline demo, modify it to pass through raw BGRA pixels (uncompressed) so the overlay receives full-quality displayable data. This is temporary — real H.264 replaces this later.

**Encoder change:** Output the full raw BGRA pixel buffer with a small header (width, height, timestamp). No actual compression.

**Decoder change:** Read the header, reconstruct the full BGRA buffer. No actual decompression needed since data is uncompressed.

This means `EncodedFrame.data` will be large (~width*height*4 bytes) but correct. The pipeline will work end-to-end. Real H.264 compression is the next phase.

## Files Modified

| File | Change |
|------|--------|
| `crates/ob-core/src/protocol.rs` | Add `VideoFrame` variant + `VideoFrameData` struct |
| `crates/ob-codec/src/encoder.rs` | Pass-through raw pixels in stub mode |
| `crates/ob-codec/src/decoder.rs` | Reconstruct raw pixels in stub mode |
| `crates/ob-display/src/overlay.rs` | Create real Win32 window + pixel rendering |
| `src/server.rs` | Add capture timer + frame streaming task |
| `src/client.rs` | Add frame receive + decode + display |

## What This Unlocks

After this phase:
- Start primary on Machine A, secondary on Machine B
- Primary captures screen at 30fps
- Frames stream over UDP to the secondary
- Secondary displays them in a topmost overlay window
- Mouse/keyboard input flows back from secondary to primary
- **A functional KVM demo** (with uncompressed video, upgradeable later)

## What's NOT in Scope

- Real H.264 codec (next phase)
- DXGI Desktop Duplication (GDI BitBlt is fine for demo)
- Window drag transfer (just screen mirroring for now)
- Linux/macOS support
- GUI — CLI only
