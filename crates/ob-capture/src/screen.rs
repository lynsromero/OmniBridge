use anyhow::Result;
use crate::frame::{CapturedFrame, FrameMetadata, FrameFormat};
use ob_core::screen::ScreenInfo;
use tracing::info;

pub struct ScreenCapturer {
    screen: ScreenInfo,
    running: bool,
    frame_count: u64,
}

impl ScreenCapturer {
    pub fn new(screen: ScreenInfo) -> Self {
        Self { screen, running: false, frame_count: 0 }
    }

    pub fn start(&mut self) -> Result<()> {
        info!("Starting screen capture for screen: {}", self.screen.name);
        self.running = true;
        Ok(())
    }

    pub fn capture_frame(&mut self) -> Result<Option<CapturedFrame>> {
        if !self.running {
            return Ok(None);
        }

        self.frame_count += 1;

        #[cfg(target_os = "windows")]
        {
            return self.capture_windows_frame();
        }

        #[cfg(target_os = "linux")]
        {
            return self.capture_linux_frame();
        }

        #[cfg(target_os = "macos")]
        {
            return self.capture_macos_frame();
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Ok(None)
        }
    }

    #[cfg(target_os = "windows")]
    fn capture_windows_frame(&mut self) -> Result<Option<CapturedFrame>> {
        use std::time::Instant;

        let start = Instant::now();
        let width = self.screen.width;
        let height = self.screen.height;
        let pixel_count = (width * height) as usize;
        let mut pixels = vec![0u8; pixel_count * 4];

        unsafe {
            self.capture_desktop_dxgi(&mut pixels, width, height)?;
        }

        let metadata = FrameMetadata {
            width,
            height,
            stride: width * 4,
            format: FrameFormat::BGRA,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            capture_time_us: start.elapsed().as_micros() as u64,
            frame_number: self.frame_count,
            dirty_regions: Vec::new(),
        };

        Ok(Some(CapturedFrame { pixels, metadata }))
    }

    #[cfg(target_os = "windows")]
    unsafe fn capture_desktop_dxgi(
        &self,
        pixels: &mut [u8],
        width: u32,
        height: u32,
    ) -> Result<()> {
        use std::ffi::c_void;

        #[link(name = "user32")]
        extern "system" {
            fn GetDesktopWindow() -> *mut c_void;
            fn GetDC(hWnd: *mut c_void) -> *mut c_void;
            fn ReleaseDC(hWnd: *mut c_void, hDC: *mut c_void) -> i32;
        }

        #[link(name = "gdi32")]
        extern "system" {
            fn CreateCompatibleDC(hdc: *mut c_void) -> *mut c_void;
            fn CreateCompatibleBitmap(hdc: *mut c_void, cx: i32, cy: i32) -> *mut c_void;
            fn SelectObject(hdc: *mut c_void, h: *mut c_void) -> *mut c_void;
            fn BitBlt(hdc: *mut c_void, x: i32, y: i32, cx: i32, cy: i32,
                       hdcSrc: *mut c_void, x1: i32, y1: i32, rop: u32) -> i32;
            fn GetDIBits(hdc: *mut c_void, hbm: *mut c_void, start: u32, cLines: u32,
                         lpvBits: *mut u8, lpbmi: *mut BITMAPINFOHEADER, usage: u32) -> i32;
            fn DeleteObject(ho: *mut c_void) -> i32;
            fn DeleteDC(hdc: *mut c_void) -> i32;
        }

        #[repr(C)]
        #[allow(non_snake_case)]
        struct BITMAPINFOHEADER {
            biSize: u32,
            biWidth: i32,
            biHeight: i32,
            biPlanes: u16,
            biBitCount: u16,
            biCompression: u32,
            biSizeImage: u32,
            biXPelsPerMeter: i32,
            biYPelsPerMeter: i32,
            biClrUsed: u32,
            biClrImportant: u32,
        }

        let _hwnd = GetDesktopWindow();
        let hdc_screen = GetDC(std::ptr::null_mut());
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbmp = CreateCompatibleBitmap(hdc_screen, width as i32, height as i32);
        let old_bmp = SelectObject(hdc_mem, hbmp);

        BitBlt(hdc_mem, 0, 0, width as i32, height as i32, hdc_screen, self.screen.x, self.screen.y, 0x00CC0020);

        let mut bmi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        GetDIBits(hdc_mem, hbmp, 0, height, pixels.as_mut_ptr(), &mut bmi, 0);

        SelectObject(hdc_mem, old_bmp);
        DeleteObject(hbmp as *mut c_void);
        DeleteDC(hdc_mem);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn capture_linux_frame(&mut self) -> Result<Option<CapturedFrame>> {
        debug!("Linux screen capture stub (implement with XShm/PipeWire)");
        Ok(None)
    }

    #[cfg(target_os = "macos")]
    fn capture_macos_frame(&mut self) -> Result<Option<CapturedFrame>> {
        debug!("macOS screen capture stub (implement with CoreGraphics)");
        Ok(None)
    }

    pub fn stop(&mut self) {
        self.running = false;
        info!("Screen capture stopped after {} frames", self.frame_count);
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}
