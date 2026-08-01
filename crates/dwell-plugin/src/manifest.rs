use std::path::Path;

use serde::{Deserialize, Serialize};

use dwell_core::{DwellError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Wasm,
    Lua,
}

impl PluginKind {
    pub fn from_source(path: &Path) -> Option<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("wasm") => Some(Self::Wasm),
            Some("lua") => Some(Self::Lua),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub entry: String,
    pub description: Option<String>,
}

impl PluginManifest {
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(DwellError::Io)?;
        toml::from_str(&content)
            .map_err(|e| DwellError::Plugin(format!("Invalid plugin.toml: {}", e)))
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let body = toml::to_string_pretty(self).map_err(|e| {
            DwellError::Plugin(format!("Failed to serialize plugin manifest: {}", e))
        })?;
        std::fs::write(path, body).map_err(DwellError::Io)
    }
}
