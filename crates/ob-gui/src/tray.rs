use std::sync::mpsc;
use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};

pub enum TrayCommand {
    ShowWindow,
    Quit,
}

pub struct SystemTray {
    tray_icon: Option<tray_icon::TrayIcon>,
    _command_rx: mpsc::Receiver<TrayCommand>,
}

impl SystemTray {
    pub fn new(_command_tx: mpsc::Sender<TrayCommand>) -> Self {
        let (_ignore, command_rx) = mpsc::channel();

        let tray_menu = Menu::new();
        let show_item = MenuItem::new("Show Window", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit_item = MenuItem::new("Quit", true, None);

        tray_menu.append(&show_item).unwrap();
        tray_menu.append(&separator).unwrap();
        tray_menu.append(&quit_item).unwrap();

        let icon = create_icon();

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("OmniBridge")
            .with_icon(icon)
            .build()
            .unwrap();

        Self {
            tray_icon: Some(tray_icon),
            _command_rx: command_rx,
        }
    }

    pub fn destroy(&mut self) {
        if let Some(tray) = self.tray_icon.take() {
            drop(tray);
        }
    }
}

fn create_icon() -> Icon {
    let size = 32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for _ in 0..size * size {
        rgba.push(0);
        rgba.push(150);
        rgba.push(255);
        rgba.push(255);
    }
    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}
