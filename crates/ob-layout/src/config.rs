use anyhow::Result;
use ob_core::layout::LayoutConfig;
use std::path::{Path, PathBuf};
use tracing::info;

pub struct LayoutConfigManager {
    config_path: PathBuf,
    config: LayoutConfig,
}

impl LayoutConfigManager {
    pub fn new(config_dir: &Path) -> Self {
        let config_path = config_dir.join("layout.json");
        let config = Self::load_from_file(&config_path).unwrap_or_default();
        Self { config_path, config }
    }

    fn load_from_file(path: &Path) -> Result<LayoutConfig> {
        if !path.exists() {
            return Ok(LayoutConfig::default());
        }
        let data = std::fs::read_to_string(path)?;
        let config: LayoutConfig = serde_json::from_str(&data)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.config_path, json)?;
        info!("Layout config saved to {:?}", self.config_path);
        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        self.config = Self::load_from_file(&self.config_path)?;
        info!("Layout config loaded from {:?}", self.config_path);
        Ok(())
    }

    pub fn config(&self) -> &LayoutConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut LayoutConfig {
        &mut self.config
    }
}
