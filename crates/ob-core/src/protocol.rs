use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    Handshake,
    HandshakeAck,
    InputEvent,
    ClipboardSync,
    WindowGrab,
    WindowDrop,
    WindowFrame,
    LayoutSync,
    Heartbeat,
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub sequence: u64,
}

impl Message {
    pub fn new(msg_type: MessageType, payload: Vec<u8>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self { msg_type, payload, timestamp, sequence: 0 }
    }

    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.sequence = seq;
        self
    }

    pub fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(self.msg_type as u32).to_le_bytes());
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        buf.extend_from_slice(&(self.payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.payload);
        Ok(buf)
    }

    pub fn deserialize(data: &[u8]) -> anyhow::Result<Self> {
        if data.len() < 20 {
            anyhow::bail!("Message too short");
        }
        let msg_type = match u32::from_le_bytes(data[0..4].try_into()?) {
            0 => MessageType::Handshake,
            1 => MessageType::HandshakeAck,
            2 => MessageType::InputEvent,
            3 => MessageType::ClipboardSync,
            4 => MessageType::WindowGrab,
            5 => MessageType::WindowDrop,
            6 => MessageType::WindowFrame,
            7 => MessageType::LayoutSync,
            8 => MessageType::Heartbeat,
            9 => MessageType::Disconnect,
            _ => anyhow::bail!("Unknown message type"),
        };
        let timestamp = u64::from_le_bytes(data[4..12].try_into()?);
        let sequence = u64::from_le_bytes(data[12..20].try_into()?);
        let payload_len = u32::from_le_bytes(data[20..24].try_into()?) as usize;
        if data.len() < 24 + payload_len {
            anyhow::bail!("Payload truncated");
        }
        let payload = data[24..24 + payload_len].to_vec();
        Ok(Self { msg_type, payload, timestamp, sequence })
    }
}
