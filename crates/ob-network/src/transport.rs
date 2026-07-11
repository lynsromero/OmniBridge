use anyhow::Result;
use ob_core::protocol::Message;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use super::udp::UdpTransport;

pub struct TransportLayer {
    pub udp: Arc<UdpTransport>,
    pub control_tx: mpsc::Sender<(SocketAddr, Message)>,
}

impl TransportLayer {
    pub async fn new(bind_addr: SocketAddr) -> Result<Self> {
        let udp = Arc::new(UdpTransport::bind(bind_addr).await?);
        let control_tx = udp.message_channel();

        Ok(Self { udp, control_tx })
    }

    pub async fn send_control(&self, msg: &Message, addr: SocketAddr) -> Result<()> {
        self.udp.send_to(msg, addr).await
    }

    pub async fn broadcast_control(&self, msg: &Message) -> Result<()> {
        self.udp.send_to_all_peers(msg).await
    }

    pub fn control_receiver(&self) -> mpsc::Sender<(SocketAddr, Message)> {
        self.udp.message_channel()
    }
}
