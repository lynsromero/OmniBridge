# OmniBridge Production Hardening & Drag Flow Design

Date: 2026-07-12
Status: Approved

## Overview

Three sub-projects to take OmniBridge from demo-quality pipeline to functional cross-device KVM tool.

| Sub-project | Goal | Key deliverable |
|-------------|------|-----------------|
| 1. Fix Data Path | Binary protocol + real codec + chunking | 1080p streaming at 30fps over UDP |
| 2. Complete Drag Flow | Edge-crossing window drag | Drag window to edge → appears on other device |
| 3. Harden & Scale | Multi-monitor + connection mgmt | Works on real multi-monitor setups, survives network blips |
| 4. System Tray + GUI | System tray icon + egui settings window | Non-intrusive status icon, device list, configuration |

## Sub-project 1: Fix Data Path

### Problem

Current pipeline serializes raw BGRA pixels as JSON (~8MB/frame at 1080p). This exceeds UDP MTU and will be silently dropped. The encoder/decoder are stubs that just copy raw bytes.

### Design

#### 1.1 Binary Protocol for WindowFrame

Replace `serde_json::to_vec(&VideoFramePayload)` with direct binary encoding.

**WindowFrame payload format (replaces VideoFramePayload):**

```
[width: u32 LE] [height: u32 LE] [timestamp_us: u64 LE] [format: u8] [frame_data: bytes]
```

- `format`: 0 = H264 (initial codec)
- `frame_data`: encoded frame bytes from ffmpeg (typically 20-200KB for 1080p)
- No intermediate struct — encoder output goes directly into `Message.payload`
- The existing `Message.serialize()` envelope (msg_type + timestamp + sequence + length) wraps this

**Changes to `ob-core/src/protocol.rs`:**
- Add `WindowFrameHeader` struct with `width`, `height`, `timestamp_us`, `format` fields
- Add `WindowFrameHeader::serialize()` / `WindowFrameHeader::deserialize()` methods
- Keep existing `MessageType::WindowFrame` — no protocol version change

#### 1.2 FFmpeg Integration

Add `ffmpeg-next` crate for software H.264 encoding/decoding.

**Encoder (`ob-codec/src/encoder.rs`):**
- Replace `encode_software_h264` stub with real ffmpeg encoding
- Configuration: H.264 baseline profile, CRF 23 (configurable), ultrfast preset
- Output: `EncodedFrame` with compressed H.264 NAL units in `data`
- Encoder holds ffmpeg `Encoder` and `Frame` objects (not Send — must run on single thread)
- Keyframe interval: every 2 seconds (configurable)

**Decoder (`ob-codec/src/decoder.rs`):**
- Replace `decode_h264` stub with real ffmpeg decoding
- Input: `EncodedFrame.data` = H.264 packet
- Output: `DecodedFrame` with BGRA pixels (ffmpeg auto-converts to AV_PIX_FMT_BGRA)
- Decoder holds ffmpeg `Decoder` and `Frame` objects

**New crate dependency:**
```toml
ffmpeg-next = "7.0"
```

**Build requirement:** ffmpeg libraries must be installed. On Windows: `pacman -S mingw-w64-x86_64-ffmpeg` in MSYS2.

#### 1.3 Frame Chunking

Wire the existing `FramePacket` (already in `ob-codec/src/frame_packet.rs`) into server and client.

**Server send flow:**
1. Capture screen → `CapturedFrame`
2. Encode with ffmpeg → compressed H.264 bytes (~20-200KB)
3. Build `WindowFrameHeader` → serialize to bytes
4. Prepend header bytes to encoded data → single frame blob
5. `FramePacket::fragment_frame(frame_blob)` → multiple packets (65KB each)
6. For each packet: `Message::new(MessageType::WindowFrame, packet.serialize())` → `udp.send_to()`

