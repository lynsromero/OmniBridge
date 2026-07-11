use anyhow::Result;
use ob_codec::decoder::VideoDecoder;
use tracing::debug;

pub struct FrameRenderer {
    #[allow(dead_code)]
    decoder: VideoDecoder,
    frame_count: u64,
    last_render_time: std::time::Instant,
}

impl FrameRenderer {
    pub fn new() -> Self {
        Self {
            decoder: VideoDecoder::new(),
            frame_count: 0,
            last_render_time: std::time::Instant::now(),
        }
    }

    pub fn render(&mut self, _encoded_data: &[u8], _width: u32, _height: u32) -> Result<()> {
        self.frame_count += 1;

        let now = std::time::Instant::now();
        let fps = 1000.0 / self.last_render_time.elapsed().as_millis().max(1) as f64;
        self.last_render_time = now;

        if self.frame_count % 60 == 0 {
            debug!("Renderer: frame={}, fps={:.1}", self.frame_count, fps);
        }

        Ok(())
    }

    pub fn stats(&self) -> RenderStats {
        RenderStats {
            frames_rendered: self.frame_count,
            avg_fps: 0.0,
            avg_decode_time_us: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderStats {
    pub frames_rendered: u64,
    pub avg_fps: f64,
    pub avg_decode_time_us: u64,
}
