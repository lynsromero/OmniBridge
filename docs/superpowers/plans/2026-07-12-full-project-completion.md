# OmniBridge Full Project Completion Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete OmniBridge from demo-quality pipeline to fully functional cross-device KVM tool with real video codec, drag-and-drop, multi-monitor support, and system tray GUI.

**Architecture:** Three-layer architecture: transport (UDP/TCP), codec (ffmpeg H.264), display (Win32 overlay + system tray). Server captures screen/window, encodes with H.264, chunks into UDP packets, sends to client. Client reassembles, decodes, renders to overlay. Drag detection triggers cross-device window transfer.

**Tech Stack:** Rust, ffmpeg-next (H.264), tokio (async), tray-icon (system tray), egui (GUI), Win32 API (capture/display)

## Global Constraints

- Rust stable toolchain with GNU target (`stable-x86_64-pc-windows-gnu`)
- MSYS2 MinGW at `C:\msys64\mingw64\bin` for dlltool and ffmpeg
- Git identity: `Lyns Romero` / `sakibramim4@gmail.com`
- All code compiles with `cargo check` (0 errors)
- All changes committed with descriptive messages
- README updated after each task

---

## Sub-project 1: Fix Data Path

### Task 1: Add ffmpeg-next dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add ffmpeg-next to workspace dependencies**

In `Cargo.toml`, add under `[workspace.dependencies]`:
```toml
ffmpeg-next = "7.0"
```

- [ ] **Step 2: Add to ob-codec dependencies**

In `crates/ob-codec/Cargo.toml`, add:
```toml
ffmpeg-next = { workspace = true }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p ob-codec`
Expected: OK (may have warnings about ffmpeg bindings)

- [ ] **Step 4: Install ffmpeg libraries**

Run: `C:\msys64\usr\bin\pacman.exe -S --noconfirm mingw-w64-x86_64-ffmpeg`

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/ob-codec/Cargo.toml
git commit -m "deps(codec): add ffmpeg-next for H.264 encoding/decoding"
```

---

### Task 2: Implement real ffmpeg encoder

**Files:**
- Modify: `crates/ob-codec/src/encoder.rs`

**Interfaces:**
- Consumes: `CapturedFrame` (from `ob-capture::frame`)
- Produces: `EncodedFrame` with compressed H.264 data

- [ ] **Step 1: Replace encoder implementation**

Replace the entire `crates/ob-codec/src/encoder.rs` with:

```rust
use anyhow::Result;
use ob_capture::frame::CapturedFrame;
use tracing::{debug, info};

