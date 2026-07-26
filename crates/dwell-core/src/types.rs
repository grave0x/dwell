use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A source entry — a tracked item in the dotfile repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Relative path from source root, e.g. "dot_config/hypr/hyprland.conf"
    pub source_path: String,
    /// What kind of entry this is
    pub kind: EntryKind,
    /// SHA-256 hex digest of raw content (before template rendering)
    pub content_hash: String,
    /// Whether this entry is encrypted at rest
    pub encrypted: bool,
}

/// The classification of a source entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    /// Regular file — may contain template directives.
    File,
    /// A directory to track as-is (children are separate entries).
    Directory,
    /// A symlink to be reproduced verbatim.
    Symlink,
    /// A run-once script (run_once_ prefix).
    RunOnce,
    /// A run-before-apply script (run_before_ prefix).
    RunBefore,
    /// A run-after-apply script (run_after_ prefix).
    RunAfter,
    /// A removal directive (dwell_remove_ prefix) — delete the target.
    Remove,
}

impl EntryKind {
    /// Detect entry kind from the source filename prefix.
    pub fn from_filename(name: &str) -> Self {
        let name = name.strip_prefix("dot_").unwrap_or(name);
        if name.starts_with("run_once_") {
            EntryKind::RunOnce
        } else if name.starts_with("run_before_") {
            EntryKind::RunBefore
        } else if name.starts_with("run_after_") {
            EntryKind::RunAfter
        } else if name.starts_with("dwell_remove_") {
            EntryKind::Remove
        } else if name.starts_with("symlink_") {
            EntryKind::Symlink
        } else {
            EntryKind::File
        }
    }

    pub fn is_script(&self) -> bool {
        matches!(self, EntryKind::RunOnce | EntryKind::RunBefore | EntryKind::RunAfter)
    }
}

/// A resolved target — where a source entry gets deployed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    /// Absolute filesystem path for deployment.
    pub destination: PathBuf,
    /// Rendered content (if file), None for symlinks and directories.
    pub contents: Option<Vec<u8>>,
    /// Unix permission bits.
    pub mode: u32,
    /// Whether this should be a symlink to the source.
    pub symlink: bool,
    /// Make the target executable.
    pub executable: bool,
}

/// A diff between current filesystem state and desired target state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub path: PathBuf,
    pub status: DiffStatus,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
    /// Unified diff text (for text files).
    pub diff_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffStatus {
    Added,
    Modified,
    Removed,
    Unchanged,
    Conflict,
}

/// A deployment operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub path: PathBuf,
    pub action: ApplyAction,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplyAction {
    Created,
    Updated,
    Deleted,
    Skipped,
    Symlinked,
}

/// Content hash computation helper.
pub fn hash_content(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Convert a dot_-prefixed source name to its target path.
///
/// `dot_config/hypr/hyprland.conf` → `/home/user/.config/hypr/hyprland.conf`
/// `dot_bashrc` → `/home/user/.bashrc`
pub fn source_to_target(source_path: &str, home: &Path) -> PathBuf {
    // Determine if this is a dotfile (starts with `dot_` or `private_dot_`)
    let is_dotfile = source_path.starts_with("dot_") || source_path.starts_with("private_dot_");

    // Strip known prefixes
    let relative = source_path
        .strip_prefix("dot_")
        .or_else(|| source_path.strip_prefix("private_dot_"))
        .unwrap_or(source_path);

    // Strip suffixes
    let relative = relative
        .strip_suffix(".tmpl")
        .unwrap_or(relative);

    // Strip script prefixes
    let relative = relative
        .strip_prefix("run_once_")
        .or_else(|| relative.strip_prefix("run_before_"))
        .or_else(|| relative.strip_prefix("run_after_"))
        .or_else(|| relative.strip_prefix("dwell_remove_"))
        .or_else(|| relative.strip_prefix("symlink_"))
        .unwrap_or(relative);

    // Only prepend dot if the original was a dotfile
    let target_relative = if is_dotfile {
        if let Some((first, rest)) = relative.split_once('/') {
            format!(".{}/{}", first, rest)
        } else {
            format!(".{}", relative)
        }
    } else {
        relative.to_string()
    };

    home.join(target_relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_content() {
        let h = hash_content(b"hello");
        assert_eq!(h.len(), 64);
        assert_ne!(h, hash_content(b"world"));
    }

    #[test]
    fn test_entry_kind_detection() {
        assert_eq!(EntryKind::from_filename("run_once_install.sh"), EntryKind::RunOnce);
        assert_eq!(EntryKind::from_filename("run_before_check.sh"), EntryKind::RunBefore);
        assert_eq!(EntryKind::from_filename("run_after_reload.sh"), EntryKind::RunAfter);
        assert_eq!(EntryKind::from_filename("dwell_remove_stale"), EntryKind::Remove);
        assert_eq!(EntryKind::from_filename("symlink_config"), EntryKind::Symlink);
        assert_eq!(EntryKind::from_filename("bashrc"), EntryKind::File);
    }

    #[test]
    fn test_source_to_target_dotfile() {
        let home = PathBuf::from("/home/user");
        let target = source_to_target("dot_bashrc", &home);
        assert_eq!(target, PathBuf::from("/home/user/.bashrc"));
    }

    #[test]
    fn test_source_to_target_nested() {
        let home = PathBuf::from("/home/user");
        let target = source_to_target("dot_config/nvim/init.lua", &home);
        assert_eq!(target, PathBuf::from("/home/user/.config/nvim/init.lua"));
    }
}
