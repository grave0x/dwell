use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Entry, Result};

/// Trait for template engines (Handlebars, Go-style, Tera, etc.).
pub trait TemplateEngine: Send + Sync {
    /// Name of this engine (for config selection).
    fn name(&self) -> &str;

    /// Render a template string with the given data context.
    fn render(&self, template: &str, data: &serde_json::Value) -> Result<String>;

    /// Check if a template string is valid (no syntax errors).
    fn validate(&self, template: &str) -> std::result::Result<(), String>;

    /// The file extension this engine handles (e.g. ".hbs", ".gotmpl").
    fn file_extension(&self) -> &str;
}

/// Trait for package managers (pacman, apt, brew, nix, etc.).
pub trait PackageManager: Send + Sync {
    /// Unique identifier, e.g. "pacman", "brew", "nix".
    fn id(&self) -> &str;

    /// Human-readable name.
    fn display_name(&self) -> &str;

    /// Check if this package manager is available on the current system.
    fn is_available(&self) -> bool;

    /// List all installed packages managed by this backend.
    fn list_installed(&self) -> Result<Vec<PackageInfo>>;

    /// Install a single package. Returns true if newly installed, false if already present.
    fn install(&self, package: &str) -> Result<bool>;

    /// Remove a single package. Returns true if actually removed.
    fn remove(&self, package: &str) -> Result<bool>;

    /// Search for a package by name/description.
    fn search(&self, query: &str) -> Result<Vec<PackageInfo>>;
}

/// Information about a single package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub installed: bool,
}

/// Trait for secret backends (age, GPG, password managers).
pub trait SecretBackend: Send + Sync {
    /// Unique identifier.
    fn id(&self) -> &str;

    /// Retrieve a secret by key/path.
    fn get_secret(&self, key: &str) -> Result<String>;

    /// Encrypt plaintext and store it.
    fn encrypt(&self, key: &str, plaintext: &[u8]) -> Result<Vec<u8>>;

    /// Decrypt ciphertext for the given key.
    fn decrypt(&self, key: &str, ciphertext: &[u8]) -> Result<Vec<u8>>;

    /// Check if this backend has a secret available.
    fn has_secret(&self, key: &str) -> bool;
}

/// Trait for declarative modules that generate dotfile entries.
pub trait Module: Send + Sync {
    /// Unique module identifier, e.g. "programs.git".
    fn id(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// The options this module exposes.
    fn options(&self) -> Vec<ModuleOption>;

    /// Generate source entries from resolved user configuration.
    fn generate(&self, values: &serde_json::Value) -> Result<Vec<Entry>>;
}

/// A single option declared by a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleOption {
    pub name: String,
    pub option_type: OptionType,
    pub default: Option<serde_json::Value>,
    pub description: String,
    pub example: Option<String>,
    pub required: bool,
}

/// Primitive and composite types for module options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OptionType {
    String,
    Integer,
    Float,
    Boolean,
    Path,
    List(Box<OptionType>),
    Dict(HashMap<String, OptionType>),
    /// One of a set of allowed values.
    Enum(Vec<String>),
    /// Nested sub-module options.
    SubModule,
}
