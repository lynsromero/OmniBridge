use ob_core::device::{DeviceId, DeviceInfo};
use ob_core::protocol::Message;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tracing::info;

pub struct ConnectionManager {
    local_device: DeviceInfo,
    peers: HashMap<DeviceId, PeerConnection>,
    message_tx: mpsc::Sender<(DeviceId, Message)>,
}

pub struct PeerConnection {
    device: DeviceInfo,
    addr: SocketAddr,
    connected: bool,
}

impl ConnectionManager {
    pub fn new(local_device: DeviceInfo) -> Self {
        let (tx, _rx) = mpsc::channel(1024);
        Self {
            local_device,
            peers: HashMap::new(),
            message_tx: tx,
        }
    }

    pub fn add_peer(&mut self, device: DeviceInfo, addr: SocketAddr) {
        info!("Adding peer: {} at {}", device.name, addr);
        self.peers.insert(device.id, PeerConnection {
            device,
            addr,
            connected: true,
        });
    }

    pub fn remove_peer(&mut self, device_id: &DeviceId) {
        self.peers.remove(device_id);
        info!("Removed peer: {}", device_id);
    }

    pub fn get_peer_addr(&self, device_id: &DeviceId) -> Option<SocketAddr> {
        self.peers.get(device_id).map(|p| p.addr)
    }

    pub fn connected_peers(&self) -> Vec<&DeviceInfo> {
        self.peers.values().filter(|p| p.connected).map(|p| &p.device).collect()
    }

    pub fn all_peers(&self) -> &HashMap<DeviceId, PeerConnection> {
        &self.peers
    }

    pub fn local_device(&self) -> &DeviceInfo {
        &self.local_device
    }

    pub fn message_channel(&self) -> mpsc::Sender<(DeviceId, Message)> {
        self.message_tx.clone()
    }
}
