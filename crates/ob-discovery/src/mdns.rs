use anyhow::Result;
use ob_core::device::DeviceInfo;
use std::net::{SocketAddr, UdpSocket};

use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub struct DeviceDiscovery {
    device: DeviceInfo,
    broadcast_addr: SocketAddr,
    discovered_devices: Vec<(DeviceInfo, SocketAddr)>,
    #[allow(dead_code)]
    discovery_tx: mpsc::Sender<(DeviceInfo, SocketAddr)>,
}

impl DeviceDiscovery {
    pub fn new(device: DeviceInfo, port: u16) -> Self {
        let (tx, _rx) = mpsc::channel(64);
        Self {
            device,
            broadcast_addr: SocketAddr::new("255.255.255.255".parse().unwrap(), port),
            discovered_devices: Vec::new(),
            discovery_tx: tx,
        }
    }

    pub fn start_broadcast(&self) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;

        let device_json = serde_json::to_string(&self.device)?;
        let broadcast_msg = format!("OMNIBRIDGE_DISCOVER|{}", device_json);

        info!("Starting device broadcast on port {}", self.broadcast_addr.port());

        std::thread::spawn(move || {
            loop {
                if let Err(e) = socket.send_to(
                    broadcast_msg.as_bytes(),
                    "255.255.255.255:19810",
                ) {
                    warn!("Broadcast failed: {}", e);
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });

        Ok(())
    }

    pub fn start_listener(&self) -> Result<mpsc::Receiver<(DeviceInfo, SocketAddr)>> {
        let (tx, rx) = mpsc::channel(64);
        let port = self.broadcast_addr.port();
        let local_id = self.device.id;

        std::thread::spawn(move || {
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to bind discovery socket: {}", e);
                    return;
                }
            };

            socket.set_broadcast(true).ok();
            let mut buf = [0u8; 4096];

            info!("Discovery listener started on port {}", port);

            loop {
                match socket.recv_from(&mut buf) {
                    Ok((len, addr)) => {
                        let msg = String::from_utf8_lossy(&buf[..len]);
                        if msg.starts_with("OMNIBRIDGE_DISCOVER|") {
                            let json = &msg["OMNIBRIDGE_DISCOVER|".len()..];
                            match serde_json::from_str::<DeviceInfo>(json) {
                                Ok(device) => {
                                    if device.id != local_id {
                                        debug!("Discovered device: {} at {}", device.name, addr);
                                        let _ = tx.blocking_send((device, addr));
                                    }
                                }
                                Err(e) => {
                                    warn!("Invalid discovery message: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Discovery recv error: {}", e);
                    }
                }
            }
        });

        Ok(rx)
    }

    pub fn discovered_devices(&self) -> &[(DeviceInfo, SocketAddr)] {
        &self.discovered_devices
    }
}
