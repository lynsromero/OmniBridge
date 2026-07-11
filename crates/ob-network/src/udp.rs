use anyhow::Result;
use bytes::{BufMut, BytesMut};
use ob_core::protocol::Message;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    peers: Arc<RwLock<HashMap<SocketAddr, PeerInfo>>>,
    message_tx: mpsc::Sender<(SocketAddr, Message)>,
    #[allow(dead_code)]
    message_rx: Arc<RwLock<mpsc::Receiver<(SocketAddr, Message)>>>,
    buffer_size: usize,
}

struct PeerInfo {
    addr: SocketAddr,
    #[allow(dead_code)]
    last_seen: std::time::Instant,
}

impl UdpTransport {
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let socket = Arc::new(UdpSocket::bind(addr).await?);
        info!("UDP transport bound to {}", addr);

        let (tx, rx) = mpsc::channel(1024);

        Ok(Self {
            socket,
            peers: Arc::new(RwLock::new(HashMap::new())),
            message_tx: tx,
            message_rx: Arc::new(RwLock::new(rx)),
            buffer_size: 65536,
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    pub async fn send_to(&self, msg: &Message, addr: SocketAddr) -> Result<()> {
        let data = msg.serialize()?;
        let mut buf = BytesMut::with_capacity(4 + data.len());
        buf.put_u32_le(data.len() as u32);
        buf.extend_from_slice(&data);
        self.socket.send_to(&buf, addr).await?;
        Ok(())
    }

    pub async fn send_to_all_peers(&self, msg: &Message) -> Result<()> {
        let peers = self.peers.read().await;
        for peer in peers.values() {
            if let Err(e) = self.send_to(msg, peer.addr).await {
                warn!("Failed to send to {}: {}", peer.addr, e);
            }
        }
        Ok(())
    }

    pub fn message_channel(&self) -> mpsc::Sender<(SocketAddr, Message)> {
        self.message_tx.clone()
    }

    pub async fn run_receive_loop(self: Arc<Self>) -> Result<()> {
        let mut buf = vec![0u8; self.buffer_size];
        loop {
            let (len, addr) = self.socket.recv_from(&mut buf).await?;
            if len < 4 {
                warn!("Received undersized packet from {}", addr);
                continue;
            }

            let packet_len = u32::from_le_bytes(buf[0..4].try_into()?) as usize;
            if len < 4 + packet_len {
                warn!("Truncated packet from {}", addr);
                continue;
            }

            match Message::deserialize(&buf[4..4 + packet_len]) {
                Ok(msg) => {
                    debug!("Received {:?} from {} (seq={})", msg.msg_type, addr, msg.sequence);

                    {
                        let mut peers = self.peers.write().await;
                        peers.insert(addr, PeerInfo {
                            addr,
                            last_seen: std::time::Instant::now(),
                        });
                    }

                    if let Err(e) = self.message_tx.send((addr, msg)).await {
                        error!("Failed to forward message: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Failed to deserialize message from {}: {}", addr, e);
                }
            }
        }
    }

    pub async fn add_peer(&self, addr: SocketAddr) {
        let mut peers = self.peers.write().await;
        peers.insert(addr, PeerInfo {
            addr,
            last_seen: std::time::Instant::now(),
        });
        info!("Added peer: {}", addr);
    }

    pub async fn remove_peer(&self, addr: &SocketAddr) {
        let mut peers = self.peers.write().await;
        peers.remove(addr);
        info!("Removed peer: {}", addr);
    }

    pub async fn peer_count(&self) -> usize {
        self.peers.read().await.len()
    }
}
