use anyhow::Result;
use ob_core::event::{InputEvent, KeyEvent, MouseButton, MouseEvent, ScrollEvent};
use tracing::debug;

pub struct InputInjector {
    #[allow(dead_code)]
    device_id: ob_core::device::DeviceId,
}

impl InputInjector {
    pub fn new(device_id: ob_core::device::DeviceId) -> Self {
        Self { device_id }
    }

    pub fn inject(&self, event: &InputEvent) -> Result<()> {
        match event {
            InputEvent::Mouse(mouse) => self.inject_mouse(mouse),
            InputEvent::Key(key) => self.inject_key(key),
            InputEvent::Scroll(scroll) => self.inject_scroll(scroll),
        }
    }

    fn inject_mouse(&self, event: &MouseEvent) -> Result<()> {
        match event {
            MouseEvent::Move { x, y, .. } => {
                debug!("Injecting mouse move to ({}, {})", x, y);
                #[cfg(target_os = "windows")]
                unsafe {
                    set_cursor_pos(*x as i32, *y as i32);
                }
                Ok(())
            }
            MouseEvent::ButtonDown(btn) | MouseEvent::ButtonUp(btn) => {
                let is_down = matches!(event, MouseEvent::ButtonDown(_));
                debug!("Injecting mouse button {:?} down={}", btn, is_down);
                #[cfg(target_os = "windows")]
                unsafe {
                    let _vk = match btn {
                        MouseButton::Left => 1,
                        MouseButton::Right => 2,
                        MouseButton::Middle => 4,
                        MouseButton::Back => 5,
                        MouseButton::Forward => 6,
                        MouseButton::Other(v) => *v as i32,
                    };
                    let flags = if is_down { 0 } else { 2 }; // MOUSEEVENTF_UP
                    mouse_event(flags, 0, 0, 0, 0);
                }
                Ok(())
            }
            MouseEvent::DoubleClick(btn) => {
                self.inject_mouse(&MouseEvent::ButtonDown(btn.clone()))?;
                self.inject_mouse(&MouseEvent::ButtonUp(btn.clone()))?;
                self.inject_mouse(&MouseEvent::ButtonDown(btn.clone()))?;
                self.inject_mouse(&MouseEvent::ButtonUp(btn.clone()))?;
                Ok(())
            }
        }
    }

    fn inject_key(&self, event: &KeyEvent) -> Result<()> {
        debug!("Injecting key event: {:?} state={:?}", event.key, event.state);
        #[cfg(target_os = "windows")]
        unsafe {
            let flags: u8 = if event.state == ob_core::event::KeyState::Released { 2 } else { 0 };
            let mut scan = map_virtual_key(event.scancode, 0);
            if scan == 0 {
                scan = event.scancode;
            }
            keybd_event(flags, event.scancode as u8, scan as u32, 0);
        }
        Ok(())
    }

    fn inject_scroll(&self, event: &ScrollEvent) -> Result<()> {
        debug!("Injecting scroll: dx={}, dy={}", event.delta_x, event.delta_y);
        #[cfg(target_os = "windows")]
        unsafe {
            let _dx = (event.delta_x * 120.0) as i32;
            let dy = (event.delta_y * 120.0) as i32;
            mouse_event(0x0800, 0, 0, dy as i32, 0);
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn mouse_event(dwFlags: u32, dx: i32, dy: i32, dwData: i32, dwExtraInfo: usize);
    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    fn MapVirtualKeyA(uCode: u32, uMapType: u32) -> u32;
}

#[cfg(target_os = "windows")]
unsafe fn set_cursor_pos(x: i32, y: i32) {
    SetCursorPos(x, y);
}

#[cfg(target_os = "windows")]
fn map_virtual_key(code: u32, map_type: u32) -> u32 {
    unsafe { MapVirtualKeyA(code, map_type) }
}
