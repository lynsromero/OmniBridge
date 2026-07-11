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
