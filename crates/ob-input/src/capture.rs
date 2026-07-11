use anyhow::Result;
use ob_core::event::InputEvent;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct InputCapture {
    event_tx: mpsc::Sender<InputEvent>,
    running: bool,
}

impl InputCapture {
    pub fn new(event_tx: mpsc::Sender<InputEvent>) -> Self {
        Self { event_tx, running: false }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting input capture");
        self.running = true;

        #[cfg(target_os = "windows")]
        {
            self.start_windows_capture().await?;
        }

        #[cfg(target_os = "linux")]
        {
            self.start_linux_capture().await?;
        }

        #[cfg(target_os = "macos")]
        {
            self.start_macos_capture().await?;
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn start_windows_capture(&self) -> Result<()> {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let tx = self.event_tx.clone();
        let running = Arc::new(Mutex::new(self.running));

        tokio::spawn(async move {
            info!("Windows input capture loop started");
            let mut last_x: f64 = 0.0;
            let mut last_y: f64 = 0.0;

            loop {
                if !*running.lock().await {
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_millis(1)).await;

                if let Some(event) = poll_windows_input(&mut last_x, &mut last_y) {
                    if let Err(e) = tx.send(event).await {
                        warn!("Failed to send input event: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn start_linux_capture(&self) -> Result<()> {
        info!("Linux input capture stub - implement with evdev/libinput");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    async fn start_macos_capture(&self) -> Result<()> {
        info!("macOS input capture stub - implement with CoreGraphics");
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running = false;
        info!("Input capture stopped");
    }
}

#[cfg(target_os = "windows")]
fn poll_windows_input(last_x: &mut f64, last_y: &mut f64) -> Option<InputEvent> {
    use ob_core::event::{MouseButton, MouseEvent};

    #[link(name = "user32")]
    extern "system" {
        fn GetCursorPos(lpPoint: *mut POINT) -> i32;
        fn GetAsyncKeyState(vKey: i32) -> i16;
    }

    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }

    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt) != 0 {
            let x = pt.x as f64;
            let y = pt.y as f64;

            if (x - *last_x).abs() > 0.5 || (y - *last_y).abs() > 0.5 {
                *last_x = x;
                *last_y = y;
                return Some(InputEvent::Mouse(MouseEvent::Move {
                    x,
                    y,
                    screen_x: pt.x,
                    screen_y: pt.y,
                }));
            }
        }

        for vk in [1, 2, 4, 5, 6] {
            let state = GetAsyncKeyState(vk);
            if (state & 0x8000u16 as i16) != 0 {
                let btn = match vk {
                    1 => MouseButton::Left,
                    2 => MouseButton::Right,
                    4 => MouseButton::Middle,
                    5 => MouseButton::Back,
                    6 => MouseButton::Forward,
                    _ => MouseButton::Other(vk as u8),
                };
                return Some(InputEvent::Mouse(MouseEvent::ButtonDown(btn)));
            }
        }
    }

    None
}
