mod server;
mod client;
mod config;

use anyhow::Result;
use clap::Parser;
use ob_cli::Cli;
use ob_core::device::{DeviceCapabilities, DeviceInfo, DeviceId, DeviceRole};
use ob_core::screen::{ScreenId, ScreenInfo};
use ob_layout::config::LayoutConfigManager;
use ob_discovery::announcer::DeviceAnnouncer;
use ob_discovery::mdns::DeviceDiscovery;
use tracing::info;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "omnibridge=info,ob_*=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        ob_cli::Commands::Start { name, port, primary } => {
            start_node(&name, port, primary).await?;
        }
        ob_cli::Commands::Connect { address, port } => {
            connect_to_node(&address, port).await?;
        }
        ob_cli::Commands::Status => {
            show_status()?;
        }
        ob_cli::Commands::Layout { action } => {
            handle_layout_command(action)?;
        }
        ob_cli::Commands::Config { action } => {
            handle_config_command(action)?;
        }
    }

    Ok(())
}

async fn start_node(name: &str, port: u16, is_primary: bool) -> Result<()> {
    info!("Starting OmniBridge node: {} (port: {}, primary: {})", name, port, is_primary);

    let device_id = DeviceId::new();
    let role = if is_primary { DeviceRole::Primary } else { DeviceRole::Secondary };

    let local_device = DeviceInfo {
        id: device_id,
        name: name.to_string(),
        role,
        host: "0.0.0.0".to_string(),
        quic_port: port,
        udp_port: port + 1,
        screens: detect_screens()?,
        capabilities: DeviceCapabilities::default(),
    };

    println!("Starting OmniBridge node: {}", name);
    println!("  Role: {}", if is_primary { "Primary" } else { "Secondary" });
    println!("  QUIC Port: {}", port);
    println!("  UDP Port: {}", port + 1);

    let announcer = DeviceAnnouncer::new(local_device.clone(), port);
    let discovery = DeviceDiscovery::new(local_device.clone(), port);

    println!("Announcing device on network...");

    let listener_rx = discovery.start_listener()?;

    std::thread::spawn(move || {
        if let Err(e) = announcer.announce_loop() {
            tracing::error!("Announcer error: {}", e);
        }
    });

    let udp_addr = format!("0.0.0.0:{}", port + 1).parse()?;
    let udp_transport = ob_network::udp::UdpTransport::bind(udp_addr).await?;
    let udp_transport = std::sync::Arc::new(udp_transport);

    if is_primary {
        let udp_clone = udp_transport.clone();
        tokio::spawn(async move {
            if let Err(e) = udp_clone.run_receive_loop().await {
                tracing::error!("UDP receive loop error: {}", e);
            }
        });
    }

    if is_primary {
        server::run_server(local_device, listener_rx, udp_transport).await?;
    } else {
        client::run_client(local_device, listener_rx, udp_transport).await?;
    }

    Ok(())
}

async fn connect_to_node(address: &str, port: u16) -> Result<()> {
    info!("Connecting to node at {}:{}", address, port);

    let device_id = DeviceId::new();

    let local_device = DeviceInfo {
        id: device_id,
        name: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        role: DeviceRole::Secondary,
        host: "0.0.0.0".to_string(),
        quic_port: port,
        udp_port: port + 1,
        screens: detect_screens()?,
        capabilities: DeviceCapabilities::default(),
    };

    println!("Connecting to {}:{}...", address, port);

    let udp_addr = format!("0.0.0.0:{}", port + 1).parse()?;
    let udp_transport = ob_network::udp::UdpTransport::bind(udp_addr).await?;
    let udp_transport = std::sync::Arc::new(udp_transport);

    let target_addr: std::net::SocketAddr = format!("{}:{}", address, port + 1).parse()?;
    udp_transport.add_peer(target_addr).await;

    let udp_clone = udp_transport.clone();
    tokio::spawn(async move {
        if let Err(e) = udp_clone.run_receive_loop().await {
            tracing::error!("UDP receive loop error: {}", e);
        }
    });

    let handshake = ob_core::protocol::Message::new(
        ob_core::protocol::MessageType::Handshake,
        serde_json::to_vec(&local_device)?,
    );

    udp_transport.send_to(&handshake, target_addr).await?;

    println!("Connected! Waiting for configuration...");

    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");

    Ok(())
}

fn detect_screens() -> Result<Vec<ScreenInfo>> {
    #[cfg(target_os = "windows")]
    {
        return detect_windows_screens();
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(vec![ScreenInfo {
            id: ScreenId(0),
            name: "Display 1".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            scale_factor: 1.0,
            is_primary: true,
        }])
    }
}

#[cfg(target_os = "windows")]
fn detect_windows_screens() -> Result<Vec<ScreenInfo>> {
    #[link(name = "user32")]
    extern "system" {
        fn GetSystemMetrics(nIndex: i32) -> i32;
    }

    const SM_CMONITORS: i32 = 80;
    const SM_XVIRTUALSCREEN: i32 = 76;
    const SM_YVIRTUALSCREEN: i32 = 77;
    const SM_CXVIRTUALSCREEN: i32 = 78;
    const SM_CYVIRTUALSCREEN: i32 = 79;

    let num_monitors = unsafe { GetSystemMetrics(SM_CMONITORS) };
    let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    info!("Detected {} monitors, virtual screen: {}x{} at ({}, {})", num_monitors, vw, vh, vx, vy);

    let mut screens = Vec::new();
    for i in 0..num_monitors.max(1) {
        screens.push(ScreenInfo {
            id: ScreenId(i as u32),
            name: format!("Display {}", i + 1),
            width: (vw / num_monitors.max(1)) as u32,
            height: vh as u32,
            x: vx + (i * vw / num_monitors.max(1)),
            y: vy,
            scale_factor: 1.0,
            is_primary: i == 0,
        });
    }

    Ok(screens)
}

fn show_status() -> Result<()> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("omnibridge");

    let _config_manager = LayoutConfigManager::new(&config_dir);

    println!("OmniBridge Status");
    println!("================");
    println!("Config directory: {:?}", config_dir);
    println!("Layout config exists: {}", config_dir.join("layout.json").exists());

    Ok(())
}

use ob_cli::LayoutAction;
use ob_cli::ConfigAction;

fn handle_layout_command(action: LayoutAction) -> Result<()> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("omnibridge");

    let mut config_manager = LayoutConfigManager::new(&config_dir);

    match action {
        LayoutAction::Show => {
            config_manager.load()?;
            let config = config_manager.config();
            println!("Layout Configuration:");
            println!("  Devices: {}", config.device_positions.len());
            println!("  Edges: {}", config.edges.len());

            for (id, pos) in &config.device_positions {
                println!("  Device {}: ({}, {})", id, pos.x, pos.y);
            }
        }
        LayoutAction::Set { from, to, direction } => {
            println!("Setting layout: {} -> {} ({})", from, to, direction);
            config_manager.load()?;
            config_manager.save()?;
        }
        LayoutAction::Reset => {
            config_manager.config_mut().device_positions.clear();
            config_manager.config_mut().edges.clear();
            config_manager.save()?;
            println!("Layout reset to defaults");
        }
    }

    Ok(())
}

fn handle_config_command(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            println!("OmniBridge Configuration");
            println!("=======================");
            println!("Default port: 19810");
            println!("Default name: omnibridge-node");
        }
        ConfigAction::Set { key, value } => {
            println!("Setting config: {} = {}", key, value);
        }
        ConfigAction::Reset => {
            println!("Configuration reset to defaults");
        }
    }

    Ok(())
}
