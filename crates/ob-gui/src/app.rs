use eframe::egui;
use std::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct RemoteDevice {
    pub name: String,
    pub address: String,
    pub connected: bool,
}

pub struct OmniBridgeApp {
    device_name: String,
    is_primary: bool,
    status: AppStatus,
    remote_devices: Vec<RemoteDevice>,
    command_tx: Option<mpsc::Sender<AppCommand>>,
    event_rx: Option<mpsc::Receiver<AppEvent>>,
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    Start { name: String, is_primary: bool },
    Stop,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Status(AppStatus),
    DeviceFound(RemoteDevice),
}

impl OmniBridgeApp {
    pub fn new(
        command_tx: mpsc::Sender<AppCommand>,
        event_rx: mpsc::Receiver<AppEvent>,
    ) -> Self {
        Self {
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "Unknown PC".to_string()),
            is_primary: true,
            status: AppStatus::Stopped,
            remote_devices: Vec::new(),
            command_tx: Some(command_tx),
            event_rx: Some(event_rx),
        }
    }

    pub fn run_main_window(
        command_tx: mpsc::Sender<AppCommand>,
        event_rx: mpsc::Receiver<AppEvent>,
    ) -> eframe::Result {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([500.0, 400.0])
                .with_min_inner_size([400.0, 300.0])
                .with_title("OmniBridge"),
            ..Default::default()
        };

        eframe::run_native(
            "OmniBridge",
            native_options,
            Box::new(|_cc| Ok(Box::new(OmniBridgeApp::new(command_tx, event_rx)))),
        )
    }

    fn start_server(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(AppCommand::Start {
                name: self.device_name.clone(),
                is_primary: self.is_primary,
            });
        }
    }

    fn stop_server(&mut self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(AppCommand::Stop);
        }
    }
}

impl eframe::App for OmniBridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    AppEvent::Status(status) => self.status = status,
                    AppEvent::DeviceFound(device) => {
                        if !self.remote_devices.iter().any(|d| d.address == device.address) {
                            self.remote_devices.push(device);
                        }
                    }
                }
            }
        }

        ui.heading("OmniBridge");
        ui.separator();

        ui.group(|ui| {
            ui.label("This Device");
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.device_name);
            });
            ui.horizontal(|ui| {
                ui.label("Role:");
                if ui.selectable_label(self.is_primary, "Primary (Server)").clicked() {
                    self.is_primary = true;
                }
                if ui.selectable_label(!self.is_primary, "Secondary (Client)").clicked() {
                    self.is_primary = false;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Status:");
                let (color, text) = match &self.status {
                    AppStatus::Stopped => (egui::Color32::GRAY, "Stopped"),
                    AppStatus::Starting => (egui::Color32::YELLOW, "Starting..."),
                    AppStatus::Running => (egui::Color32::GREEN, "Running"),
                    AppStatus::Error(e) => (egui::Color32::RED, e.as_str()),
                };
                ui.colored_label(color, text);
            });
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.label("Remote Devices");
            if self.remote_devices.is_empty() {
                ui.label("No devices discovered");
            } else {
                for device in &mut self.remote_devices {
                    ui.horizontal(|ui| {
                        let color = if device.connected {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.colored_label(color, "●");
                        ui.label(&device.name);
                        ui.label(&device.address);
                        if device.connected {
                            if ui.button("Disconnect").clicked() {
                                device.connected = false;
                            }
                        } else if ui.button("Connect").clicked() {
                            device.connected = true;
                        }
                    });
                }
            }
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let running = self.status == AppStatus::Running || self.status == AppStatus::Starting;
            if running {
                if ui.button("Stop").clicked() {
                    self.stop_server();
                }
            } else if ui.button("Start").clicked() {
                self.start_server();
            }
        });
    }
}
