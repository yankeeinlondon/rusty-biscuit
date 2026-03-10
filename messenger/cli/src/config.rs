use std::collections::HashMap;
use std::path::PathBuf;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

/// CLI configuration loaded from `~/.messenger.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub default_route: Option<String>,
    #[serde(default)]
    pub routes: HashMap<String, RouteConfig>,
}

/// A named route pointing to a provider and channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub provider: String,
    pub channel_id: String,
    #[serde(default = "default_token_env_empty")]
    pub token_env: String,
}

fn default_token_env_empty() -> String {
    String::new()
}

impl Config {
    /// Load config from `~/.messenger.json`, returning default if not found.
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Save config to `~/.messenger.json`.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            color_eyre::eyre::eyre!("could not determine home directory")
        })?;
        Ok(home.join(".messenger.json"))
    }

    /// Return all route names that use the given provider.
    pub fn routes_for_provider(&self, provider: &str) -> Vec<String> {
        self.routes
            .iter()
            .filter(|(_, r)| r.provider == provider)
            .map(|(name, _)| name.clone())
            .collect()
    }
}