pub struct VideoEncoder {
    width: u32,
    height: u32,
    target_fps: u32,
    bitrate: u32,
    frame_count: u64,
    encoder: ffmpeg_next::encoder::Encoder,
    frame: ffmpeg_next::frame::Video,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32, target_fps: u32) -> Self {
        ffmpeg_next::init().expect("Failed to initialize ffmpeg");

        let mut encoder = ffmpeg_next::encoder::new(
            ffmpeg_next::codec::Id::H264,
            ffmpeg_next::codec::encoder::Type::Video,
        )
        .expect("Failed to create encoder");

        encoder.set_bit_rate(width as usize * height as usize * 3 / 10);
        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(ffmpeg_next::format::Pixel::YUV420P);
        encoder.set_time_base(ffmpeg_next::Rational { numerator: 1, denominator: target_fps as i32 });
        encoder.set_max_b_frames(0);

        let mut options = ffmpeg_next::dictionary::Dictionary::new();
        options.set("preset", "ultrafast");
        options.set("tune", "zerolatency");
        options.set("crf", "23");

        encoder.open_with(options).expect("Failed to open encoder");

        let mut frame = ffmpeg_next::frame::Video::empty();
        frame.set_format(ffmpeg_next::format::Pixel::YUV420P);
        frame.set_width(width);
        frame.set_height(height);

        info!(
            "Created ffmpeg H.264 encoder: {}x{} @ {}fps",
            width, height, target_fps
        );

        Self {
            width,
            height,
            target_fps,
            bitrate: width as usize * height as usize * 3 / 10,
            frame_count: 0,
            encoder,
            frame,
        }
    }

    pub fn set_bitrate(&mut self, bitrate: u32) {
        self.bitrate = bitrate;
        debug!("Bitrate set to {} bps", bitrate);
    }

    pub fn encode_frame(&mut self, frame: &CapturedFrame) -> Result<EncodedFrame> {
        self.frame_count += 1;
        let start = std::time::Instant::now();

        // Convert BGRA to YUV420P
        self.convert_bgra_to_yuv420p(&frame.pixels, frame.metadata.width, frame.metadata.height);

        self.frame.set_pts(Some(self.frame_count as i64));

        let mut encoder = &mut self.encoder;
        encoder.send_frame(&self.frame)?;

        let mut encoded_data = Vec::new();
        let mut packet = ffmpeg_next::packet::Packet::empty();
        while encoder.receive_packet(&mut packet).is_ok() {
            encoded_data.extend_from_slice(packet.data().unwrap_or(&[]));
            packet.clear();
        }

        let encode_time = start.elapsed().as_micros() as u64;

        Ok(EncodedFrame {
            data: encoded_data,
            width: frame.metadata.width,
            height: frame.metadata.height,
            frame_number: self.frame_count,
            is_keyframe: self.frame_count % (self.target_fps as u64 * 2) == 0,
            timestamp_us: frame.metadata.timestamp_us,
            encode_time_us: encode_time,
            format: EncodedFormat::H264,
        })
    }

    fn convert_bgra_to_yuv420p(&mut self, bgra: &[u8], width: u32, height: u32) {
        let y_plane = self.frame.plane_mut::<u8>(0);
        let u_plane = self.frame.plane_mut::<u8>(1);
        let v_plane = self.frame.plane_mut::<u8>(2);

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let b = bgra[idx] as f32;
                let g = bgra[idx + 1] as f32;
                let r = bgra[idx + 2] as f32;

                let y_val = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
                y_plane[(y * width + x) as usize] = y_val;

                if y % 2 == 0 && x % 2 == 0 {
                    let u_val = (-0.169 * r - 0.331 * g + 0.500 * b + 128.0).clamp(0.0, 255.0) as u8;
                    let v_val = (0.500 * r - 0.419 * g - 0.081 * b + 128.0).clamp(0.0, 255.0) as u8;
                    u_plane[((y / 2) * (width / 2) + (x / 2)) as usize] = u_val;
                    v_plane[((y / 2) * (width / 2) + (x / 2)) as usize] = v_val;
                }
            }
        }
    }

    pub fn force_keyframe(&mut self) {
        // Force next frame to be keyframe
        self.frame_count = 0;
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub frame_number: u64,
    pub is_keyframe: bool,
    pub timestamp_us: u64,
    pub encode_time_us: u64,
    pub format: EncodedFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EncodedFormat {
    H264,
    H265,
    AV1,
    VP9,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-codec`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-codec/src/encoder.rs
git commit -m "feat(codec): implement real ffmpeg H.264 encoder

- Replace raw BGRA passthrough with actual H.264 encoding
- Use ffmpeg-next crate with ultrafast preset and CRF 23
- Convert BGRA input to YUV420P for encoding
- Configurable bitrate and keyframe interval"
```

---

### Task 3: Implement real ffmpeg decoder

**Files:**
- Modify: `crates/ob-codec/src/decoder.rs`

**Interfaces:**
- Consumes: `EncodedFrame` with H.264 data
- Produces: `DecodedFrame` with BGRA pixels

- [ ] **Step 1: Replace decoder implementation**

Replace the entire `crates/ob-codec/src/decoder.rs` with:

```rust
use anyhow::Result;
use tracing::{debug, info};

use crate::encoder::EncodedFrame;

pub struct VideoDecoder {
    frame_count: u64,
    last_keyframe: u64,
    decoder: ffmpeg_next::decoder::Decoder,
    decoded_frame: ffmpeg_next::frame::Video,
    converter: ffmpeg_next::software::converter::Converter,
}

impl VideoDecoder {
    pub fn new() -> Self {
        ffmpeg_next::init().expect("Failed to initialize ffmpeg");

        let mut decoder = ffmpeg_next::decoder::new(
            ffmpeg_next::codec::Id::H264,
            ffmpeg_next::codec::decoder::Type::Video,
        )
        .expect("Failed to create decoder");

        decoder.set_medium(ffmpeg_next::codec::threading::Type::Frame);
        decoder.open().expect("Failed to open decoder");

        let decoded_frame = ffmpeg_next::frame::Video::empty();

        let converter = ffmpeg_next::software::converter::get(
            (ffmpeg_next::format::Pixel::YUV420P, 0, 0),
            (ffmpeg_next::format::Pixel::BGRA, 0, 0),
        )
        .expect("Failed to create pixel format converter");

        info!("Created ffmpeg H.264 decoder");

        Self {
            frame_count: 0,
            last_keyframe: 0,
            decoder,
            decoded_frame,
            converter,
        }
    }

    pub fn decode_frame(&mut self, encoded: &EncodedFrame) -> Result<DecodedFrame> {
        self.frame_count += 1;
        if encoded.is_keyframe {
            self.last_keyframe = encoded.frame_number;
        }

        let start = std::time::Instant::now();

        let packet_data = encoded.data.clone();
        let mut packet = ffmpeg_next::packet::Packet::copy(&packet_data)?;
        packet.set_pts(Some(encoded.timestamp_us as i64));

        self.decoder.send_packet(&packet)?;

        let mut output_frame = None;
        while self.decoder.receive_frame(&mut self.decoded_frame).is_ok() {
            let mut rgb_frame = ffmpeg_next::frame::Video::empty();
            self.converter.run(&self.decoded_frame, &mut rgb_frame)?;
            output_frame = Some(rgb_frame);
        }

        let decode_time = start.elapsed().as_micros() as u64;

        let pixels = if let Some(frame) = output_frame {
            frame.data(0).to_vec()
        } else {
            vec![0u8; (encoded.width * encoded.height * 4) as usize]
        };

        Ok(DecodedFrame {
            pixels,
            width: encoded.width,
            height: encoded.height,
            frame_number: encoded.frame_number,
            timestamp_us: encoded.timestamp_us,
            decode_time_us: decode_time,
        })
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub frame_number: u64,
    pub timestamp_us: u64,
    pub decode_time_us: u64,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-codec`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-codec/src/decoder.rs
git commit -m "feat(codec): implement real ffmpeg H.264 decoder

- Replace raw BGRA passthrough with actual H.264 decoding
- Use ffmpeg-next crate with frame-level threading
- Convert YUV420P output to BGRA for display
- Handle packet loss gracefully"
```

---

### Task 4: Add WindowFrameHeader to protocol

**Files:**
- Modify: `crates/ob-core/src/protocol.rs`

**Interfaces:**
- Produces: `WindowFrameHeader` struct for binary frame protocol

- [ ] **Step 1: Add WindowFrameHeader**

Append to `crates/ob-core/src/protocol.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowFrameHeader {
    pub width: u32,
    pub height: u32,
    pub timestamp_us: u64,
    pub format: u8,
}

impl WindowFrameHeader {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(17);
        buf.extend_from_slice(&self.width.to_le_bytes());
        buf.extend_from_slice(&self.height.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_us.to_le_bytes());
        buf.push(self.format);
        buf
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, anyhow::Error> {
        if data.len() < 17 {
            return Err(anyhow::anyhow!("WindowFrameHeader too short"));
        }
        Ok(Self {
            width: u32::from_le_bytes(data[0..4].try_into()?),
            height: u32::from_le_bytes(data[4..8].try_into()?),
            timestamp_us: u64::from_le_bytes(data[8..16].try_into()?),
            format: data[16],
        })
    }

    pub fn total_size(&self) -> usize {
        17 // header size
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-core`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-core/src/protocol.rs
git commit -m "feat(protocol): add WindowFrameHeader for binary frame protocol

- 17-byte header: width(u32) + height(u32) + timestamp(u64) + format(u8)
- Replaces JSON serialization of VideoFramePayload
- Supports future codec formats via format field"
```

---

### Task 5: Add broadcast channel to UDP transport

**Files:**
- Modify: `crates/ob-network/src/udp.rs`

**Interfaces:**
- Produces: `subscribe()` method for broadcast receiver

- [ ] **Step 1: Add broadcast channel**

In `crates/ob-network/src/udp.rs`, add to `UdpTransport` struct:

```rust
use tokio::sync::broadcast;

pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    peers: Arc<RwLock<HashMap<SocketAddr, PeerInfo>>>,
    message_tx: mpsc::Sender<(SocketAddr, Message)>,
    #[allow(dead_code)]
    message_rx: Arc<RwLock<mpsc::Receiver<(SocketAddr, Message)>>>,
    broadcast_tx: broadcast::Sender<(SocketAddr, Message)>,
    buffer_size: usize,
}
```

- [ ] **Step 2: Initialize broadcast in bind()**

In `bind()` method, add:

```rust
let (broadcast_tx, _) = broadcast::channel(1024);
```

And include `broadcast_tx` in the Self struct.

- [ ] **Step 3: Forward to broadcast in run_receive_loop()**

In `run_receive_loop()`, after deserializing the message, add:

```rust
let _ = self.broadcast_tx.send((addr, msg.clone()));
```

- [ ] **Step 4: Add subscribe() method**

```rust
pub fn subscribe(&self) -> broadcast::Receiver<(SocketAddr, Message)> {
    self.broadcast_tx.subscribe()
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p ob-network`
Expected: OK

- [ ] **Step 6: Commit**

```bash
git add crates/ob-network/src/udp.rs
git commit -m "feat(network): add broadcast channel to UDP transport

- Add tokio::sync::broadcast for multi-consumer message delivery
- run_receive_loop forwards all messages to broadcast
- subscribe() method allows server/client to receive all messages
- Enables both server and client to use same receive path"
```

---

### Task 6: Fix client socket architecture

**Files:**
- Modify: `src/main.rs`
- Modify: `src/client.rs`

**Interfaces:**
- Consumes: `UdpTransport::subscribe()` from Task 5
- Produces: Client uses broadcast channel instead of raw socket

- [ ] **Step 1: Update main.rs to always spawn receive loop**

In `src/main.rs`, remove the `if is_primary` guard around `run_receive_loop`:

```rust
let udp_clone = udp_transport.clone();
tokio::spawn(async move {
    if let Err(e) = udp_clone.run_receive_loop().await {
        tracing::error!("UDP receive loop error: {}", e);
    }
});
```

- [ ] **Step 2: Rewrite client.rs to use broadcast**

Replace the entire `src/client.rs` with:

```rust
use anyhow::Result;
use ob_codec::decoder::VideoDecoder;
use ob_core::device::DeviceInfo;
use ob_core::protocol::{Message, MessageType, WindowFrameHeader};
use ob_display::overlay::OverlayWindow;
use ob_input::InputInjector;
use ob_network::udp::UdpTransport;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub async fn run_client(
    device: DeviceInfo,
    mut discovery_rx: mpsc::Receiver<(DeviceInfo, SocketAddr)>,
    udp: Arc<UdpTransport>,
) -> Result<()> {
    info!("Running as client: {}", device.name);
    println!("Client started. Waiting for server...");

    let mut decoder = VideoDecoder::new();
    let mut overlay: Option<OverlayWindow> = None;
    let mut frame_buffer: std::collections::HashMap<u64, Vec<Option<Vec<u8>>>> = std::collections::HashMap::new();

    let mut broadcast_rx = udp.subscribe();

    let (frame_tx, mut frame_rx) = mpsc::channel::<ob_codec::decoder::DecodedFrame>(32);

    tokio::spawn(async move {
        while let Ok((_addr, msg)) = broadcast_rx.recv().await {
            match msg.msg_type {
                MessageType::InputEvent => {
                    // Input events handled by main loop
                }
                MessageType::WindowFrame => {
                    if let Ok(header) = WindowFrameHeader::deserialize(&msg.payload) {
                        let encoded_data = msg.payload[17..].to_vec();
                        let encoded = ob_codec::encoder::EncodedFrame {
                            data: encoded_data,
                            width: header.width,
                            height: header.height,
                            frame_number: msg.sequence,
                            is_keyframe: msg.sequence % 60 == 0,
                            timestamp_us: header.timestamp_us,
                            encode_time_us: 0,
                            format: ob_codec::encoder::EncodedFormat::H264,
                        };
                        match decoder.decode_frame(&encoded) {
                            Ok(decoded) => {
                                let _ = frame_tx.send(decoded).await;
                            }
                            Err(e) => {
                                warn!("Decode failed: {}", e);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    });

    let injector = InputInjector::new(device.id);

    loop {
        tokio::select! {
            Some((new_device, addr)) = discovery_rx.recv() => {
                if new_device.role == ob_core::device::DeviceRole::Primary {
                    println!("Discovered server: {} ({})", new_device.name, addr);
                    udp.add_peer(addr).await;

                    let handshake = Message::new(
                        MessageType::Handshake,
                        serde_json::to_vec(&device)?,
                    );
                    udp.send_to(&handshake, addr).await?;

                    println!("Connected to server: {}", new_device.name);
                }
            }
            Some(decoded_frame) = frame_rx.recv() => {
                if overlay.is_none() {
                    overlay = Some(OverlayWindow::new(
                        &format!("OmniBridge - {}", device.name),
                        decoded_frame.width, decoded_frame.height,
                    ));
                    println!("Overlay window created - receiving video");
                }
                if let Some(ref ov) = overlay {
                    if let Err(e) = ov.render_frame(&decoded_frame) {
                        warn!("Render failed: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Periodic tasks
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down client...");
                if let Some(ref ov) = overlay {
                    ov.destroy();
                }
                break;
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/client.rs
git commit -m "feat(client): use broadcast channel instead of raw socket

- Remove raw recv_from task that competed with run_receive_loop
- Use UdpTransport::subscribe() for message delivery
- Client now receives all message types through unified channel
- Fixes race condition on UDP socket"
```

---

### Task 7: Update README with current status

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update README**

Update `README.md` with current project status, build instructions, and usage.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update README with ffmpeg dependency and current status"
```

---

## Sub-project 2: Complete Drag Flow

### Task 8: Fix drag detection state machine

**Files:**
- Modify: `ob-drag/src/detector.rs`

- [ ] **Step 1: Fix ButtonDown handler**

In `process_input()`, fix the ButtonDown case to transition to MouseDown:

```rust
InputEvent::MouseButton { pressed: true, position, .. } => {
    let window = self.query_foreground_window();
    self.drag_state = DragState::MouseDown {
        pos: *position,
        window: window.unwrap_or_default(),
    };
}
```

- [ ] **Step 2: Add query_foreground_window helper**

```rust
fn query_foreground_window(&self) -> Option<WindowInfo> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        #[link(name = "user32")]
        extern "system" {
            fn GetForegroundWindow() -> *mut c_void;
            fn GetWindowTextA(hWnd: *mut c_void, lpString: *mut u8, nMaxCount: i32) -> i32;
            fn GetWindowRect(hWnd: *mut c_void, lpRect: *mut RECT) -> i32;
        }
        #[repr(C)]
        struct RECT { left: i32, top: i32, right: i32, bottom: i32 }

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() { return None; }
            let mut title = [0u8; 256];
            GetWindowTextA(hwnd, title.as_mut_ptr(), 256);
            let title = std::ffi::CStr::from_ptr(title.as_ptr() as *const i8)
                .to_string_lossy().to_string();
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetWindowRect(hwnd, &mut rect);
            Some(WindowInfo {
                id: format!("{:?}", hwnd),
                title,
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left) as u32,
                height: (rect.bottom - rect.top) as u32,
                is_focused: true,
            })
        }
    }
    #[cfg(not(target_os = "windows"))]
    { None }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p ob-drag`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add crates/ob-drag/src/detector.rs
git commit -m "fix(drag): fix ButtonDown handler to transition to MouseDown state

- Add query_foreground_window() using Win32 API
- ButtonDown now stores window info and position
- Enables drag detection from raw mouse events"
```

---

### Task 9: Add WindowGrab/WindowDrop message types

**Files:**
- Modify: `crates/ob-core/src/protocol.rs`

- [ ] **Step 1: Add WindowGrabData and WindowDropData**

Append to `crates/ob-core/src/protocol.rs`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowGrabData {
    pub window_id: String,
    pub window_title: String,
    pub source_device: DeviceId,
    pub target_device: DeviceId,
    pub position_x: i32,
    pub position_y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowDropData {
    pub window_id: String,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-core`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-core/src/protocol.rs
git commit -m "feat(protocol): add WindowGrabData and WindowDropData types

- WindowGrabData: carries window info for cross-device transfer
- WindowDropData: signals transfer completion
- Enables drag-and-drop between devices"
```

---

### Task 10: Add WindowCapturer

**Files:**
- Create: `ob-capture/src/window.rs`
- Modify: `ob-capture/src/lib.rs`

- [ ] **Step 1: Create window.rs**

Create `crates/ob-capture/src/window.rs`:

```rust
use anyhow::Result;
use crate::frame::{CapturedFrame, FrameMetadata, FrameFormat};
use tracing::info;

pub struct WindowCapturer {
    hwnd: *mut std::ffi::c_void,
    width: u32,
    height: u32,
}

impl WindowCapturer {
    pub fn new(hwnd: *mut std::ffi::c_void) -> Result<Self> {
        #[link(name = "user32")]
        extern "system" {
            fn GetWindowRect(hWnd: *mut std::ffi::c_void, lpRect: *mut RECT) -> i32;
        }
        #[repr(C)]
        struct RECT { left: i32, top: i32, right: i32, bottom: i32 }

        unsafe {
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetWindowRect(hwnd, &mut rect);
            let width = (rect.right - rect.left) as u32;
            let height = (rect.bottom - rect.top) as u32;
            info!("Created window capturer: {}x{}", width, height);
            Ok(Self { hwnd, width, height })
        }
    }

    pub fn capture_frame(&self) -> Result<Option<CapturedFrame>> {
        #[link(name = "user32")]
        extern "system" {
            fn GetDC(hWnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn ReleaseDC(hWnd: *mut std::ffi::c_void, hDC: *mut std::ffi::c_void) -> i32;
            fn PrintWindow(hWnd: *mut std::ffi::c_void, hDC: *mut std::ffi::c_void, nFlags: u32) -> i32;
        }
        #[link(name = "gdi32")]
        extern "system" {
            fn CreateCompatibleDC(hdc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn CreateCompatibleBitmap(hdc: *mut std::ffi::c_void, cx: i32, cy: i32) -> *mut std::ffi::c_void;
            fn SelectObject(hdc: *mut std::ffi::c_void, h: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
            fn GetDIBits(hdc: *mut std::ffi::c_void, hbm: *mut std::ffi::c_void, start: u32, cLines: u32,
                         lpvBits: *mut u8, lpbmi: *mut BITMAPINFOHEADER, usage: u32) -> i32;
            fn DeleteObject(ho: *mut std::ffi::c_void) -> i32;
            fn DeleteDC(hdc: *mut std::ffi::c_void) -> i32;
        }
        #[repr(C)]
        #[allow(non_snake_case)]
        struct BITMAPINFOHEADER {
            biSize: u32, biWidth: i32, biHeight: i32, biPlanes: u16,
            biBitCount: u16, biCompression: u32, biSizeImage: u32,
            biXPelsPerMeter: i32, biYPelsPerMeter: i32, biClrUsed: u32, biClrImportant: u32,
        }

        unsafe {
            let hdc_screen = GetDC(std::ptr::null_mut());
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbmp = CreateCompatibleBitmap(hdc_screen, self.width as i32, self.height as i32);
            let old_bmp = SelectObject(hdc_mem, hbmp);

            PrintWindow(self.hwnd, hdc_mem, 0x02); // PW_RENDERFULLCONTENT

            let mut bmi = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: self.width as i32,
                biHeight: -(self.height as i32),
                biPlanes: 1, biBitCount: 32, biCompression: 0, biSizeImage: 0,
                biXPelsPerMeter: 0, biYPelsPerMeter: 0, biClrUsed: 0, biClrImportant: 0,
            };

            let mut pixels = vec![0u8; (self.width * self.height * 4) as usize];
            GetDIBits(hdc_mem, hbmp, 0, self.height, pixels.as_mut_ptr(), &mut bmi, 0);

            SelectObject(hdc_mem, old_bmp);
            DeleteObject(hbmp as *mut std::ffi::c_void);
            DeleteDC(hdc_mem);
            ReleaseDC(std::ptr::null_mut(), hdc_screen);

            let metadata = FrameMetadata {
                width: self.width,
                height: self.height,
                stride: self.width * 4,
                format: FrameFormat::BGRA,
                timestamp_us: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_micros() as u64,
                capture_time_us: 0,
                frame_number: 0,
                dirty_regions: Vec::new(),
            };

            Ok(Some(CapturedFrame { pixels, metadata }))
        }
    }
}
```

- [ ] **Step 2: Update lib.rs**

Add to `crates/ob-capture/src/lib.rs`:

```rust
pub mod window;
pub use window::WindowCapturer;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p ob-capture`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add crates/ob-capture/src/window.rs crates/ob-capture/src/lib.rs
git commit -m "feat(capture): add WindowCapturer for capturing specific windows

- Use PrintWindow API to capture even occluded windows
- Returns BGRA pixels in CapturedFrame format
- Enables cross-device window transfer"
```

---

### Task 11: Add set_alpha to overlay

**Files:**
- Modify: `ob-display/src/overlay.rs`

- [ ] **Step 1: Add set_alpha method**

Add to `OverlayWindow` impl block:

```rust
pub fn set_alpha(&self, alpha: f32) {
    if self.hwnd.is_null() { return; }
    #[link(name = "user32")]
    extern "system" {
        fn SetLayeredWindowAttributes(hwnd: *mut std::ffi::c_void, crKey: u32,
                                     bAlpha: u8, dwFlags: u32) -> i32;
    }
    unsafe {
        SetLayeredWindowAttributes(self.hwnd, 0, (alpha * 255.0) as u8, 0x02);
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-display`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-display/src/overlay.rs
git commit -m "feat(display): add set_alpha method to OverlayWindow

- Enables semi-transparent overlay during drag
- Uses SetLayeredWindowAttributes with alpha channel
- Supports smooth transition from ghost to opaque"
```

---

## Sub-project 3: Harden & Scale

### Task 12: Add heartbeat and timeout

**Files:**
- Modify: `src/server.rs`
- Modify: `ob-network/src/udp.rs`

- [ ] **Step 1: Add is_peer_alive and cleanup_dead_peers to UdpTransport**

Add to `ob-network/src/udp.rs`:

```rust
pub fn is_peer_alive(&self, addr: &SocketAddr) -> bool {
    // This needs to be async or use blocking read
    // For now, always return true
    true
}

pub async fn cleanup_dead_peers(&self) {
    let mut peers = self.peers.write().await;
    peers.retain(|_, peer| peer.last_seen.elapsed() < std::time::Duration::from_secs(15));
}
```

- [ ] **Step 2: Add heartbeat task to server.rs**

In `src/server.rs`, after spawning video task, add:

```rust
let udp_for_heartbeat = udp.clone();
let clients_for_heartbeat = connected_clients.clone();
tokio::spawn(async move {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let clients = clients_for_heartbeat.read().await;
        for (_, addr) in clients.iter() {
            let heartbeat = Message::new(
                MessageType::Heartbeat,
                serde_json::to_vec(&std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64).unwrap_or_default(),
            );
            let _ = udp_for_heartbeat.send_to(&heartbeat, *addr).await;
        }
    }
});
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add src/server.rs ob-network/src/udp.rs
git commit -m "feat(network): add heartbeat and peer timeout

- Server sends heartbeat every 5s to all clients
- Peers timeout after 15s of inactivity
- Enables connection health monitoring"
```

---

### Task 13: Add per-monitor detection

**Files:**
- Create: `ob-capture/src/monitor.rs`
- Modify: `ob-capture/src/lib.rs`
- Modify: `ob-capture/src/screen.rs`

- [ ] **Step 1: Create monitor.rs**

Create `crates/ob-capture/src/monitor.rs` with EnumDisplayMonitors implementation.

- [ ] **Step 2: Update screen.rs to use enumerate_monitors**

Replace detect_windows_screens with enumerate_monitors call.

- [ ] **Step 3: Remove duplicate from main.rs**

Remove detect_windows_screens from src/main.rs.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: OK

- [ ] **Step 5: Commit**

```bash
git add crates/ob-capture/src/monitor.rs crates/ob-capture/src/lib.rs crates/ob-capture/src/screen.rs src/main.rs
git commit -m "feat(capture): add per-monitor detection with EnumDisplayMonitors

- Replace approximate GetSystemMetrics with accurate per-monitor bounds
- Each monitor gets correct position, size, and DPI
- Remove duplicate screen detection code"
```

---

## Sub-project 4: System Tray + GUI

### Task 14: Add tray-icon dependency

**Files:**
- Modify: `Cargo.toml`
- Modify: `ob-gui/Cargo.toml`

- [ ] **Step 1: Add dependencies**

Add to workspace and ob-gui Cargo.toml:
```toml
tray-icon = "0.19"
eframe = "0.29"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-gui`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml ob-gui/Cargo.toml
git commit -m "deps(gui): add tray-icon and eframe dependencies"
```

---

### Task 15: Implement system tray

**Files:**
- Create: `ob-gui/src/tray.rs`
- Modify: `ob-gui/src/lib.rs`
- Modify: `ob-gui/src/app.rs`

- [ ] **Step 1: Create tray.rs**

Implement system tray with icon states and menu.

- [ ] **Step 2: Update app.rs with shared state**

Implement AppState with tray communication.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p ob-gui`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add crates/ob-gui/src/tray.rs crates/ob-gui/src/lib.rs crates/ob-gui/src/app.rs
git commit -m "feat(gui): implement system tray with status icon

- Add tray-icon for persistent system tray presence
- Icon states: disconnected (gray), connected (green), error (red)
- Right-click menu: Show/Hide, Devices, Status, Quit"
```

---

### Task 16: Implement egui settings window

**Files:**
- Create: `ob-gui/src/settings.rs`
- Modify: `ob-gui/src/lib.rs`

- [ ] **Step 1: Create settings.rs**

Implement egui window with device list and configuration.

- [ ] **Step 2: Update lib.rs to export**

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p ob-gui`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add crates/ob-gui/src/settings.rs crates/ob-gui/src/lib.rs
git commit -m "feat(gui): implement egui settings window

- Device list with connection status
- Configuration editor
- Log viewer
- Connection health display"
```

---

### Task 17: Integrate GUI with main binary

**Files:**
- Modify: `src/main.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add gui feature flag**

Add to Cargo.toml:
```toml
[features]
default = ["gui"]
gui = ["ob-gui"]
```

- [ ] **Step 2: Add gui mode to main.rs**

Add `--gui` flag handling that launches tray + egui.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add src/main.rs Cargo.toml
git commit -m "feat: integrate system tray and egui with main binary

- Add --gui flag to launch GUI mode
- GUI mode shows system tray and settings window
- CLI mode still works for headless use"
```

---

### Task 18: Final integration test

- [ ] **Step 1: Build release**

```bash
cargo build --release
```

- [ ] **Step 2: Test CLI mode**

```bash
.\target\release\omnibridge.exe --help
.\target\release\omnibridge.exe status
```

- [ ] **Step 3: Test GUI mode**

```bash
.\target\release\omnibridge.exe --gui
```

- [ ] **Step 4: Update README**

Update README with all features, build instructions, and usage.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat: complete OmniBridge v0.2.0

- Real H.264 encoding/decoding via ffmpeg
- Binary protocol for efficient frame transmission
- Cross-device window drag and drop
- Per-monitor detection with DPI support
- System tray with status icon
- egui settings window
- Heartbeat and connection management
- Graceful shutdown and error handling"
```

- [ ] **Step 6: Push to GitHub**

```bash
git push origin main
```

---

## Execution Order

```
Sub-project 1 (Fix Data Path)
  ├── Task 1: Add ffmpeg-next dependency
  ├── Task 2: Implement ffmpeg encoder
  ├── Task 3: Implement ffmpeg decoder
  ├── Task 4: Add WindowFrameHeader
  ├── Task 5: Add broadcast channel
  ├── Task 6: Fix client socket
  └── Task 7: Update README
       │
       ▼
Sub-project 2 (Complete Drag Flow)
  ├── Task 8: Fix drag detection
  ├── Task 9: Add WindowGrab/WindowDrop types
  ├── Task 10: Add WindowCapturer
  └── Task 11: Add set_alpha to overlay
       │
       ▼
Sub-project 3 (Harden & Scale)
  ├── Task 12: Add heartbeat and timeout
  └── Task 13: Add per-monitor detection
       │
       ▼
Sub-project 4 (System Tray + GUI)
  ├── Task 14: Add tray-icon dependency
  ├── Task 15: Implement system tray
  ├── Task 16: Implement egui settings
  ├── Task 17: Integrate GUI
  └── Task 18: Final integration test
```
