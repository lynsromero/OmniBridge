use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub Uuid);

impl DeviceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceRole {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub role: DeviceRole,
    pub host: String,
    pub quic_port: u16,
    pub udp_port: u16,
    pub screens: Vec<super::screen::ScreenInfo>,
    pub capabilities: DeviceCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub has_gpu_encoder: bool,
    pub has_gpu_decoder: bool,
    pub max_capture_fps: u32,
    pub supported_codecs: Vec<String>,
    pub cpu_cores: u32,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            has_gpu_encoder: false,
            has_gpu_decoder: false,
            max_capture_fps: 60,
            supported_codecs: vec!["h264".to_string()],
            cpu_cores: num_cpus(),
        }
    }
}

fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}
