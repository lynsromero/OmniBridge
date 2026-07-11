use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniBridgeConfig {
    pub device_name: String,
    pub is_primary: bool,
    pub port: u16,
    pub auto_discover: bool,
    pub edge_threshold: u32,
    pub capture_fps: u32,
    pub bitrate: u32,
    pub enable_encryption: bool,
    pub layout: Option<String>,
}

impl Default for OmniBridgeConfig {
    fn default() -> Self {
        Self {
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "omnibridge-node".to_string()),
            is_primary: false,
            port: 19810,
            auto_discover: true,
            edge_threshold: 5,
            capture_fps: 60,
            bitrate: 20_000_000,
            enable_encryption: true,
            layout: None,
        }
    }
}

#[allow(dead_code)]
impl OmniBridgeConfig {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("omnibridge")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(data) => {
                    serde_json::from_str(&data).unwrap_or_default()
                }
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}
