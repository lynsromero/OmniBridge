use anyhow::Result;
use ob_codec::decoder::VideoDecoder;
use ob_core::device::DeviceInfo;
use ob_core::protocol::{FrameFormat, Message, MessageType, WindowFrameHeader};
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

    let (overlay_tx, mut overlay_rx) = mpsc::channel::<ob_codec::decoder::DecodedFrame>(32);

    let mut decoder: Option<VideoDecoder> = None;
    let mut overlay: Option<OverlayWindow> = None;

    let mut broadcast_rx = udp.subscribe();
    let injector_device_id = device.id;
    tokio::spawn(async move {
        let injector = InputInjector::new(injector_device_id);
        loop {
            match broadcast_rx.recv().await {
                Ok((_addr, msg)) => {
                    match msg.msg_type {
                        MessageType::WindowFrame => {
                            if let Ok(header) = WindowFrameHeader::decode(&msg.payload) {
                                let frame_data = header.frame_data(&msg.payload);

                                if decoder.is_none() {
                                    decoder = Some(VideoDecoder::new(header.width, header.height));
                                }

                                if let Some(ref mut dec) = decoder {
                                    let encoded = ob_codec::encoder::EncodedFrame {
                                        data: frame_data.to_vec(),
                                        width: header.width,
                                        height: header.height,
                                        frame_number: msg.sequence,
                                        is_keyframe: header.is_keyframe,
                                        timestamp_us: header.timestamp_us,
                                        encode_time_us: 0,
                                        format: match header.format {
                                            FrameFormat::H264 => ob_codec::encoder::EncodedFormat::H264,
                                            _ => ob_codec::encoder::EncodedFormat::H264,
                                        },
                                    };
                                    match dec.decode_frame(&encoded) {
                                        Ok(decoded) => {
                                            let _ = overlay_tx.send(decoded).await;
                                        }
                                        Err(e) => {
                                            warn!("Decode failed: {}", e);
                                        }
                                    }
                                }
                            }
                        }
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
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Skipped {} messages (receiver lagged)", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    warn!("Broadcast channel closed");
                    break;
                }
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
