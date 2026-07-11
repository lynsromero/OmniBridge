use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedFrame {
    pub pixels: Vec<u8>,
    pub metadata: FrameMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: FrameFormat,
    pub timestamp_us: u64,
    pub capture_time_us: u64,
    pub frame_number: u64,
    pub dirty_regions: Vec<DirtyRegion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameFormat {
    BGRA,
    RGBA,
    NV12,
    I420,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirtyRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl CapturedFrame {
    pub fn size_bytes(&self) -> usize {
        self.pixels.len()
    }
}
