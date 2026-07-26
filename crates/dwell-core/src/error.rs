use std::path::PathBuf;

/// Result type alias for all dwell operations.
pub type Result<T> = std::result::Result<T, DwellError>;

/// Unified error type for the dwell toolchain.
#[derive(Debug, thiserror::Error)]
pub enum DwellError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File not found: {0}")]
    NotFound(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Already exists: {0}")]
    AlreadyExists(PathBuf),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Package manager error: {0}")]
    PackageManager(String),

    #[error("Module error: {0}")]
    Module(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("{0}")]
    Other(String),
}

impl From<&str> for DwellError {
    fn from(s: &str) -> Self {
        DwellError::Other(s.to_string())
    }
}

impl From<String> for DwellError {
    fn from(s: String) -> Self {
        DwellError::Other(s)
    }
}
