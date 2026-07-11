use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub Uuid);

impl Default for WindowId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: WindowId,
    pub handle: u64,
    pub title: String,
    pub process_name: String,
    pub process_id: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub is_focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
    Idle,
    Dragging {
        source_device: crate::device::DeviceId,
        target_device: crate::device::DeviceId,
    },
    Streaming {
        source_device: crate::device::DeviceId,
        target_device: crate::device::DeviceId,
    },
    Dropped {
        target_device: crate::device::DeviceId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowTransferRequest {
    pub window: WindowInfo,
    pub source_device: crate::device::DeviceId,
    pub target_device: crate::device::DeviceId,
    pub source_position: (i32, i32),
    pub timestamp: u64,
}
