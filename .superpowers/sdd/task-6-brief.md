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

