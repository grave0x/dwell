use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Dependency manifest for a single dotfile entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEntry {
    /// System packages required (e.g. ["neovim", "ripgrep"])
    #[serde(default)]
    pub requires: Vec<String>,
    /// Other source entries this dotfile sources/imports
    #[serde(default)]
    pub sources: Vec<String>,
    /// Additional tools/binaries needed
    #[serde(default)]
    pub tools: Vec<String>,
    /// Custom download URLs for tools not in package managers
    #[serde(default)]
    pub custom: Vec<CustomTool>,
}

/// A tool that must be downloaded/bundled rather than installed via a package manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTool {
    pub name: String,
    pub version: Option<String>,
    pub url: Option<String>,
    pub extract_to: Option<String>,
    pub binary: Option<String>,
}

/// Top-level dependency configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DepsConfig {
    /// Per-source-entry dependencies
    #[serde(default)]
    pub deps: HashMap<String, DepEntry>,
}

impl DepsConfig {
    /// Load deps.toml from a source directory.
    pub fn load(source_root: &Path) -> std::io::Result<Self> {
        let path = source_root.join(".dwell").join("deps.toml");
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let config: DepsConfig = toml::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DepsConfig::default()),
            Err(e) => Err(e),
        }
    }

    /// Save deps.toml to a source directory.
    pub fn save(&self, source_root: &Path) -> std::io::Result<()> {
        let dir = source_root.join(".dwell");
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(dir.join("deps.toml"), content)
    }

    /// Collect all unique required packages across all entries.
    pub fn all_requires(&self) -> Vec<String> {
        let mut pkgs: Vec<String> = self.deps.values()
            .flat_map(|d| d.requires.iter().cloned())
            .collect();
        pkgs.sort();
        pkgs.dedup();
        pkgs
    }

    /// Collect all unique tools across all entries.
    pub fn all_tools(&self) -> Vec<String> {
        let mut tools: Vec<String> = self.deps.values()
            .flat_map(|d| d.tools.iter().cloned())
            .collect();
        tools.sort();
        tools.dedup();
        tools
    }
}
