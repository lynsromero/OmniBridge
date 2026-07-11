use ob_core::device::DeviceId;
use ob_core::window::WindowInfo;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DragState {
    pub transfers: HashMap<String, WindowTransferState>,
}

#[derive(Debug, Clone)]
pub struct WindowTransferState {
    pub window: WindowInfo,
    pub source_device: DeviceId,
    pub target_device: DeviceId,
    pub state: TransferPhase,
    pub progress: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferPhase {
    Initializing,
    Capturing,
    Encoding,
    Streaming,
    Complete,
    Error(String),
}

impl DragState {
    pub fn new() -> Self {
        Self {
            transfers: HashMap::new(),
        }
    }

    pub fn start_transfer(&mut self, window: WindowInfo, source: DeviceId, target: DeviceId) {
        let state = WindowTransferState {
            window: window.clone(),
            source_device: source,
            target_device: target,
            state: TransferPhase::Initializing,
            progress: 0.0,
        };
        self.transfers.insert(window.id.to_string(), state);
    }

    pub fn update_transfer(&mut self, window_id: &str, phase: TransferPhase, progress: f32) {
        if let Some(state) = self.transfers.get_mut(window_id) {
            state.state = phase;
            state.progress = progress;
        }
    }

    pub fn complete_transfer(&mut self, window_id: &str) {
        if let Some(state) = self.transfers.get_mut(window_id) {
            state.state = TransferPhase::Complete;
            state.progress = 1.0;
        }
    }

    pub fn remove_transfer(&mut self, window_id: &str) {
        self.transfers.remove(window_id);
    }
}

impl Default for DragState {
    fn default() -> Self {
        Self::new()
    }
}
