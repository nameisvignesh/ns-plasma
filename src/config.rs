use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::provider::ProviderSettings;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("could not parse config: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UiSettings {
    pub theme: String,
    pub accent: String,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: "plasma-dark".to_owned(),
            accent: "#8CA8FF".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub provider: Option<ProviderSettings>,
    pub ui: UiSettings,
}

impl AppConfig {
    pub fn path(workspace: &Path) -> PathBuf {
        workspace.join(".plasma").join("settings.json")
    }

    pub fn load(workspace: &Path) -> Result<Self, ConfigError> {
        let path = Self::path(workspace);
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, workspace: &Path) -> Result<(), ConfigError> {
        let directory = workspace.join(".plasma");
        fs::create_dir_all(&directory)?;
        let path = Self::path(workspace);
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, format!("{text}\n"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_loads_safe_defaults() {
        let root = std::env::temp_dir().join(format!("ns-plasma-config-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let config = AppConfig::load(&root).expect("defaults should load");
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn config_round_trips_without_secrets() {
        let root =
            std::env::temp_dir().join(format!("ns-plasma-config-roundtrip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let config = AppConfig {
            provider: Some(ProviderSettings {
                name: "Local".to_owned(),
                base_url: "http://localhost:11434/v1".to_owned(),
                model: "model-id".to_owned(),
                api_key_env: None,
            }),
            ui: UiSettings::default(),
        };
        config.save(&root).expect("save should work");
        let loaded = AppConfig::load(&root).expect("load should work");
        assert_eq!(loaded, config);
        let serialized = fs::read_to_string(AppConfig::path(&root)).expect("file exists");
        assert!(!serialized.contains("secret-value"));
        let _ = fs::remove_dir_all(root);
    }
}
