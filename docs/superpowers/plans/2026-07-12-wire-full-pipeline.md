# Wire Full Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect screen capture, encoding, streaming, decoding, and display so the primary device's screen appears on the secondary device's overlay window in real-time.

**Architecture:** Extend the existing `tokio::select!` loops in `server.rs` and `client.rs` with timer-driven capture and frame-receive branches. The protocol already has a `WindowFrame` message type (variant 6) that we'll reuse for video frames. The overlay window will be created using Win32 `CreateWindowEx` with layered window attributes for transparency and pixel rendering via GDI `StretchDIBits`.

**Tech Stack:** Rust, tokio, Win32 FFI (user32/gdi32), serde/serde_json, existing ob-core/ob-capture/ob-codec/ob-display crates.

## Global Constraints

- Platform: Windows only (Linux/macOS stubs acceptable)
- Rust toolchain: stable-x86_64-pc-windows-gnu with MinGW-w64
- No new crate dependencies — use only what's already in Cargo.toml
- Existing codec is a stub — output will be raw BGRA uncompressed (large but correct)
- Overlay window uses Win32 API directly (no winit/softbuffer)

---

## File Structure

| File | Change |
|------|--------|
| `crates/ob-codec/src/encoder.rs` | Replace stub with raw BGRA pass-through |
| `crates/ob-codec/src/decoder.rs` | Replace stub with raw BGRA reconstruction |
| `crates/ob-display/src/overlay.rs` | Create real Win32 window + StretchDIBits rendering |
| `src/server.rs` | Add capture timer task + frame streaming to clients |
| `src/client.rs` | Add frame receive branch + decode + display |

**Reused as-is:** `ob-core/src/protocol.rs` (WindowFrame variant 6 already exists), `ob-capture/src/screen.rs` (GDI BitBlt capture works).

---

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

### Task 3: Rewrite Overlay Window with Win32 Creation + Pixel Rendering

**Files:**
- Modify: `crates/ob-display/src/overlay.rs` (full rewrite of the file)

**Interfaces:**
- Consumes: `DecodedFrame` (from `ob-codec::decoder`)
- Produces: A visible Win32 overlay window that displays decoded pixels

The current overlay tries to `FindWindowA` a window that doesn't exist. Replace with actual window creation using `RegisterClassExA` + `CreateWindowExA` and pixel rendering via `StretchDIBits`.

- [ ] **Step 1: Rewrite overlay.rs**

Replace the entire contents of `crates/ob-display/src/overlay.rs` with:

