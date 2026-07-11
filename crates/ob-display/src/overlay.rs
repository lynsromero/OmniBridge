use anyhow::Result;
use ob_core::device::DeviceId;
use ob_core::window::WindowInfo;
use ob_codec::decoder::DecodedFrame;
use tracing::{debug, info};

pub struct OverlayWindow {
    id: String,
    title: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    visible: bool,
    alpha: f32,
    #[allow(dead_code)]
    source_device: DeviceId,
}

impl OverlayWindow {
    pub fn new(title: &str, source_device: DeviceId) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        info!("Creating overlay window: {} from device {}", title, source_device);
        Self {
            id,
            title: title.to_string(),
            x: 0,
            y: 0,
            width: 400,
            height: 300,
            visible: true,
            alpha: 0.95,
            source_device,
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        debug!("Overlay '{}' moved to ({}, {})", self.title, x, y);
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha.clamp(0.0, 1.0);
    }

    pub fn render_frame(&self, frame: &DecodedFrame) -> Result<()> {
        if !self.visible {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            self.render_windows(frame)?;
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn render_windows(&self, _frame: &DecodedFrame) -> Result<()> {
        use std::ffi::c_void;

        #[link(name = "user32")]
        extern "system" {
            fn FindWindowA(lpClassName: *const u8, lpWindowName: *const u8) -> *mut c_void;
            fn SetWindowPos(hWnd: *mut c_void, hWndInsertAfter: *mut c_void,
                           x: i32, y: i32, cx: i32, cy: i32, uFlags: u32) -> i32;
            fn SetLayeredWindowAttributes(hwnd: *mut c_void, crKey: u32,
                                         bAlpha: u8, dwFlags: u32) -> i32;
        }

        let class_name = format!("OmniBridgeOverlay\0");
        let hwnd = unsafe {
            FindWindowA(class_name.as_ptr() as *const u8, std::ptr::null())
        };

        if !hwnd.is_null() {
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    self.x,
                    self.y,
                    self.width as i32,
                    self.height as i32,
                    0x0040 | 0x0002,
                );
                SetLayeredWindowAttributes(hwnd, 0, (self.alpha * 255.0) as u8, 0x02);
            }
        }

        Ok(())
    }

    pub fn update_from_window(&mut self, window: &WindowInfo) {
        self.x = window.x;
        self.y = window.y;
        self.width = window.width;
        self.height = window.height;
        self.title = window.title.clone();
    }

    pub fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}
