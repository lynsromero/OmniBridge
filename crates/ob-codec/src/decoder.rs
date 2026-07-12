use anyhow::Result;
use ffmpeg_next as ffmpeg;
use tracing::info;

use crate::encoder::EncodedFrame;

pub struct VideoDecoder {
    width: u32,
    height: u32,
    decoder: ffmpeg::codec::decoder::video::Video,
    frame_count: u64,
    last_keyframe: u64,
}

impl VideoDecoder {
    pub fn new(width: u32, height: u32) -> Self {
        ffmpeg::init().expect("Failed to initialize ffmpeg");

        let mut params = ffmpeg::codec::Parameters::new();
        unsafe {
            let raw = params.as_mut_ptr();
            (*raw).codec_type = ffmpeg::media::Type::Video.into();
            (*raw).codec_id = ffmpeg::codec::Id::H264.into();
            (*raw).width = width as i32;
            (*raw).height = height as i32;
        }

        let mut ctx = ffmpeg::decoder::new();
        ctx.set_parameters(params).expect("Failed to set decoder parameters");

        let decoder = ctx.video().expect("Failed to create H.264 video decoder");

        info!("Created ffmpeg H.264 decoder: {}x{}", width, height);

        Self {
            width,
            height,
            decoder,
            frame_count: 0,
            last_keyframe: 0,
        }
    }

    pub fn decode_frame(&mut self, encoded: &EncodedFrame) -> Result<DecodedFrame> {
        self.frame_count += 1;
        if encoded.is_keyframe {
            self.last_keyframe = encoded.frame_number;
        }

        let start = std::time::Instant::now();

        let packet = ffmpeg::packet::Packet::copy(&encoded.data);
        self.decoder.send_packet(&packet)?;

        let mut frame = ffmpeg::frame::Video::new(ffmpeg::format::Pixel::BGRA, self.width, self.height);
        self.decoder.receive_frame(&mut frame)?;

        let mut pixels = Vec::with_capacity((self.width * self.height * 4) as usize);
        unsafe {
            let frame_ptr = frame.as_ptr();
            for y in 0..self.height {
                for x in 0..self.width {
                    let idx = (y * frame.stride(0) as u32 + x * 4) as usize;
                    let data = (*frame_ptr).data[0];
                    pixels.push(*data.add(idx));
                    pixels.push(*data.add(idx + 1));
                    pixels.push(*data.add(idx + 2));
                    pixels.push(*data.add(idx + 3));
                }
            }
        }

        let decode_time = start.elapsed().as_micros() as u64;

        Ok(DecodedFrame {
            pixels,
            width: self.width,
            height: self.height,
            frame_number: encoded.frame_number,
            timestamp_us: encoded.timestamp_us,
            decode_time_us: decode_time,
        })
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
