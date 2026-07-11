use ob_core::device::DeviceId;
use ob_core::event::{InputEvent, MouseEvent};
use ob_core::screen::ScreenRect;
use ob_core::window::WindowInfo;
use ob_layout::edge::EdgeDetector;
use tracing::{debug, info};

pub struct WindowDragDetector {
    current_device: DeviceId,
    edge_detector: EdgeDetector,
    drag_state: DragState,
    drag_threshold: i32,
    #[allow(dead_code)]
    title_bar_height: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DragState {
    Idle,
    MouseDown {
        pos: (i32, i32),
        window: Option<WindowInfo>,
    },
    Dragging {
        start_pos: (i32, i32),
        current_pos: (i32, i32),
        window: WindowInfo,
    },
    EdgeCrossing {
        target_device: DeviceId,
        direction: ob_core::layout::Direction,
        window: WindowInfo,
        cursor_pos: (i32, i32),
    },
}

impl WindowDragDetector {
    pub fn new(current_device: DeviceId, screen_bounds: Vec<(DeviceId, ScreenRect)>) -> Self {
        let mut edge_detector = EdgeDetector::new(current_device, 5);
        edge_detector.update_screens(screen_bounds);

        Self {
            current_device,
            edge_detector,
            drag_state: DragState::Idle,
            drag_threshold: 5,
            title_bar_height: 32,
        }
    }

    pub fn process_input(&mut self, event: &InputEvent) -> Option<DragEvent> {
        match event {
            InputEvent::Mouse(MouseEvent::Move { screen_x, screen_y, .. }) => {
                self.handle_mouse_move(*screen_x, *screen_y)
            }
            InputEvent::Mouse(MouseEvent::ButtonDown(btn)) => {
                if *btn == ob_core::event::MouseButton::Left {
                    None
                } else {
                    None
                }
            }
            InputEvent::Mouse(MouseEvent::ButtonUp(btn)) => {
                if *btn == ob_core::event::MouseButton::Left {
                    self.handle_mouse_up()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_mouse_move(&mut self, screen_x: i32, screen_y: i32) -> Option<DragEvent> {
        let pos = (screen_x, screen_y);

        let current_state = self.drag_state.clone();

        match &current_state {
            DragState::Idle => {
                if let Some((target_device, direction)) = self.edge_detector.detect_edge_crossing(screen_x, screen_y) {
                    debug!("Cursor at edge: {:?} towards {:?}", direction, target_device);
                }
                None
            }
            DragState::MouseDown { pos: start_pos, window } => {
                let dx = (pos.0 - start_pos.0).abs();
                let dy = (pos.1 - start_pos.1).abs();

                if dx > self.drag_threshold || dy > self.drag_threshold {
                    if let Some(win) = window.clone() {
                        info!("Window drag started: {}", win.title);
                        self.drag_state = DragState::Dragging {
                            start_pos: *start_pos,
                            current_pos: pos,
                            window: win,
                        };
                    }
                }
                None
            }
            DragState::Dragging { start_pos, window, .. } => {
                let win = window.clone();
                let start = *start_pos;

                if let Some((target_device, direction)) = self.edge_detector.detect_edge_crossing(screen_x, screen_y) {
                    info!(
                        "Window '{}' crossing edge to device {:?} via {:?}",
                        win.title, target_device, direction
                    );
                    self.drag_state = DragState::EdgeCrossing {
                        target_device,
                        direction,
                        window: win.clone(),
                        cursor_pos: pos,
                    };

                    let target_pos = self.edge_detector.translate_coordinates(
                        screen_x, screen_y,
                        self.current_device,
                        target_device,
                        direction,
                    ).unwrap_or((0, 0));

                    return Some(DragEvent::WindowCrossingEdge {
                        window: win,
                        target_device,
                        direction,
                        target_position: target_pos,
                    });
                }

                self.drag_state = DragState::Dragging {
                    start_pos: start,
                    current_pos: pos,
                    window: win.clone(),
                };

                Some(DragEvent::WindowDragging {
                    window: win,
                    position: pos,
                })
            }
            DragState::EdgeCrossing { target_device, direction, window, .. } => {
                let target_pos = self.edge_detector.translate_coordinates(
                    screen_x, screen_y,
                    self.current_device,
                    *target_device,
                    *direction,
                ).unwrap_or((0, 0));

                Some(DragEvent::WindowCrossingEdge {
                    window: window.clone(),
                    target_device: *target_device,
                    direction: *direction,
                    target_position: target_pos,
                })
            }
        }
    }

    fn handle_mouse_up(&mut self) -> Option<DragEvent> {
        let current_state = self.drag_state.clone();
        match &current_state {
            DragState::Dragging { window, current_pos, .. } => {
                let event = DragEvent::WindowDropped {
                    window: window.clone(),
                    position: *current_pos,
                };
                self.drag_state = DragState::Idle;
                Some(event)
            }
            DragState::EdgeCrossing { window, .. } => {
                let event = DragEvent::WindowDropped {
                    window: window.clone(),
                    position: (0, 0),
                };
                self.drag_state = DragState::Idle;
                Some(event)
            }
            _ => {
                self.drag_state = DragState::Idle;
                None
            }
        }
    }

    pub fn start_drag(&mut self, pos: (i32, i32), window: WindowInfo) {
        self.drag_state = DragState::MouseDown {
            pos,
            window: Some(window),
        };
    }

    pub fn state(&self) -> &DragState {
        &self.drag_state
    }
}

#[derive(Debug, Clone)]
pub enum DragEvent {
    WindowDragging {
        window: WindowInfo,
        position: (i32, i32),
    },
    WindowCrossingEdge {
        window: WindowInfo,
        target_device: DeviceId,
        direction: ob_core::layout::Direction,
        target_position: (i32, i32),
    },
    WindowDropped {
        window: WindowInfo,
        position: (i32, i32),
    },
}
