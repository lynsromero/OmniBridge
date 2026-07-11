use anyhow::Result;
use bytes::{BufMut, BytesMut};
use ob_core::protocol::Message;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::info;

pub struct QuicTransport {
    #[allow(dead_code)]
    listener: Option<TcpListener>,
    connections: Vec<TcpStream>,
    message_tx: mpsc::Sender<(SocketAddr, Message)>,
}

impl QuicTransport {
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        info!("QUIC-like transport (TCP framed) listening on {}", addr);
        let (tx, _rx) = mpsc::channel(1024);
        Ok(Self {
            listener: Some(listener),
            connections: Vec::new(),
            message_tx: tx,
        })
    }

    pub async fn connect(&mut self, addr: SocketAddr) -> Result<()> {
        let stream = TcpStream::connect(addr).await?;
        info!("Connected to peer at {}", addr);
        self.connections.push(stream);
        Ok(())
    }

    pub async fn send(&mut self, msg: &Message) -> Result<()> {
        let data = msg.serialize()?;
        for stream in &mut self.connections {
            let mut buf = BytesMut::with_capacity(4 + data.len());
            buf.put_u32_le(data.len() as u32);
            buf.extend_from_slice(&data);
            stream.write_all(&buf).await?;
        }
        Ok(())
    }

    pub fn message_channel(&self) -> mpsc::Sender<(SocketAddr, Message)> {
        self.message_tx.clone()
    }
}
