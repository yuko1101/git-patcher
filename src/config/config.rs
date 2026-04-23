use std::path::PathBuf;

use serde::Deserialize;

use crate::patcher::sync_strategy::SyncStrategy;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub sync_strategy: SyncStrategy,
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> anyhow::Result<Self> {
        let path = path.or_else(|| {
            std::env::var("GIT_PATCHER_CONFIG")
                .ok()
                .map(|e| PathBuf::from(e))
        });
        let content = path.map(|p| std::fs::read_to_string(p)).transpose()?;
        let config = content
            .map(|c| toml::from_str(&c))
            .transpose()?
            .unwrap_or_default();

        Ok(config)
    }
}
