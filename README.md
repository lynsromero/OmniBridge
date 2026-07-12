# OmniBridge

Advanced cross-device KVM with seamless window dragging. Move your mouse across screen edges and drag windows between computers as if they were one unified desktop.

## Architecture

OmniBridge is a Rust workspace with 11 specialized crates:

| Crate | Purpose |
|-------|---------|
| `ob-core` | Shared types: devices, screens, events, protocols |
| `ob-network` | UDP streaming with broadcast channels + TCP-framed reliable transport |
| `ob-capture` | Screen and window capture via Win32 GDI |
| `ob-codec` | H.264 encoding/decoding via FFmpeg (libx264) |
| `ob-display` | Overlay windows and frame rendering |
| `ob-input` | Input capture and injection (keyboard, mouse, scroll) |
| `ob-layout` | Screen topology, edge detection, coordinate mapping |
| `ob-discovery` | mDNS device discovery and announcer |
| `ob-drag` | Window drag detection, edge-crossing, and transfer |
| `ob-cli` | Command-line interface (clap) |
| `ob-gui` | Native GUI (planned) |

### How It Works

1. **Discovery** - Devices broadcast via UDP on port 19810, auto-discovering peers on the LAN
2. **Edge Detection** - When the cursor hits a screen edge, OmniBridge checks for adjacent devices
3. **Window Drag** - Dragging a window to a screen edge triggers capture on the source device
4. **Encode** - Captured BGRA frames are converted to YUV420P and encoded to H.264 via FFmpeg (libx264 ultrafast)
5. **Stream** - Encoded H.264 packets are sent via UDP with a binary `WindowFrameHeader` (37 bytes)
6. **Decode** - The client decodes H.264 packets back to BGRA frames
7. **Display** - An overlay window on the target device renders the decoded frame in real-time
8. **Input Injection** - Keyboard and mouse events are forwarded and injected on the remote device

### Video Pipeline

```
Screen → BGRA capture → YUV420P conversion → H.264 encode (libx264)
    → UDP stream with WindowFrameHeader → H.264 decode → BGRA → Overlay
```

- Encoder: FFmpeg libx264, ultrafast preset, zerolatency tune, CRF 23
- Decoder: FFmpeg H.264 software decoder, BGRA output
- Frame format: Binary `WindowFrameHeader` (source device UUID, dimensions, timestamp, keyframe flag, format)
- Transport: UDP with broadcast channel for multi-consumer message delivery

## Quick Start

### Prerequisites

- Rust (stable, `x86_64-pc-windows-gnu` target)
- MSYS2 with MinGW-w64 toolchain:
  ```bash
  pacman -S mingw-w64-x86_64-binutils mingw-w64-x86_64-ffmpeg mingw-w64-x86_64-pkgconf
  ```
- Ensure `C:\msys64\mingw64\bin` is in your PATH for build

### Build

```bash
# Set MSYS2 environment
export PATH="C:\msys64\mingw64\bin:$PATH"
export PKG_CONFIG_PATH="C:\msys64\mingw64\lib\pkgconfig"

cargo build --release
```

### Run

```bash
# On the primary device (the one with keyboard/mouse):
omnibridge start --name "Desktop" --primary --port 19810

# On the secondary device:
omnibridge start --name "Laptop" --port 19810

# Or connect directly:
omnibridge connect --address 192.168.1.100 --port 19810
```

### Commands

```
omnibridge start --name <NAME> --port <PORT> --primary    # Start as primary node
omnibridge start --name <NAME> --port <PORT>              # Start as secondary node
omnibridge connect --address <IP> --port <PORT>           # Connect to a node
omnibridge status                                         # Show status
omnibridge layout show                                    # Show layout config
omnibridge layout set --from <DEV> --to <DEV> --direction <DIR>
omnibridge layout reset                                   # Reset layout
```

## Network Protocol

- **UDP (lossy)** - Video frame streaming with binary `WindowFrameHeader`, broadcast channel for multi-consumer delivery
- **TCP-framed (reliable)** - Control messages: handshake, input events, window transfer commands
- **Discovery** - UDP broadcast on port 19810 with JSON device info

## Configuration

Layout and configuration files are stored in:
- **Windows:** `%APPDATA%/omnibridge/`
- **Linux:** `~/.config/omnibridge/`
- **macOS:** `~/Library/Application Support/omnibridge/`

## License

MIT License. See [LICENSE](LICENSE) for details.
