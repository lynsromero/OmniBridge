use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEvent {
    Move {
        x: f64,
        y: f64,
        screen_x: i32,
        screen_y: i32,
    },
    ButtonDown(MouseButton),
    ButtonUp(MouseButton),
    DoubleClick(MouseButton),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEvent {
    pub scancode: u32,
    pub key: String,
    pub modifiers: KeyModifiers,
    pub state: KeyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
    pub caps_lock: bool,
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self { ctrl: false, alt: false, shift: false, super_key: false, caps_lock: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollEvent {
    pub delta_x: f64,
    pub delta_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Mouse(MouseEvent),
    Key(KeyEvent),
    Scroll(ScrollEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardData {
    pub content_type: ClipboardContentType,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardContentType {
    Text,
    Html,
    Image,
    Files,
}
