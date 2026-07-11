use anyhow::Result;
use ob_core::device::DeviceInfo;
use std::net::UdpSocket;
use tracing::{info, warn};

pub struct DeviceAnnouncer {
    device: DeviceInfo,
    broadcast_port: u16,
}

impl DeviceAnnouncer {
    pub fn new(device: DeviceInfo, broadcast_port: u16) -> Self {
        Self { device, broadcast_port }
    }

    pub fn announce(&self) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;

        let device_json = serde_json::to_string(&self.device)?;
        let msg = format!("OMNIBRIDGE_DISCOVER|{}", device_json);

        info!(
            "Announcing device '{}' on port {}",
            self.device.name,
            self.broadcast_port
        );

        socket.send_to(
            msg.as_bytes(),
            format!("255.255.255.255:{}", self.broadcast_port),
        )?;

        Ok(())
    }

    pub fn announce_loop(&self) -> Result<()> {
        loop {
            if let Err(e) = self.announce() {
                warn!("Announce failed: {}", e);
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }
}
