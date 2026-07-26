use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::DwellError;

/// Top-level dwell configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub meta: MetaConfig,
    pub data: DataConfig,
    pub modules: ModuleConfig,
    pub packages: PackageConfig,
    pub secrets: SecretConfig,
    pub plugins: PluginConfig,
    pub generations: GenerationConfig,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/home".to_string());
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Config {
            meta: MetaConfig {
                name: "dotfiles".to_string(),
                version: "0.1.0".to_string(),
            },
            data: DataConfig {
                home,
                hostname,
                platform: std::env::consts::OS.to_string(),
                extra: HashMap::new(),
            },
            modules: ModuleConfig {
                enabled: HashMap::new(),
            },
            packages: PackageConfig {
                system: Vec::new(),
            },
            secrets: SecretConfig {
                files: Vec::new(),
            },
            plugins: PluginConfig {
                wasm: Vec::new(),
                lua: Vec::new(),
            },
            generations: GenerationConfig {
                keep: 10,
            },
        }
    }
}

impl Config {
    /// Load configuration from a TOML file path.
    /// Supports `~` expansion for the home directory.
    pub fn load(path: &Path) -> crate::Result<Self> {
        let expanded = expand_tilde(path);
        let content = fs::read_to_string(&expanded)
            .map_err(|e| DwellError::InvalidConfig(format!(
                "Failed to read config at {}: {}", expanded.display(), e
            )))?;
        let cfg: Config = toml::from_str(&content)
            .map_err(|e| DwellError::InvalidConfig(format!(
                "Failed to parse config: {}", e
            )))?;
        Ok(cfg)
    }
}

fn expand_tilde(path: &Path) -> std::path::PathBuf {
    let s = path.to_string_lossy();
    if s.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            if s == "~" {
                return home;
            }
            let rest = s.strip_prefix("~/").unwrap_or(&s[1..]);
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

/// Meta-section: name and version of this configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaConfig {
    pub name: String,
    pub version: String,
}

impl Default for MetaConfig {
    fn default() -> Self {
        MetaConfig {
            name: "dotfiles".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

/// Data-section: environment context with extra custom fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DataConfig {
    pub home: String,
    pub hostname: String,
    pub platform: String,
    /// Extra data fields (flattened into templates).
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

impl Default for DataConfig {
    fn default() -> Self {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/home".to_string());
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        DataConfig {
            home,
            hostname,
            platform: std::env::consts::OS.to_string(),
            extra: HashMap::new(),
        }
    }
}

/// Module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleConfig {
    /// Enabled modules with their options.
    pub enabled: HashMap<String, toml::Value>,
}

impl Default for ModuleConfig {
    fn default() -> Self {
        ModuleConfig {
            enabled: HashMap::new(),
        }
    }
}

/// Package configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PackageConfig {
    /// System packages to install.
    pub system: Vec<String>,
}

impl Default for PackageConfig {
    fn default() -> Self {
        PackageConfig {
            system: Vec::new(),
        }
    }
}

/// Secret configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecretConfig {
    /// Encrypted files to decrypt.
    pub files: Vec<String>,
}

impl Default for SecretConfig {
    fn default() -> Self {
        SecretConfig {
            files: Vec::new(),
        }
    }
}

/// Plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginConfig {
    pub wasm: Vec<String>,
    pub lua: Vec<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        PluginConfig {
            wasm: Vec::new(),
            lua: Vec::new(),
        }
    }
}

/// Generation (snapshot) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GenerationConfig {
    /// Number of old generations to keep.
    pub keep: usize,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        GenerationConfig { keep: 10 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_empty_config_deserialization() {
        let cfg: Config = toml::from_str("").unwrap();
        let default = Config::default();
        assert_eq!(cfg.meta.name, default.meta.name);
        assert_eq!(cfg.meta.version, default.meta.version);
        assert_eq!(cfg.data.platform, default.data.platform);
        assert!(cfg.modules.enabled.is_empty());
        assert!(cfg.packages.system.is_empty());
        assert!(cfg.secrets.files.is_empty());
        assert!(cfg.plugins.wasm.is_empty());
        assert!(cfg.plugins.lua.is_empty());
        assert_eq!(cfg.generations.keep, 10);
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
[meta]
name = "my-dotfiles"
version = "1.0.0"

[packages]
system = ["vim", "tmux"]

[generations]
keep = 5
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.meta.name, "my-dotfiles");
        assert_eq!(cfg.meta.version, "1.0.0");
        assert_eq!(cfg.packages.system, vec!["vim", "tmux"]);
        assert_eq!(cfg.generations.keep, 5);
        // Unset sections should have defaults
        assert!(cfg.modules.enabled.is_empty());
        assert!(cfg.secrets.files.is_empty());
        assert!(cfg.plugins.wasm.is_empty());
        assert!(cfg.plugins.lua.is_empty());
    }

    #[test]
    fn test_config_load_with_temp_file() {
        let toml_str = r#"
[meta]
name = "test-dotfiles"

[generations]
keep = 3
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "{}", toml_str).unwrap();
        let path = tmp.path().to_path_buf();
        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.meta.name, "test-dotfiles");
        assert_eq!(cfg.generations.keep, 3);
        // Defaults preserved for unset fields
        assert_eq!(cfg.meta.version, "0.1.0");
    }

    #[test]
    fn test_config_load_missing_file() {
        let result = Config::load(Path::new("/nonexistent/dwell.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_expand_tilde() {
        let path = expand_tilde(Path::new("~/myconfig.toml"));
        assert!(path.is_absolute());
        assert!(path.to_string_lossy().contains("myconfig.toml"));
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let path = expand_tilde(Path::new("/etc/dwell.toml"));
        assert_eq!(path.to_string_lossy(), "/etc/dwell.toml");
    }
}
