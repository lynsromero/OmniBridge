use anyhow::Result;
use tracing::info;

use crate::encoder::EncodedFrame;

pub struct VideoDecoder {
    frame_count: u64,
    last_keyframe: u64,
}

impl VideoDecoder {
    pub fn new() -> Self {
        info!("Creating video decoder");
        Self { frame_count: 0, last_keyframe: 0 }
    }

    pub fn decode_frame(&mut self, encoded: &EncodedFrame) -> Result<DecodedFrame> {
        self.frame_count += 1;
        if encoded.is_keyframe {
            self.last_keyframe = encoded.frame_number;
        }

        let start = std::time::Instant::now();

        let pixels = match encoded.format {
            crate::encoder::EncodedFormat::H264 => self.decode_h264(encoded)?,
            _ => self.decode_h264(encoded)?,
        };

        let decode_time = start.elapsed().as_micros() as u64;

        Ok(DecodedFrame {
            pixels,
            width: encoded.width,
            height: encoded.height,
            frame_number: encoded.frame_number,
            timestamp_us: encoded.timestamp_us,
            decode_time_us: decode_time,
        })
    }

    fn decode_h264(&self, encoded: &EncodedFrame) -> Result<Vec<u8>> {
        if encoded.data.len() < 13 {
            return Err(anyhow::anyhow!("Encoded frame too small"));
        }

        let width = u32::from_le_bytes(encoded.data[0..4].try_into()?);
        let height = u32::from_le_bytes(encoded.data[4..8].try_into()?);
        let _timestamp = u64::from_le_bytes(encoded.data[8..16].try_into()?);

        let pixel_count = (width * height) as usize;
        let mut pixels = vec![0u8; pixel_count * 4];

        let _quality = *encoded.data.get(16).unwrap_or(&3);
        let skip = ((width as usize * height as usize) / (encoded.data.len() - 17).max(1)).max(1);

        let mut i = 0;
        let mut pi = 0;
        while i < encoded.data.len() - 17 && pi < pixel_count * 4 {
            let val = encoded.data[17 + i];
            pixels[pi] = val;
            pixels[pi + 1] = val;
            pixels[pi + 2] = val;
            pixels[pi + 3] = 255;
            pi += 4 * skip;
            i += 1;
        }

        Ok(pixels)
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

#[derive(Debug, Clone)]
pub struct DecodedFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub frame_number: u64,
    pub timestamp_us: u64,
    pub decode_time_us: u64,
}