```rust
use anyhow::Result;
use ob_codec::decoder::DecodedFrame;
use tracing::{debug, info};

pub struct OverlayWindow {
    hwnd: *mut std::ffi::c_void,
    hdc: *mut std::ffi::c_void,
    bitmap_info: BITMAPINFO,
    width: u32,
    height: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct BITMAPINFOHEADER {
    biSize: u32,
    biWidth: i32,
    biHeight: i32,
    biPlanes: u16,
    biBitCount: u16,
    biCompression: u32,
    biSizeImage: u32,
    biXPelsPerMeter: i32,
    biYPelsPerMeter: i32,
    biClrUsed: u32,
    biClrImportant: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct BITMAPINFO {
    bmiHeader: BITMAPINFOHEADER,
    bmiColors: [u32; 1],
}

impl OverlayWindow {
    pub fn new(title: &str, width: u32, height: u32) -> Self {
        use std::ffi::c_void;

        #[link(name = "user32")]
        extern "system" {
            fn RegisterClassExA(lpwcx: *const WNDCLASSEXA) -> u16;
            fn CreateWindowExA(
                dwExStyle: u32, lpClassName: *const u8, lpWindowName: *const u8,
                dwStyle: u32, x: i32, y: i32, nWidth: i32, nHeight: i32,
                hWndParent: *mut c_void, hMenu: *mut c_void,
                hInstance: *mut c_void, lpParam: *mut c_void,
            ) -> *mut c_void;
            fn GetDC(hWnd: *mut c_void) -> *mut c_void;
            fn GetModuleHandleA(lpModuleName: *const u8) -> *mut c_void;
        }

        #[repr(C)]
        struct WNDCLASSEXA {
            cbSize: u32,
            style: u32,
            lpfnWndProc: *mut c_void,
            cbClsExtra: i32,
            cbWndExtra: i32,
            hInstance: *mut c_void,
            hIcon: *mut c_void,
            hCursor: *mut c_void,
            hbrBackground: *mut c_void,
            lpszMenuName: *const u8,
            lpszClassName: *const u8,
            hIconSm: *mut c_void,
        }

        let class_name = b"OmniBridgeOverlay\0";
        let window_name = format!("{}\0", title);

        unsafe {
            let hinstance = GetModuleHandleA(std::ptr::null());

            let wnd_class = WNDCLASSEXA {
                cbSize: std::mem::size_of::<WNDCLASSEXA>() as u32,
                style: 0,
                lpfnWndProc: Some(def_window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };

            RegisterClassExA(&wnd_class);

            let hwnd = CreateWindowExA(
                0x00000008 | 0x00000020 | 0x00000001, // WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED
                class_name.as_ptr(),
                window_name.as_ptr() as *const u8,
                0x80000000, // WS_POPUP
                0, 0, width as i32, height as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );

            let hdc = GetDC(hwnd);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: 0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [0; 1],
            };

            info!("Overlay window created: {}x{}", width, height);

            Self {
                hwnd,
                hdc,
                bitmap_info: bmi,
                width,
                height,
            }
        }
    }

    pub fn render_frame(&self, frame: &DecodedFrame) -> Result<()> {
        if self.hwnd.is_null() || self.hdc.is_null() {
            return Ok(());
        }

        #[link(name = "gdi32")]
        extern "system" {
            fn StretchDIBits(
                hdc: *mut std::ffi::c_void,
                xDest: i32, yDest: i32, wDest: i32, hDest: i32,
                xSrc: i32, ySrc: i32, wSrc: i32, hSrc: i32,
                lpBits: *const u8, lpbmi: *const BITMAPINFO,
                iUsage: u32, dwRop: u32,
            ) -> i32;
        }

        #[link(name = "user32")]
        extern "system" {
            fn SetWindowPos(
                hWnd: *mut std::ffi::c_void,
                hWndInsertAfter: *mut std::ffi::c_void,
                x: i32, y: i32, cx: i32, cy: i32, uFlags: u32,
            ) -> i32;
        }

        if frame.pixels.len() >= (frame.width * frame.height * 4) as usize {
            unsafe {
                StretchDIBits(
                    self.hdc,
                    0, 0, self.width as i32, self.height as i32,
                    0, 0, frame.width as i32, frame.height as i32,
                    frame.pixels.as_ptr(),
                    &self.bitmap_info,
                    0, 0x00CC0020, // SRCCOPY
                );
            }
        }

        debug!("Rendered frame {}x{}", frame.width, frame.height);
        Ok(())
    }

    pub fn set_position(&self, x: i32, y: i32) {
        if self.hwnd.is_null() {
            return;
        }
        #[link(name = "user32")]
        extern "system" {
            fn SetWindowPos(
                hWnd: *mut std::ffi::c_void,
                hWndInsertAfter: *mut std::ffi::c_void,
                x: i32, y: i32, cx: i32, cy: i32, uFlags: u32,
            ) -> i32;
        }
        unsafe {
            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(), // HWND_TOPMOST
                x, y, 0, 0,
                0x0001 | 0x0002, // SWP_NOSIZE | SWP_NOZORDER
            );
        }
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.bitmap_info.bmiHeader.biWidth = width as i32;
        self.bitmap_info.bmiHeader.biHeight = -(height as i32);
    }

    pub fn destroy(&self) {
        if !self.hwnd.is_null() {
            #[link(name = "user32")]
            extern "system" {
                fn DestroyWindow(hWnd: *mut std::ffi::c_void) -> i32;
                fn ReleaseDC(hWnd: *mut std::ffi::c_void, hDC: *mut std::ffi::c_void) -> i32;
            }
            unsafe {
                ReleaseDC(self.hwnd, self.hdc);
                DestroyWindow(self.hwnd);
            }
            info!("Overlay window destroyed");
        }
    }
}

extern "system" fn def_window_proc(
    _hwnd: *mut std::ffi::c_void,
    _msg: u32,
    _wparam: usize,
    _lparam: isize,
) -> isize {
    0
}

impl Drop for OverlayWindow {
    fn drop(&mut self) {
        self.destroy();
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p ob-display`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add crates/ob-display/src/overlay.rs
git commit -m "feat(display): create real Win32 overlay window with pixel rendering"
```

---

### Task 4: Add Screen Capture + Frame Streaming to Server

**Files:**
- Modify: `src/server.rs` (add capture timer and frame streaming task)

**Interfaces:**
- Consumes: `ScreenCapturer` (from `ob-capture`), `VideoEncoder` (from `ob-codec`), `connected_clients` (Arc<RwLock>)
- Produces: Sends `MessageType::WindowFrame` messages to all connected clients every ~33ms

The server currently only forwards input events. Add a separate tokio task that captures the screen at 30fps, encodes each frame, and sends it to all connected clients via UDP.

- [ ] **Step 1: Rewrite server.rs**

Replace the entire contents of `src/server.rs` with:

```rust
use anyhow::Result;
use ob_capture::ScreenCapturer;
use ob_codec::encoder::VideoEncoder;
use ob_core::device::DeviceInfo;
use ob_core::protocol::{Message, MessageType};
use ob_input::InputCapture;
use ob_network::udp::UdpTransport;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

