use std::{fs, path::PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

const STATE_JSON: &str = "state.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct InternalState {
    pub target_revision: Option<String>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl InternalState {
    fn new(path: PathBuf) -> Self {
        Self {
            target_revision: None,
            path: Some(path),
        }
    }

    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        let state_file = path.join(STATE_JSON);
        if state_file.exists() {
            let content = fs::read_to_string(state_file)?;
            let mut state: InternalState = serde_json::from_str(&content)?;
            state.path.replace(path);
            Ok(state)
        } else {
            Ok(InternalState::new(path))
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = self
            .path
            .as_ref()
            .context("InternalState path is not set. This should never happen.")?;
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }

        let state_file = &path.join(STATE_JSON);
        let content = serde_json::to_string_pretty(self)?;
        fs::write(state_file, content)?;
        Ok(())
    }
}
