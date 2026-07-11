use anyhow::Result;
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

    let connected_clients: Arc<RwLock<Vec<(DeviceInfo, SocketAddr)>>> = Arc::new(RwLock::new(Vec::new()));

    let (input_tx, mut input_rx) = mpsc::channel::<ob_core::event::InputEvent>(256);

    let mut input_capture = InputCapture::new(input_tx);

    #[cfg(target_os = "windows")]
    {
        input_capture.start().await?;
        println!("Input capture active - move mouse to screen edges to switch devices");
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

    loop {
        tokio::select! {
            Some((new_device, addr)) = discovery_rx.recv() => {
                let already_connected = connected_clients.read().await.iter().any(|(d, _)| d.id == new_device.id);
                if !already_connected {
                    println!("Device connected: {} ({})", new_device.name, addr);

                    let handshake_ack = Message::new(
                        MessageType::HandshakeAck,
                        serde_json::to_vec(&device)?,
                    );
                    udp.send_to(&handshake_ack, addr).await?;

                    connected_clients.write().await.push((new_device, addr));
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                // Periodic tasks
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down server...");
                break;
            }
        }
    }

    Ok(())
}
