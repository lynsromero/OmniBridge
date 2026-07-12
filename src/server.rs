use anyhow::Result;
use ob_capture::ScreenCapturer;
use ob_codec::encoder::VideoEncoder;
use ob_core::device::DeviceInfo;
use ob_core::protocol::{FrameFormat, Message, MessageType, WindowFrameHeader};
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

                            let header = WindowFrameHeader {
                                source_device: *device_id.0.as_bytes(),
                                width: encoded.width,
                                height: encoded.height,
                                timestamp_us: encoded.timestamp_us,
                                is_keyframe: encoded.is_keyframe,
                                format: FrameFormat::H264,
                            };

                            let payload = header.to_payload(&encoded.data);

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