**Client receive flow:**
1. `run_receive_loop` delivers `WindowFrame` messages to channel
2. Client collects packets by `frame_number` into a `Vec<Option<FramePacket>>`
3. When all packets received (or timeout): `FramePacket::reassemble(packets)` → frame blob
4. Parse `WindowFrameHeader` from blob prefix
5. Extract encoded data → `VideoDecoder::decode_frame()` → `DecodedFrame`
6. Render to overlay

**Packet loss handling:**
- Maintain a `HashMap<u64, IncompleteFrame>` keyed by frame_number
- `IncompleteFrame` tracks: received packets, total expected, first seen timestamp
- If incomplete after 100ms: discard frame, request keyframe (set encoder's keyframe flag)
- Keyframe request: client sends `MessageType::Heartbeat` with a "request_keyframe" flag in the payload (reuse existing message type to avoid protocol changes)
- Server receives request → forces next encode to be keyframe

#### 1.4 Fix Client Socket Architecture

Remove the raw `recv_from` from client.rs. Both server and client use `run_receive_loop`.

**Changes to `src/main.rs`:**
- Always spawn `run_receive_loop` (for both primary and secondary)
- Pass `udp.message_channel()` sender to server/client

**Changes to `src/client.rs`:**
- Remove the raw `recv_from` spawn task
- Create a `mpsc::channel` for messages
- Spawn a task that reads from the message channel and handles WindowFrame + InputEvent
- WindowFrame: decode → send to overlay via channel
- InputEvent: inject via InputInjector

**Changes to `crates/ob-network/src/udp.rs`:**
- Add `tokio::sync::broadcast::Sender<(SocketAddr, Message)>` to UdpTransport
- `run_receive_loop` forwards all messages to broadcast (in addition to internal channel)
- Add `pub fn subscribe(&self) -> broadcast::Receiver<(SocketAddr, Message)>` method
- Server/client call `subscribe()` and filter by `msg_type`

### Files changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `ffmpeg-next` dependency |
| `ob-core/src/protocol.rs` | Add WindowFrameHeader struct |
| `ob-codec/src/encoder.rs` | Replace stub with ffmpeg encoder |
| `ob-codec/src/decoder.rs` | Replace stub with ffmpeg decoder |
| `ob-network/src/udp.rs` | Add broadcast channel, subscribe() method |
| `src/server.rs` | Binary protocol, FramePacket chunking, broadcast receive |
| `src/client.rs` | Binary protocol, FramePacket reassembly, broadcast receive |

---

## Sub-project 2: Complete Drag Flow

### Problem

The drag detection state machine has a gap (ButtonDown doesn't transition to MouseDown), and the WindowTransferManager is never wired into the main loops. The cross-device window drag feature doesn't work.

### Design

#### 2.1 Fix Drag Detection

**Changes to `ob-drag/src/detector.rs`:**

Fix the `ButtonDown` handler in `process_input`:
```rust
InputEvent::MouseButton { pressed: true, .. } => {
    // Query foreground window via GetForegroundWindow + GetWindowTextA
    let window = self.query_foreground_window();
    self.drag_state = DragState::MouseDown { pos: event.position, window };
}
```

Fix the `ButtonUp` handler:
```rust
InputEvent::MouseButton { pressed: false, .. } => {
    if let DragState::MouseDown { pos, window } = &self.drag_state {
        // Check if cursor moved enough to be a drag
        let dist = ((event.position.0 - pos.0).pow(2) + (event.position.1 - pos.1).pow(2) as f32).sqrt();
        if dist > self.drag_threshold as f32 {
            self.drag_state = DragState::Dragging {
                start_pos: *pos,
                current_pos: event.position,
                window: window.clone(),
            };
        } else {
            self.drag_state = DragState::Idle;
        }
    }
}
```

**New helper method:**
```rust
fn query_foreground_window(&self) -> Option<WindowInfo> {
    #[cfg(target_os = "windows")]
    {
        // GetForegroundWindow + GetWindowTextA + GetWindowRect
        // Return WindowInfo with title, position, size
    }
}
```

#### 2.2 Edge Crossing → Transfer Trigger

When `DragState` enters `EdgeCrossing`, initiate a cross-device transfer.

**Message flow:**

1. **Server detects edge crossing** (in input capture loop):
   - `WindowDragDetector::process_input()` returns `DragEvent::WindowCrossingEdge`
   - Server sends `WindowGrab` message to target device with window info

2. **Client receives WindowGrab:**
   - Creates overlay window at the target edge position
   - Sets overlay to semi-transparent (alpha=0.6)
   - Sends `WindowGrab` message back to server as acknowledgment

3. **Server starts window capture:**
   - Creates `WindowCapturer` for the specific window HWND
   - Starts encoding loop (same as video streaming, but for specific window)
   - Sends `WindowFrame` packets to the client

4. **Client renders frames to overlay:**
   - Decodes and renders each frame to the overlay window
   - Overlay follows cursor Y-position along the edge

5. **Mouse release detected:**
   - Server sends `WindowDrop` message to client
   - Server stops capture
   - Client makes overlay opaque (alpha=1.0)
   - Window is now "on" the target device

**New message types (already defined in protocol.rs):**
- `MessageType::WindowGrab` — payload: serialized `WindowGrabData` (window info, source device, position)
- `MessageType::WindowDrop` — payload: serialized `WindowDropData` (window ID)

**New types in `ob-core/src/protocol.rs`:**
```rust
#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct WindowDropData {
    pub window_id: String,
}
```

#### 2.3 Window Capture

Add `WindowCapturer` to `ob-capture` for capturing a specific window.

**Implementation:**
```rust
pub struct WindowCapturer {
    hwnd: *mut c_void,
    width: u32,
    height: u32,
}

impl WindowCapturer {
    pub fn new(hwnd: *mut c_void) -> Result<Self> {
        // Get window rect via GetWindowRect
        // Calculate width/height
    }

    pub fn capture_frame(&self) -> Result<CapturedFrame> {
        // Use PrintWindow or BitBlt with window DC
        // PrintWindow captures even occluded windows
        // Returns BGRA pixels
    }
}
```

**Changes to `ob-capture/src/lib.rs`:**
- Add `pub mod window;`
- Add `pub use window::WindowCapturer;`

**New file `ob-capture/src/window.rs`:**
- `WindowCapturer` struct with Win32 FFI
- `capture_frame()` using PrintWindow API
- Proper GDI resource cleanup

#### 2.4 Overlay Positioning

The overlay must appear at the correct edge position on the target device.

**Edge-to-position mapping:**
- Right edge of source → Left edge of target, Y = same relative position
- Left edge of source → Right edge of target, Y = same relative position
- Bottom edge of source → Top edge of target, X = same relative position
- Top edge of source → Bottom edge of target, X = same relative position

**Changes to `ob-display/src/overlay.rs`:**
- Add `set_alpha(&self, alpha: f32)` method using `SetLayeredWindowAttributes`
- Overlay created with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT`
- `WS_EX_TRANSPARENT` makes overlay click-through during drag

**Changes to `ob-layout/src/edge.rs`:**
- `translate_coordinates()` already handles position mapping — use it for overlay positioning

#### 2.5 Transfer State Machine

**New file `ob-drag/src/transfer.rs` (rewrite):**
```rust
pub enum TransferPhase {
    Idle,
    Grabbing,      // Waiting for client ack
    Streaming,     // Sending frames
    Dropping,      // Mouse released, waiting for cleanup
}

pub struct ActiveTransfer {
    pub window_id: String,
    pub source_device: DeviceId,
    pub target_device: DeviceId,
    pub capturer: WindowCapturer,
    pub encoder: VideoEncoder,
    pub phase: TransferPhase,
}
```

**Integration with server.rs:**
- Server holds `Option<ActiveTransfer>`
- When `WindowGrab` ack received: start capture loop
- When `WindowDrop` triggered: stop capture, clean up

### Files changed

| File | Change |
|------|--------|
| `ob-core/src/protocol.rs` | Add WindowGrabData, WindowDropData |
| `ob-drag/src/detector.rs` | Fix MouseDown/ButtonUp transitions |
| `ob-drag/src/transfer.rs` | Rewrite with proper state machine |
| `ob-capture/src/window.rs` | New: WindowCapturer |
| `ob-capture/src/lib.rs` | Export WindowCapturer |
| `ob-display/src/overlay.rs` | Add set_alpha(), WS_EX_LAYERED |
| `src/server.rs` | Handle WindowGrab/WindowDrop, drive transfer |
| `src/client.rs` | Handle WindowGrab, create positioned overlay |

---

## Sub-project 3: Harden & Scale

### Problem

Screen detection is approximate (divides virtual screen evenly), no DPI handling, duplicate types, no connection health monitoring, no graceful shutdown.

### Design

#### 3.1 Per-Monitor Detection

Replace `GetSystemMetrics` with `EnumDisplayMonitors` + `GetMonitorInfoW`.

**New file `ob-capture/src/monitor.rs`:**
```rust
pub struct MonitorInfo {
    pub handle: HMONITOR,
    pub name: String,
    pub rect: Rect,         // Physical pixels
    pub work_rect: Rect,    // Excluding taskbar
    pub dpi: f64,           // Per-monitor DPI (96 = 100%)
    pub is_primary: bool,
}

pub fn enumerate_monitors() -> Result<Vec<MonitorInfo>> {
    // EnumDisplayMonitors with callback
    // For each: GetMonitorInfoW for bounds, GetDpiForMonitor for DPI
}
```

**Changes to `ob-capture/src/screen.rs`:**
- Replace `detect_windows_screens()` with `enumerate_monitors()`
- Each `ScreenInfo` gets accurate bounds and DPI
- Remove duplicate function from `src/main.rs`

**Monitor hot-plug handling:**
- Spawn a hidden window that listens for `WM_DISPLAYCHANGE`
- When received: re-enumerate monitors, update screen info
- Send `MessageType::LayoutSync` to peers with updated screen topology

#### 3.2 DPI Handling

**Coordinate scaling:**
```rust
pub fn scale_coord(logical: i32, dpi: f64) -> i32 {
    (logical as f64 * dpi / 96.0) as i32
}

pub fn unscale_coord(physical: i32, dpi: f64) -> i32 {
    (physical as f64 * 96.0 / dpi) as i32
}
```

**Changes to `ob-layout/src/edge.rs`:**
- `edge_threshold` scaled by DPI: `threshold * dpi / 96`
- `detect_edge_crossing()` uses DPI-scaled screen bounds
- `translate_coordinates()` accounts for DPI differences between source and target

**Changes to `ob-display/src/overlay.rs`:**
- Overlay dimensions scaled by target DPI
- `StretchDIBits` target dimensions use physical pixels

**Changes to `ob-input/src/inject.rs`:**
- Input coordinates scaled from logical to physical before injection
- `SetCursorPos`, `mouse_event` use physical pixel coordinates

#### 3.3 Deduplication

**Move shared types:**
- `WindowFrameHeader` (from Sub-project 1) should be in `ob-core/src/protocol.rs` if not already
- `detect_windows_screens()` → `ob-capture/src/screen.rs` (remove from main.rs)
- Any remaining duplicate types between server.rs and client.rs → `ob-core/src/protocol.rs`

**Shared constants:**
- `MAX_PACKET_SIZE = 65000` → `ob-codec/src/frame_packet.rs` (already there)
- `HEARTBEAT_INTERVAL_MS = 5000` → `ob-core/src/protocol.rs`
- `CONNECTION_TIMEOUT_MS = 15000` → `ob-core/src/protocol.rs`

#### 3.4 Connection Management

**Heartbeat:**
- Server sends `MessageType::Heartbeat` every 5s to all connected clients
- Heartbeat payload: server timestamp (u64) for clock synchronization
- Client responds with `MessageType::Heartbeat` (echo)

**Timeout:**
- Each peer tracks `last_seen: Instant`
- If `last_seen.elapsed() > 15s`: remove peer from list
- Log warning when peer times out
- Client: if server times out, attempt reconnect (re-send Handshake)

**Reconnect:**
- Client tracks `connected_to: Option<SocketAddr>`
- If connection lost: enter "searching" state, re-broadcast discovery
- On finding server: re-send Handshake
- Exponential backoff: 1s, 2s, 4s, 8s, max 30s

**Changes to `ob-network/src/udp.rs`:**
- Add `last_seen` tracking per peer (already exists but unused)
- Add `is_peer_alive(addr) -> bool` method
- Add `cleanup_dead_peers()` method (called periodically)

**Changes to `src/server.rs`:**
- Spawn heartbeat task: `loop { sleep(5s); send Heartbeat to all peers }`
- In main loop: periodically call `cleanup_dead_peers()`

**Changes to `src/client.rs`:**
- In main loop: check if server is alive, reconnect if not
- Track reconnection state and backoff

#### 3.5 Error Handling

**Capture retry:**
- On capture failure: retry up to 3 times with 10ms delay
- After 3 failures: log error, skip frame, continue
- After 10 consecutive failures: pause capture for 1s, reset counter

**Graceful shutdown:**
- Use `tokio_util::sync::CancellationToken` for all spawned tasks
- On ctrl+c: signal cancellation, wait for tasks to finish (max 2s)
- Drop handlers clean up Win32 resources (window handles, GDI objects)

**Structured logging:**
- All errors include device ID and message type context
- Example: `warn!(device=%addr, msg_type=?msg.msg_type, "Failed to process message")`

#### 3.6 Fix def_window_proc

**Changes to `ob-display/src/overlay.rs`:**
```rust
extern "system" fn def_window_proc(
    hwnd: *mut c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    #[link(name = "user32")]
    extern "system" {
        fn DefWindowProcA(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn PostQuitMessage(exit_code: i32);
    }
    match msg {
        0x0002 => { // WM_DESTROY
            PostQuitMessage(0);
            0
        }
        _ => unsafe { DefWindowProcA(hwnd, msg, wparam, lparam) }
    }
}
```

### Files changed

| File | Change |
|------|--------|
| `ob-capture/src/monitor.rs` | New: per-monitor enumeration |
| `ob-capture/src/screen.rs` | Use enumerate_monitors, remove GetSystemMetrics |
| `ob-core/src/protocol.rs` | Add shared types, constants |
| `ob-layout/src/edge.rs` | DPI-scaled thresholds and coordinates |
| `ob-display/src/overlay.rs` | DPI-aware rendering, fix def_window_proc |
| `ob-input/src/inject.rs` | DPI-scaled coordinate injection |
| `ob-network/src/udp.rs` | Peer health tracking, cleanup |
| `src/main.rs` | Remove duplicate detect_windows_screens |
| `src/server.rs` | Heartbeat task, error recovery, graceful shutdown |
| `src/client.rs` | Reconnect logic, structured errors |

---

## Execution Order

```
Sub-project 1 (Fix Data Path)
  ├── 1.1 Binary protocol
  ├── 1.2 FFmpeg integration
  ├── 1.3 Frame chunking
  └── 1.4 Fix client socket
       │
       ▼
Sub-project 2 (Complete Drag Flow)
  ├── 2.1 Fix drag detection
  ├── 2.2 Edge crossing → transfer trigger
  ├── 2.3 Window capture
  ├── 2.4 Overlay positioning
  └── 2.5 Transfer state machine
       │
       ▼
Sub-project 3 (Harden & Scale)
  ├── 3.1 Per-monitor detection
  ├── 3.2 DPI handling
  ├── 3.3 Deduplication
  ├── 3.4 Connection management
  ├── 3.5 Error handling
  └── 3.6 Fix def_window_proc
       │
       ▼
Sub-project 4 (System Tray + GUI)
  ├── 4.1 System tray icon
  ├── 4.2 egui settings window
  └── 4.3 Integration
```

Each sub-project gets its own implementation plan. Sub-project 1 is the foundation — Sub-projects 2, 3, and 4 depend on it.

---

## Sub-project 4: System Tray + GUI

### Problem

OmniBridge is currently CLI-only. No visual indicator of connection status, no way to see discovered devices, no settings UI. Users must use terminal commands to interact with the app.

### Design

#### 4.1 System Tray

Use `tray-icon` crate for a persistent system tray icon.

**Tray icon states:**
- **Disconnected** (gray icon): No server found
- **Connected** (green icon): Connected to a peer
- **Error** (red icon): Connection lost or error

**Right-click menu:**
- Show/Hide settings window
- Connected devices (list with icons)
- Status (role, address, uptime)
- Start/Stop (toggle primary/secondary mode)
- Quit

**Implementation:**
- Spawn tray icon on a dedicated thread (tray-icon requires a message loop)
- Use `crossbeam-channel` for tray ↔ app communication
- On menu click: send event to main app thread

**New crate dependency:**
```toml
tray-icon = "0.19"
```

#### 4.2 egui Settings Window

Use `eframe` (egui framework) for a small settings/status window.

**Window contents:**
- Device name (editable)
- Role indicator (Primary/Secondary)
- Connected devices list with status
- Port configuration
- Layout configuration (screen positions)
- Connection health (latency, packet loss)
- Log viewer (scrollable, filtered)

**Window behavior:**
- Hidden by default (closed to tray)
- Show via tray menu or double-click tray icon
- Non-blocking — runs in a separate thread
- Scales to content, no fixed size

**Implementation:**
- `eframe::run_native()` in a dedicated thread
- Shared state via `Arc<RwLock<AppState>>` between tray, app, and GUI
- egui renders at 30fps (configurable)

**New crate dependency:**
```toml
eframe = "0.29"
```

#### 4.3 Integration

**Shared state structure:**
```rust
pub struct AppState {
    pub device_name: String,
    pub role: DeviceRole,
    pub status: AppStatus,
    pub connected_devices: Vec<DeviceInfo>,
    pub config: OmniBridgeConfig,
    pub logs: VecDeque<LogEntry>,
}
```

**Thread architecture:**
```
Main thread (tokio runtime)
  ├── Server/Client async tasks
  ├── Tray icon thread (crossbeam-channel)
  └── egui thread (Arc<RwLock<AppState>>)
```

**Lifecycle:**
1. App starts → creates tray icon + shared state
2. User clicks "Start" in tray menu → launches server/client on tokio runtime
3. Status updates flow from async tasks → shared state → tray icon + egui
4. User clicks "Quit" in tray menu → signals shutdown, waits for cleanup

**Changes to `src/main.rs`:**
- If `--gui` flag (or no flags): launch tray + egui instead of CLI mode
- CLI mode still works for headless/automation use
- Add `omnibridge gui` command to explicitly start GUI mode

**Changes to `Cargo.toml`:**
- Add `tray-icon` and `eframe` dependencies
- Make them optional features (default = ["gui"])

### Files changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add tray-icon, eframe deps; add "gui" feature |
| `ob-gui/src/lib.rs` | Rewrite with tray + egui integration |
| `ob-gui/src/app.rs` | Rewrite with shared AppState |
| `ob-gui/src/tray.rs` | New: system tray management |
| `ob-gui/src/settings.rs` | New: egui settings window |
| `ob-gui/src/state.rs` | New: shared AppState |
| `src/main.rs` | Add --gui flag, gui mode dispatch |
