use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omnibridge")]
#[command(about = "Advanced cross-device KVM with seamless window dragging")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Start {
        #[arg(short, long, default_value = "omnibridge-node")]
        name: String,

        #[arg(short, long, default_value = "19810")]
        port: u16,

        #[arg(short, long)]
        primary: bool,
    },

    Connect {
        #[arg(short, long)]
        address: String,

        #[arg(short, long, default_value = "19810")]
        port: u16,
    },

    Status,

    Gui,

    Layout {
        #[command(subcommand)]
        action: LayoutAction,
    },

    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
pub enum LayoutAction {
    Show,
    Set {
        #[arg(short, long)]
        from: String,

        #[arg(short, long)]
        to: String,

        #[arg(short, long)]
        direction: String,
    },
    Reset,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    Show,
    Set {
        #[arg(short, long)]
        key: String,

        #[arg(short, long)]
        value: String,
    },
    Reset,
}
