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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameFormat {
    H264,
    Raw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowFrameHeader {
    pub source_device: [u8; 16],
    pub width: u32,
    pub height: u32,
    pub timestamp_us: u64,
    pub is_keyframe: bool,
    pub format: FrameFormat,
}

impl WindowFrameHeader {
    pub const HEADER_SIZE: usize = 16 + 4 + 4 + 8 + 1 + 4;

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::HEADER_SIZE);
        buf.extend_from_slice(&self.source_device);
        buf.extend_from_slice(&self.width.to_le_bytes());
        buf.extend_from_slice(&self.height.to_le_bytes());
        buf.extend_from_slice(&self.timestamp_us.to_le_bytes());
        buf.push(self.is_keyframe as u8);
        buf.extend_from_slice(&(self.format as u32).to_le_bytes());
        buf
    }

    pub fn decode(data: &[u8]) -> anyhow::Result<Self> {
        if data.len() < Self::HEADER_SIZE {
            anyhow::bail!("WindowFrameHeader too short: {} < {}", data.len(), Self::HEADER_SIZE);
        }
        let mut source_device = [0u8; 16];
        source_device.copy_from_slice(&data[0..16]);
        let width = u32::from_le_bytes(data[16..20].try_into()?);
        let height = u32::from_le_bytes(data[20..24].try_into()?);
        let timestamp_us = u64::from_le_bytes(data[24..32].try_into()?);
        let is_keyframe = data[32] != 0;
        let format = match u32::from_le_bytes(data[33..37].try_into()?) {
            0 => FrameFormat::H264,
            1 => FrameFormat::Raw,
            _ => anyhow::bail!("Unknown frame format"),
        };
        Ok(Self { source_device, width, height, timestamp_us, is_keyframe, format })
    }

    pub fn frame_data<'a>(&self, payload: &'a [u8]) -> &'a [u8] {
        &payload[Self::HEADER_SIZE..]
    }

    pub fn from_message(msg: &Message) -> anyhow::Result<Self> {
        Self::decode(&msg.payload)
    }

    pub fn to_payload(&self, frame_data: &[u8]) -> Vec<u8> {
        let mut payload = self.encode();
        payload.extend_from_slice(frame_data);
        payload
    }
}
