use ob_core::device::{DeviceInfo, DeviceRole};
use tracing::info;

pub struct OmniBridgeApp {
    pub device_name: String,
    pub is_primary: bool,
    pub connected_devices: Vec<DeviceInfo>,
    pub status: AppStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl OmniBridgeApp {
    pub fn new(device_name: String, is_primary: bool) -> Self {
        info!("Creating OmniBridge app: {} (primary={})", device_name, is_primary);
        Self {
            device_name,
            is_primary,
            connected_devices: Vec::new(),
            status: AppStatus::Disconnected,
        }
    }

    pub fn add_device(&mut self, device: DeviceInfo) {
        self.connected_devices.push(device);
        info!("Device added: {}", self.connected_devices.last().unwrap().name);
    }

    pub fn remove_device(&mut self, device_id: ob_core::device::DeviceId) {
        self.connected_devices.retain(|d| d.id != device_id);
    }

    pub fn set_status(&mut self, status: AppStatus) {
        self.status = status;
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        output.push_str("====================================\n");
        output.push_str("         OmniBridge v0.1.0\n");
        output.push_str("====================================\n");
        output.push_str(&format!("Device: {}\n", self.device_name));
        output.push_str(&format!("Role: {}\n", if self.is_primary { "Primary" } else { "Secondary" }));
        output.push_str(&format!("Status: {:?}\n", self.status));
        output.push_str("------------------------------------\n");

        if self.connected_devices.is_empty() {
            output.push_str("No devices connected\n");
        } else {
            for device in &self.connected_devices {
                output.push_str(&format!("  -> {} ({})\n", device.name, device.role_str()));
            }
        }

        output.push_str("====================================\n");
        output
    }
}

trait DeviceInfoExt {
    fn role_str(&self) -> &str;
}

impl DeviceInfoExt for DeviceInfo {
    fn role_str(&self) -> &str {
        match self.role {
            DeviceRole::Primary => "Primary",
            DeviceRole::Secondary => "Secondary",
        }
    }
}
