# OmniBridge

Advanced cross-device KVM with seamless window dragging. Move your mouse across screen edges and drag windows between computers as if they were one unified desktop.

## Architecture

OmniBridge is a Rust workspace with 11 specialized crates:

| Crate | Purpose |
|-------|---------|
| `ob-core` | Shared types: devices, screens, events, protocols |
| `ob-network` | UDP streaming + TCP-framed reliable transport |
| `ob-capture` | Screen and window capture via Win32 GDI |
| `ob-codec` | H.264 encoding/decoding (software + GPU detection) |
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
4. **Stream** - The window content is encoded and streamed via UDP to the target device
5. **Display** - An overlay window on the target device renders the stream in real-time
6. **Input Injection** - Keyboard and mouse events are forwarded and injected on the remote device

## Quick Start

### Prerequisites

- Rust (stable, `x86_64-pc-windows-gnu` target)
- MinGW-w64 (installed via MSYS2: `pacman -S mingw-w64-x86_64-binutils`)

### Build

```bash
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

- **UDP (lossy)** - Video frame streaming, high throughput, low latency
- **TCP-framed (reliable)** - Control messages: handshake, input events, window transfer commands
- **Discovery** - UDP broadcast on port 19810 with JSON device info

## Configuration

Layout and configuration files are stored in:
- **Windows:** `%APPDATA%/omnibridge/`
- **Linux:** `~/.config/omnibridge/`
- **macOS:** `~/Library/Application Support/omnibridge/`

## License

MIT License. See [LICENSE](LICENSE) for details.
