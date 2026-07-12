use anyhow::Result;
use ffmpeg_next as ffmpeg;
use ob_capture::frame::CapturedFrame;
use tracing::{debug, info};

#[allow(dead_code)]
pub struct VideoEncoder {
    width: u32,
    height: u32,
    target_fps: u32,
    bitrate: u32,
    frame_count: u64,
    encoder: ffmpeg::codec::encoder::video::Encoder,
    frame: ffmpeg::frame::Video,
}

impl VideoEncoder {
    pub fn new(width: u32, height: u32, target_fps: u32) -> Self {
        ffmpeg::init().expect("Failed to initialize ffmpeg");

        let mut ctx = ffmpeg::encoder::new().video().expect("Failed to create video encoder context");

        ctx.set_width(width);
        ctx.set_height(height);
        ctx.set_format(ffmpeg::format::Pixel::YUV420P);
        ctx.set_time_base(ffmpeg::Rational::new(1, target_fps as i32));
        ctx.set_max_b_frames(0);
        ctx.set_bit_rate(width as usize * height as usize * 3 / 10);

        let mut options = ffmpeg::Dictionary::new();
        options.set("preset", "ultrafast");
        options.set("tune", "zerolatency");
        options.set("crf", "23");

        let encoder = ctx.open_with(options).expect("Failed to open encoder");

        let frame = ffmpeg::frame::Video::new(ffmpeg::format::Pixel::YUV420P, width, height);

        info!(
            "Created ffmpeg H.264 encoder: {}x{} @ {}fps",
            width, height, target_fps
        );

        Self {
            width,
            height,
            target_fps,
            bitrate: width * height * 3 / 10,
            frame_count: 0,
            encoder,
            frame,
        }
    }

    pub fn set_bitrate(&mut self, bitrate: u32) {
        self.bitrate = bitrate;
        debug!("Bitrate set to {} bps", bitrate);
    }

    pub fn encode_frame(&mut self, frame: &CapturedFrame) -> Result<EncodedFrame> {
        self.frame_count += 1;
        let start = std::time::Instant::now();

        self.convert_bgra_to_yuv420p(&frame.pixels, frame.metadata.width, frame.metadata.height);
        self.frame.set_pts(Some(self.frame_count as i64));

        self.encoder.send_frame(&self.frame)?;

        let mut encoded_data = Vec::new();
        let mut packet = ffmpeg::packet::Packet::empty();
        while self.encoder.receive_packet(&mut packet).is_ok() {
            if let Some(data) = packet.data() {
                encoded_data.extend_from_slice(data);
            }
            packet = ffmpeg::packet::Packet::empty();
        }

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

    fn convert_bgra_to_yuv420p(&mut self, bgra: &[u8], width: u32, height: u32) {
        let y_stride = self.frame.stride(0);
        let u_stride = self.frame.stride(1);
        let v_stride = self.frame.stride(2);

        unsafe {
            let frame_ptr = self.frame.as_mut_ptr();
            let y_ptr = (*frame_ptr).data[0] as *mut u8;
            let u_ptr = (*frame_ptr).data[1] as *mut u8;
            let v_ptr = (*frame_ptr).data[2] as *mut u8;

            for y in 0..height {
                for x in 0..width {
                    let idx = ((y * width + x) * 4) as usize;
                    let b = bgra[idx] as f32;
                    let g = bgra[idx + 1] as f32;
                    let r = bgra[idx + 2] as f32;

                    let y_val = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
                    let y_pos = (y * y_stride as u32 + x) as usize;
                    *y_ptr.add(y_pos) = y_val;

                    if y % 2 == 0 && x % 2 == 0 {
                        let u_val =
                            (-0.169 * r - 0.331 * g + 0.500 * b + 128.0).clamp(0.0, 255.0)
                                as u8;
                        let v_val =
                            (0.500 * r - 0.419 * g - 0.081 * b + 128.0).clamp(0.0, 255.0)
                                as u8;
                        let u_pos = ((y / 2) * u_stride as u32 + (x / 2)) as usize;
                        let v_pos = ((y / 2) * v_stride as u32 + (x / 2)) as usize;
                        *u_ptr.add(u_pos) = u_val;
                        *v_ptr.add(v_pos) = v_val;
                    }
                }
            }
        }
    }

    pub fn force_keyframe(&mut self) {
        self.frame_count = 0;
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EncodedFormat {
    H264,
    H265,
    AV1,
    VP9,
}
