use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use dwell_core::{DwellError, Result};

use crate::manifest::{PluginKind, PluginManifest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub source: String,
    pub installed_at: u64,
}

pub struct PluginRegistry {
    root: PathBuf,
    index_path: PathBuf,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let root = dirs::data_dir()
            .map(|d| d.join("dwell").join("plugins"))
            .unwrap_or_else(|| PathBuf::from("/tmp/dwell/plugins"));
        let index_path = root.join("registry.json");
        Self { root, index_path }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list(&self) -> Result<Vec<InstalledPlugin>> {
        if !self.index_path.exists() {
            return Ok(Vec::new());
        }
        let body = std::fs::read_to_string(&self.index_path).map_err(DwellError::Io)?;
        serde_json::from_str(&body)
            .map_err(|e| DwellError::Plugin(format!("Invalid plugin registry: {}", e)))
    }

    pub fn install_from_source(&self, source: &Path) -> Result<InstalledPlugin> {
        std::fs::create_dir_all(&self.root).map_err(DwellError::Io)?;

        let (manifest, plugin_dir) = if source.is_dir() {
            let manifest_path = source.join("plugin.toml");
            if !manifest_path.exists() {
                return Err(DwellError::Plugin(format!(
                    "Missing plugin.toml in {}",
                    source.display()
                )));
            }
            let manifest = PluginManifest::load_from(&manifest_path)?;
            let plugin_dir = self
                .root
                .join(format!("{}-{}", manifest.name, manifest.version));
            if plugin_dir.exists() {
                std::fs::remove_dir_all(&plugin_dir).map_err(DwellError::Io)?;
            }
            copy_dir_recursive(source, &plugin_dir)?;
            (manifest, plugin_dir)
        } else if source.is_file() {
            let kind = PluginKind::from_source(source).ok_or_else(|| {
                DwellError::Plugin(format!(
                    "Unsupported plugin file type: {}",
                    source.display()
                ))
            })?;
            let stem = source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("plugin")
                .to_string();
            let version = "0.1.0".to_string();
            let manifest = PluginManifest {
                name: stem.clone(),
                version: version.clone(),
                kind,
                entry: source
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string(),
                description: Some("Installed from single file source".into()),
            };
            let plugin_dir = self.root.join(format!("{}-{}", stem, version));
            std::fs::create_dir_all(&plugin_dir).map_err(DwellError::Io)?;
            std::fs::copy(source, plugin_dir.join(&manifest.entry)).map_err(DwellError::Io)?;
            manifest.save_to(&plugin_dir.join("plugin.toml"))?;
            (manifest, plugin_dir)
        } else {
            return Err(DwellError::Plugin(format!(
                "Plugin source not found: {}",
                source.display()
            )));
        };

        let installed = InstalledPlugin {
            manifest,
            source: source.to_string_lossy().to_string(),
            installed_at: now_secs(),
        };

        self.upsert(installed.clone())?;
        let _ = plugin_dir;
        Ok(installed)
    }

    pub fn scaffold(&self, name: &str) -> Result<PathBuf> {
        let dir = std::env::current_dir().map_err(DwellError::Io)?.join(name);
        if dir.exists() {
            return Err(DwellError::AlreadyExists(dir));
        }

        std::fs::create_dir_all(dir.join("src")).map_err(DwellError::Io)?;

        let manifest = PluginManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            kind: PluginKind::Lua,
            entry: "src/main.lua".to_string(),
            description: Some("New Lua plugin".to_string()),
        };
        manifest.save_to(&dir.join("plugin.toml"))?;

        std::fs::write(
            dir.join("src/main.lua"),
            "return {\n  name = \"example\",\n  run = function(ctx)\n    return \"ok\"\n  end\n}\n",
        )
        .map_err(DwellError::Io)?;

        Ok(dir)
    }

    fn upsert(&self, plugin: InstalledPlugin) -> Result<()> {
        let mut items = self.list()?;
        items.retain(|p| p.manifest.name != plugin.manifest.name);
        items.push(plugin);
        items.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        let body = serde_json::to_string_pretty(&items)
            .map_err(|e| DwellError::Plugin(format!("Failed to serialize registry: {}", e)))?;
        std::fs::write(&self.index_path, body).map_err(DwellError::Io)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(DwellError::Io)?;
    for entry in std::fs::read_dir(from).map_err(DwellError::Io)? {
        let entry = entry.map_err(DwellError::Io)?;
        let path = entry.path();
        let dst = to.join(entry.file_name());
        let ty = entry.file_type().map_err(DwellError::Io)?;
        if ty.is_dir() {
            copy_dir_recursive(&path, &dst)?;
        } else if ty.is_file() {
            std::fs::copy(&path, &dst).map_err(DwellError::Io)?;
        }
    }
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
