use serde::{Deserialize, Serialize};
use crate::encoder::EncodedFrame;

pub const MAX_PACKET_SIZE: usize = 65000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePacket {
    pub header: FramePacketHeader,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePacketHeader {
    pub frame_number: u64,
    pub packet_index: u16,
    pub total_packets: u16,
    pub is_keyframe: bool,
    pub width: u32,
    pub height: u32,
    pub timestamp_us: u64,
    pub original_size: u32,
}

impl FramePacket {
    pub fn fragment_frame(frame: &EncodedFrame) -> Vec<Self> {
        let max_data = MAX_PACKET_SIZE - 64;
        let total_packets = ((frame.data.len() + max_data - 1) / max_data).max(1) as u16;

        let mut packets = Vec::with_capacity(total_packets as usize);

        for i in 0..total_packets {
            let start = i as usize * max_data;
            let end = ((i + 1) as usize * max_data).min(frame.data.len());
            let chunk = frame.data[start..end].to_vec();

            packets.push(FramePacket {
                header: FramePacketHeader {
                    frame_number: frame.frame_number,
                    packet_index: i,
                    total_packets,
                    is_keyframe: frame.is_keyframe,
                    width: frame.width,
                    height: frame.height,
                    timestamp_us: frame.timestamp_us,
                    original_size: frame.data.len() as u32,
                },
                data: chunk,
            });
        }

        packets
    }

    pub fn reassemble(packets: &mut Vec<Option<FramePacket>>) -> Option<EncodedFrame> {
        if packets.is_empty() {
            return None;
        }

        let first = packets.first()?.as_ref()?;
        let total = first.header.total_packets as usize;

        if packets.len() < total {
            return None;
        }

        for p in packets.iter().take(total) {
            if p.is_none() {
                return None;
            }
        }

        let mut data = Vec::with_capacity(first.header.original_size as usize);
        for i in 0..total {
            if let Some(packet) = &packets[i] {
                data.extend_from_slice(&packet.data);
            }
        }

        Some(EncodedFrame {
            data,
            width: first.header.width,
            height: first.header.height,
            frame_number: first.header.frame_number,
            is_keyframe: first.header.is_keyframe,
            timestamp_us: first.header.timestamp_us,
            encode_time_us: 0,
            format: crate::encoder::EncodedFormat::H264,
        })
    }
}