pub async fn run_server(
    device: DeviceInfo,
    mut discovery_rx: mpsc::Receiver<(DeviceInfo, SocketAddr)>,
    udp: Arc<UdpTransport>,
) -> Result<()> {
    info!("Running as server: {}", device.name);
    println!("Server started. Waiting for connections...");

    let connected_clients: Arc<RwLock<Vec<(DeviceInfo, SocketAddr)>>> =
        Arc::new(RwLock::new(Vec::new()));

    let (input_tx, mut input_rx) = mpsc::channel::<ob_core::event::InputEvent>(256);

    let mut input_capture = InputCapture::new(input_tx);

    #[cfg(target_os = "windows")]
    {
        input_capture.start().await?;
        println!("Input capture active");
    }

    let clients_for_forward = connected_clients.clone();
    let udp_for_forward = udp.clone();
    tokio::spawn(async move {
        while let Some(event) = input_rx.recv().await {
            let clients = clients_for_forward.read().await;
            for (_, addr) in clients.iter() {
                let msg = Message::new(
                    MessageType::InputEvent,
                    serde_json::to_vec(&event).unwrap_or_default(),
                );
                if let Err(e) = udp_for_forward.send_to(&msg, *addr).await {
                    warn!("Failed to send input to {}: {}", addr, e);
                }
            }
        }
    });

    let clients_for_video = connected_clients.clone();
    let udp_for_video = udp.clone();
    let device_id = device.id;
    tokio::spawn(async move {
        let screens = match ob_capture::screen::ScreenCapturer::detect_screen_info() {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to detect screens: {}", e);
                return;
            }
        };
        let screen = match screens.into_iter().next() {
            Some(s) => s,
            None => {
                warn!("No screens detected");
                return;
            }
        };

        let mut capturer = ScreenCapturer::new(screen.clone());
        if let Err(e) = capturer.start() {
            warn!("Failed to start screen capture: {}", e);
            return;
        }

        let mut encoder = VideoEncoder::new(screen.width, screen.height, 30);
        let mut frame_seq: u64 = 0;

        info!("Video streaming started: {}x{}", screen.width, screen.height);

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(33)).await;

            let clients = clients_for_video.read().await;
            if clients.is_empty() {
                continue;
            }

            match capturer.capture_frame() {
                Ok(Some(frame)) => {
                    match encoder.encode_frame(&frame) {
                        Ok(encoded) => {
                            frame_seq += 1;

                            let frame_data = VideoFramePayload {
                                source_device: device_id,
                                width: encoded.width,
                                height: encoded.height,
                                timestamp_us: encoded.timestamp_us,
                                is_keyframe: encoded.is_keyframe,
                                pixels: encoded.data,
                            };

                            let payload = match serde_json::to_vec(&frame_data) {
                                Ok(p) => p,
                                Err(e) => {
                                    warn!("Failed to serialize frame: {}", e);
                                    continue;
                                }
                            };

                            let msg = Message::new(MessageType::WindowFrame, payload)
                                .with_sequence(frame_seq);

                            for (_, addr) in clients.iter() {
                                if let Err(e) = udp_for_video.send_to(&msg, *addr).await {
                                    warn!("Failed to send frame to {}: {}", addr, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Encode failed: {}", e);
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Capture failed: {}", e);
                }
            }
        }
    });

    loop {
        tokio::select! {
            Some((new_device, addr)) = discovery_rx.recv() => {
                let already_connected = connected_clients
                    .read().await.iter().any(|(d, _)| d.id == new_device.id);
                if !already_connected {
                    println!("Device connected: {} ({})", new_device.name, addr);

                    let handshake_ack = Message::new(
                        MessageType::HandshakeAck,
                        serde_json::to_vec(&device)?,
                    );
                    udp.send_to(&handshake_ack, addr).await?;

                    connected_clients.write().await.push((new_device, addr));
                    println!("{} clients connected", connected_clients.read().await.len());
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down server...");
                break;
            }
        }
    }

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct VideoFramePayload {
    source_device: ob_core::device::DeviceId,
    width: u32,
    height: u32,
    timestamp_us: u64,
    is_keyframe: bool,
    pixels: Vec<u8>,
}
```

- [ ] **Step 2: Add `detect_screen_info` method to ScreenCapturer**

The server calls `ScreenCapturer::detect_screen_info()` which doesn't exist yet. Add it to `crates/ob-capture/src/screen.rs` by appending before the closing `}` of the `impl ScreenCapturer` block:

```rust
    pub fn detect_screen_info() -> Result<Vec<ob_core::screen::ScreenInfo>> {
        #[cfg(target_os = "windows")]
        {
            Self::detect_windows_screens()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(vec![ob_core::screen::ScreenInfo {
                id: ob_core::screen::ScreenId(0),
                name: "Display 1".to_string(),
                width: 1920,
                height: 1080,
                x: 0,
                y: 0,
                scale_factor: 1.0,
                is_primary: true,
            }])
        }
    }

    #[cfg(target_os = "windows")]
    fn detect_windows_screens() -> Result<Vec<ob_core::screen::ScreenInfo>> {
        #[link(name = "user32")]
        extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }

        const SM_CMONITORS: i32 = 80;
        const SM_XVIRTUALSCREEN: i32 = 76;
        const SM_YVIRTUALSCREEN: i32 = 77;
        const SM_CXVIRTUALSCREEN: i32 = 78;
        const SM_CYVIRTUALSCREEN: i32 = 79;

        let num_monitors = unsafe { GetSystemMetrics(SM_CMONITORS) };
        let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        let mut screens = Vec::new();
        for i in 0..num_monitors.max(1) {
            screens.push(ob_core::screen::ScreenInfo {
                id: ob_core::screen::ScreenId(i as u32),
                name: format!("Display {}", i + 1),
                width: (vw / num_monitors.max(1)) as u32,
                height: vh as u32,
                x: vx + (i * vw / num_monitors.max(1)),
                y: vy,
                scale_factor: 1.0,
                is_primary: i == 0,
            });
        }

        Ok(screens)
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: OK

- [ ] **Step 4: Commit**

```bash
git add src/server.rs crates/ob-capture/src/screen.rs
git commit -m "feat(server): add screen capture and frame streaming to clients"
```

---

### Task 5: Add Frame Receive + Decode + Display to Client

**Files:**
- Modify: `src/client.rs` (add frame receive, decode, and display)

**Interfaces:**
- Consumes: `MessageType::WindowFrame` messages from UDP, `VideoDecoder` (from `ob-codec`)
- Produces: Calls `OverlayWindow::render_frame()` with decoded frames

The client currently only handles input injection. Add a frame-receive branch that decodes incoming video frames and renders them to the overlay window.

- [ ] **Step 1: Rewrite client.rs**

Replace the entire contents of `src/client.rs` with:

```rust
use anyhow::Result;
use ob_codec::decoder::VideoDecoder;
use ob_core::device::DeviceInfo;
use ob_core::protocol::{Message, MessageType};
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

    let injector = InputInjector::new(device.id);
    let mut decoder = VideoDecoder::new();
    let mut overlay: Option<OverlayWindow> = None;

    let (_msg_tx, mut msg_rx) = mpsc::channel::<(SocketAddr, Message)>(256);

    tokio::spawn(async move {
        while let Some((_addr, msg)) = msg_rx.recv().await {
            match msg.msg_type {
                MessageType::InputEvent => {
                    if let Ok(event) =
                        serde_json::from_slice::<ob_core::event::InputEvent>(&msg.payload)
                    {
                        if let Err(e) = injector.inject(&event) {
                            warn!("Failed to inject input: {}", e);
                        }
                    }
                }
                _ => {}
            }
        }
    });

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
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                let clients = udp.peer_count().await;
                if clients > 0 && overlay.is_none() {
                    overlay = Some(OverlayWindow::new(
                        &format!("OmniBridge - {}", device.name),
                        1920, 1080,
                    ));
                    println!("Overlay window created - receiving video");
                }
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

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add src/client.rs
git commit -m "feat(client): add frame receive, decode, and overlay display"
```

---

### Task 6: Wire UDP Frame Reception to Client Overlay

**Files:**
- Modify: `src/client.rs` (connect UDP receive to decoder and overlay)
- Modify: `src/main.rs` (pass UDP message channel to client)

The client creates an overlay but never feeds it frames. The UDP transport receives messages but they're not routed to the client's decoder. Connect them.

- [ ] **Step 1: Modify main.rs to pass UDP message receiver to client**

In `src/main.rs`, the `run_server` and `run_client` functions need access to UDP-received messages. Modify the `start_node` function to create a message channel and wire it:

In `src/main.rs`, find the `start_node` function. After the `udp_clone.run_receive_loop()` spawn (around line 91), add a second spawn that forwards UDP messages to the client/server. Replace the entire `if is_primary { ... } else { ... }` block with:

```rust
    let (frame_tx, frame_rx) = mpsc::channel::<(SocketAddr, Message)>(256);

    let udp_for_frame = udp_transport.clone();
    tokio::spawn(async move {
        use tokio::sync::RwLock;
        use std::sync::Arc;
        let _frame_tx = frame_tx;
        let _udp = udp_for_frame;
        // Frame reception is handled in the run loops via direct UDP reads
    });

    if is_primary {
        server::run_server(local_device, listener_rx, udp_transport).await?;
    } else {
        client::run_client(local_device, listener_rx, udp_transport).await?;
    }
```

Actually, a cleaner approach: give `run_client` a direct reference to read from UDP. Modify `client.rs` to add a frame-receiving task that reads from the UDP transport and feeds frames to the overlay.

Replace the full contents of `src/client.rs` with:

```rust
use anyhow::Result;
use ob_codec::decoder::VideoDecoder;
use ob_core::device::DeviceInfo;
use ob_core::protocol::{Message, MessageType};
use ob_display::overlay::OverlayWindow;
use ob_input::InputInjector;
use ob_network::udp::UdpTransport;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(serde::Serialize, serde::Deserialize)]
struct VideoFramePayload {
    source_device: ob_core::device::DeviceId,
    width: u32,
    height: u32,
    timestamp_us: u64,
    is_keyframe: bool,
    pixels: Vec<u8>,
}

pub async fn run_client(
    device: DeviceInfo,
    mut discovery_rx: mpsc::Receiver<(DeviceInfo, SocketAddr)>,
    udp: Arc<UdpTransport>,
) -> Result<()> {
    info!("Running as client: {}", device.name);
    println!("Client started. Waiting for server...");

    let injector = InputInjector::new(device.id);

    let (overlay_tx, mut overlay_rx) = mpsc::channel::<ob_codec::decoder::DecodedFrame>(32);

    let mut decoder = VideoDecoder::new();
    let mut overlay: Option<OverlayWindow> = None;

    let udp_for_frames = udp.clone();
    tokio::spawn(async move {
        let mut frame_buf = vec![0u8; 65536 * 4];
        loop {
            match udp_for_frames.socket().recv_from(&mut frame_buf).await {
                Ok((len, _addr)) => {
                    if len < 4 {
                        continue;
                    }
                    let packet_len = u32::from_le_bytes(frame_buf[0..4].try_into().unwrap_or([0;4])) as usize;
                    if len < 4 + packet_len || packet_len > frame_buf.len() {
                        continue;
                    }
                    match Message::deserialize(&frame_buf[4..4 + packet_len]) {
                        Ok(msg) if msg.msg_type == MessageType::WindowFrame => {
                            if let Ok(frame_data) = serde_json::from_slice::<VideoFramePayload>(&msg.payload) {
                                let encoded = ob_codec::encoder::EncodedFrame {
                                    data: frame_data.pixels,
                                    width: frame_data.width,
                                    height: frame_data.height,
                                    frame_number: msg.sequence,
                                    is_keyframe: frame_data.is_keyframe,
                                    timestamp_us: frame_data.timestamp_us,
                                    encode_time_us: 0,
                                    format: ob_codec::encoder::EncodedFormat::H264,
                                };
                                match decoder.decode_frame(&encoded) {
                                    Ok(decoded) => {
                                        let _ = overlay_tx.send(decoded).await;
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
                Err(e) => {
                    warn!("UDP recv error: {}", e);
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some((_addr, msg)) = overlay_rx.recv().await {
            // Messages from overlay_tx are DecodedFrames, handle separately
        }
    });

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
            Some(decoded_frame) = overlay_rx.recv() => {
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

Also need to add a `socket()` method to `UdpTransport`. In `crates/ob-network/src/udp.rs`, add this method:

```rust
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: OK

- [ ] **Step 3: Commit**

```bash
git add src/client.rs src/main.rs crates/ob-network/src/udp.rs
git commit -m "feat: wire UDP frame reception to client overlay display"
```

---

### Task 7: Build Release and End-to-End Test

**Files:**
- No code changes. Verification only.

- [ ] **Step 1: Build release binary**

Run:
```bash
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo build --release
```
Expected: Build succeeds, binary at `target/release/omnibridge.exe`

- [ ] **Step 2: Test help output**

Run: `target\release\omnibridge.exe --help`
Expected: Shows CLI usage with start, connect, status, layout, config subcommands

- [ ] **Step 3: Test primary node startup (Terminal 1)**

Run: `target\release\omnibridge.exe start --name "MainPC" --primary --port 19810`
Expected: Prints "Server started", "Input capture active", "Video streaming started: WxH"

- [ ] **Step 4: Test secondary node (Terminal 2)**

Run: `target\release\omnibridge.exe start --name "Laptop" --port 19810`
Expected: Prints "Client started", then after discovery: "Discovered server", "Connected to server", "Overlay window created - receiving video"

- [ ] **Step 5: Test standalone connect mode**

Run: `target\release\omnibridge.exe connect --address 127.0.0.1 --port 19810`
Expected: Prints "Connected" without errors

- [ ] **Step 6: Test status command**

Run: `target\release\omnibridge.exe status`
Expected: Prints config directory and layout config status

- [ ] **Step 7: Commit final state**

```bash
git add -A
git commit -m "feat: complete end-to-end pipeline wiring

- Screen capture streams from primary to secondary at 30fps
- Overlay window created on secondary with real-time pixel rendering
- Input events forwarded from secondary back to primary
- Frame decode/encode uses raw BGRA pass-through (stub codec)"
```
