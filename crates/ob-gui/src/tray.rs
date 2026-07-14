use std::sync::mpsc;
use tracing::info;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayStatus {
    Disconnected,
    Connected,
    Error,
}

pub enum TrayCommand {
    ShowSettings,
    Quit,
}

pub struct SystemTray {
    tray_icon: Option<tray_icon::TrayIcon>,
    _show_item: MenuItem,
    _quit_item: MenuItem,
    command_rx: mpsc::Receiver<TrayCommand>,
    command_tx: mpsc::Sender<TrayCommand>,
}

impl SystemTray {
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel();

        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Show Settings", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit", true, None);

        tray_menu.append(&show_item).unwrap();
        tray_menu.append(&separator).unwrap();
        tray_menu.append(&quit_item).unwrap();

        let icon = create_icon(TrayStatus::Disconnected);

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("OmniBridge - Disconnected")
            .with_icon(icon)
            .build()
            .unwrap();

        info!("System tray created");

        Self {
            tray_icon: Some(tray_icon),
            _show_item: show_item,
            _quit_item: quit_item,
            command_rx,
            command_tx,
        }
    }

    pub fn command_tx(&self) -> mpsc::Sender<TrayCommand> {
        self.command_tx.clone()
    }

    pub fn poll_command(&self) -> Option<TrayCommand> {
        self.command_rx.try_recv().ok()
    }

    pub fn set_status(&mut self, status: TrayStatus) {
        if let Some(ref mut tray) = self.tray_icon {
            let icon = create_icon(status);
            let _ = tray.set_icon(Some(icon));

            let tooltip = match status {
                TrayStatus::Disconnected => "OmniBridge - Disconnected",
                TrayStatus::Connected => "OmniBridge - Connected",
                TrayStatus::Error => "OmniBridge - Error",
            };
            let _ = tray.set_tooltip(Some(tooltip));
        }
    }

    pub fn destroy(&mut self) {
        if let Some(tray) = self.tray_icon.take() {
            drop(tray);
        }
    }
}

fn create_icon(status: TrayStatus) -> Icon {
    let (r, g, b) = match status {
        TrayStatus::Disconnected => (128, 128, 128),
        TrayStatus::Connected => (0, 200, 0),
        TrayStatus::Error => (200, 0, 0),
    };

    let size = 32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..size * size {
        rgba.push(r);
        rgba.push(g);
        rgba.push(b);
        rgba.push(255);
    }

    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

pub fn poll_tray_events() {
    if let Ok(event) = TrayIconEvent::receiver().try_recv() {
        match event {
            TrayIconEvent::Click { .. } => {}
            _ => {}
        }
    }
}
