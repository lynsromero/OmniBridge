use crate::tray::{SystemTray, TrayStatus, TrayCommand};
use ob_core::device::{DeviceInfo, DeviceRole};
use std::sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}};
use tracing::info;

pub struct OmniBridgeApp {
    pub device_name: String,
    pub is_primary: bool,
    pub connected_devices: Vec<DeviceInfo>,
    pub status: AppStatus,
    tray: Option<SystemTray>,
    is_settings_open: Arc<AtomicBool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl AppStatus {
    pub fn to_tray_status(&self) -> TrayStatus {
        match self {
            AppStatus::Disconnected => TrayStatus::Disconnected,
            AppStatus::Connecting => TrayStatus::Connected,
            AppStatus::Connected => TrayStatus::Connected,
            AppStatus::Error(_) => TrayStatus::Error,
        }
    }
}

impl OmniBridgeApp {
    pub fn new(device_name: String, is_primary: bool) -> Self {
        info!("Creating OmniBridge app: {} (primary={})", device_name, is_primary);
        Self {
            device_name,
            is_primary,
            connected_devices: Vec::new(),
            status: AppStatus::Disconnected,
            tray: None,
            is_settings_open: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_tray(mut self) -> Self {
        self.tray = Some(SystemTray::new());
        self
    }

    pub fn add_device(&mut self, device: DeviceInfo) {
        self.connected_devices.push(device);
        info!("Device added: {}", self.connected_devices.last().unwrap().name);
        self.update_tray();
    }

    pub fn remove_device(&mut self, device_id: ob_core::device::DeviceId) {
        self.connected_devices.retain(|d| d.id != device_id);
        self.update_tray();
    }

    pub fn set_status(&mut self, status: AppStatus) {
        self.status = status;
        self.update_tray();
    }

    fn update_tray(&mut self) {
        if let Some(ref mut tray) = self.tray {
            tray.set_status(self.status.to_tray_status());
        }
    }

    pub fn poll_tray_commands(&mut self) -> Option<TrayCommand> {
        self.tray.as_ref().and_then(|tray| tray.poll_command())
    }

    pub fn open_settings(&mut self) {
        if self.is_settings_open.load(Ordering::SeqCst) {
            return;
        }

        self.is_settings_open.store(true, Ordering::SeqCst);
        let (close_tx, close_rx) = mpsc::channel();
        let app = crate::settings::SettingsApp::new(close_tx);

        std::thread::spawn(move || {
            let _ = crate::settings::run_settings(app);
        });

        let is_open = self.is_settings_open.clone();
        std::thread::spawn(move || {
            loop {
                if close_rx.try_recv().is_ok() {
                    is_open.store(false, Ordering::SeqCst);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
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
