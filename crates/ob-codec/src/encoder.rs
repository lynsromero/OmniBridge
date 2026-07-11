use anyhow::Result;
use ob_capture::frame::CapturedFrame;
use tracing::{debug, info};

pub struct VideoEncoder {
    width: u32,
    height: u32,
    target_fps: u32,
    bitrate: u32,
    frame_count: u64,
    encoder_type: EncoderType,
}

#[derive(Debug, Clone, Copy)]
pub enum EncoderType {
    SoftwareH264,
    NvencH264,
    QsvH264,
    AmfH264,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32, target_fps: u32) -> Self {
        let encoder_type = detect_best_encoder();
        info!(
            "Creating video encoder: {}x{} @ {}fps using {:?}",
            width, height, target_fps, encoder_type
        );

        Self {
            width,
            height,
            target_fps,
            bitrate: width * height * 3 / 10,
            frame_count: 0,
            encoder_type,
        }
    }

    pub fn set_bitrate(&mut self, bitrate: u32) {
        self.bitrate = bitrate;
        debug!("Bitrate set to {} bps", bitrate);
    }

    pub fn encode_frame(&mut self, frame: &CapturedFrame) -> Result<EncodedFrame> {
        self.frame_count += 1;
        let start = std::time::Instant::now();

        let encoded_data = match self.encoder_type {
            EncoderType::SoftwareH264 => self.encode_software_h264(frame)?,
            _ => self.encode_software_h264(frame)?,
        };

        let encode_time = start.elapsed().as_micros() as u64;

        Ok(EncodedFrame {
            data: encoded_data,
            width: frame.metadata.width,
            height: frame.metadata.height,
            frame_number: self.frame_count,
            is_keyframe: self.frame_count % (self.target_fps as u64 * 2) == 0,
            timestamp_us: frame.metadata.timestamp_us,
            encode_time_us: encode_time,
            format: EncodedFormat::H264,
        })
    }

    fn encode_software_h264(&self, frame: &CapturedFrame) -> Result<Vec<u8>> {
        let mut output = Vec::with_capacity(20 + frame.pixels.len());

        output.extend_from_slice(&frame.metadata.width.to_le_bytes());
        output.extend_from_slice(&frame.metadata.height.to_le_bytes());
        output.extend_from_slice(&frame.metadata.timestamp_us.to_le_bytes());
        output.extend_from_slice(&(frame.pixels.len() as u32).to_le_bytes());

        output.extend_from_slice(&frame.pixels);

        Ok(output)
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

fn detect_best_encoder() -> EncoderType {
    #[cfg(target_os = "windows")]
    {
        if check_nvenc_available() {
            return EncoderType::NvencH264;
        }
        if check_qsv_available() {
            return EncoderType::QsvH264;
        }
    }
    EncoderType::SoftwareH264
}

#[cfg(target_os = "windows")]
fn check_nvenc_available() -> bool {
    let paths = [
        "C:\\Windows\\System32\\nvEncodeAPI64.dll",
        "C:\\Windows\\System32\\nvencapi.dll",
    ];
    paths.iter().any(|p| std::path::Path::new(p).exists())
}

#[cfg(target_os = "windows")]
fn check_qsv_available() -> bool {
    let paths = [
        "C:\\Windows\\System32\\libmfxhw64.dll",
        "C:\\Windows\\System32\\IntelQuickSyncVideo.dll",
    ];
    paths.iter().any(|p| std::path::Path::new(p).exists())
}

#[cfg(not(target_os = "windows"))]
fn check_nvenc_available() -> bool { false }

#[cfg(not(target_os = "windows"))]
fn check_qsv_available() -> bool { false }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub frame_number: u64,
    pub is_keyframe: bool,
    pub timestamp_us: u64,
    pub encode_time_us: u64,
    pub format: EncodedFormat,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncodedFormat {
    H264,
    H265,
    AV1,
    VP9,
}
