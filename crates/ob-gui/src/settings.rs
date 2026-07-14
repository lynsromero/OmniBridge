use eframe::egui;
use std::sync::mpsc;

pub enum SettingsEvent {
    Close,
}

pub struct SettingsApp {
    device_name: String,
    is_primary: bool,
    status_text: String,
    connected_devices: Vec<String>,
    _close_tx: Option<mpsc::Sender<SettingsEvent>>,
}

impl SettingsApp {
    pub fn new(close_tx: mpsc::Sender<SettingsEvent>) -> Self {
        Self {
            device_name: "My Device".to_string(),
            is_primary: false,
            status_text: "Disconnected".to_string(),
            connected_devices: Vec::new(),
            _close_tx: Some(close_tx),
        }
    }

    pub fn set_status(&mut self, status: &str) {
        self.status_text = status.to_string();
    }

    pub fn set_devices(&mut self, devices: Vec<String>) {
        self.connected_devices = devices;
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("OmniBridge Settings");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Device Name:");
            ui.text_edit_singleline(&mut self.device_name);
        });

        ui.horizontal(|ui| {
            ui.label("Role:");
            ui.selectable_value(&mut self.is_primary, true, "Primary (Server)");
            ui.selectable_value(&mut self.is_primary, false, "Secondary (Client)");
        });

        ui.separator();
        ui.label(format!("Status: {}", self.status_text));

        ui.separator();
        ui.label("Connected Devices:");
        if self.connected_devices.is_empty() {
            ui.label("  No devices connected");
        } else {
            for device in &self.connected_devices {
                ui.label(format!("  {}", device));
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Start").clicked() {
                self.status_text = "Starting...".to_string();
            }
            if ui.button("Stop").clicked() {
                self.status_text = "Stopped".to_string();
            }
        });
    }
}

pub fn run_settings(app: SettingsApp) -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_title("OmniBridge"),
        ..Default::default()
    };

    eframe::run_native(
        "OmniBridge",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
