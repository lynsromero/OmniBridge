use anyhow::Result;
use ob_core::event::{InputEvent, KeyEvent, MouseEvent};
use tokio::sync::mpsc;
use tracing::{debug, info};

pub struct InputHook {
    event_tx: mpsc::Sender<InputEvent>,
    screen_edge_callback: Option<Box<dyn Fn(f64, f64) + Send + Sync>>,
}

impl InputHook {
    pub fn new(event_tx: mpsc::Sender<InputEvent>) -> Self {
        Self {
            event_tx,
            screen_edge_callback: None,
        }
    }

    pub fn set_screen_edge_handler<F: Fn(f64, f64) + Send + Sync + 'static>(&mut self, handler: F) {
        self.screen_edge_callback = Some(Box::new(handler));
    }

    pub async fn monitor_screen_edges(&self, screen_bounds: ob_core::screen::ScreenRect) -> Result<()> {
        info!("Monitoring screen edges for transitions");
        let tx = self.event_tx.clone();

        #[cfg(target_os = "windows")]
        {
            let bounds = screen_bounds;

            tokio::spawn(async move {
                #[link(name = "user32")]
                extern "system" {
                    fn GetCursorPos(lpPoint: *mut POINT) -> i32;
                }

                #[repr(C)]
                struct POINT {
                    x: i32,
                    y: i32,
                }

                let mut last_x: f64 = 0.0;
                let mut last_y: f64 = 0.0;

                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

                    unsafe {
                        let mut pt = POINT { x: 0, y: 0 };
                        if GetCursorPos(&mut pt) != 0 {
                            let x = pt.x as f64;
                            let y = pt.y as f64;

                            if x != last_x || y != last_y {
                                last_x = x;
                                last_y = y;

                                if !bounds.contains(x as i32, y as i32) {
                                    debug!("Cursor crossed screen edge at ({}, {})", x, y);
                                    let _ = tx.send(InputEvent::Mouse(MouseEvent::Move {
                                        x, y,
                                        screen_x: pt.x,
                                        screen_y: pt.y,
                                    })).await;
                                }
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }

    pub fn is_hotkey_combo(&self, event: &KeyEvent) -> bool {
        event.modifiers.ctrl && event.modifiers.alt && event.key == "O"
    }
}
