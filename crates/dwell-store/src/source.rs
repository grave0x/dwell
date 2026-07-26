//! Source directory enumeration and entry management.

use std::fs;
use std::path::{Path, PathBuf};

use dwell_core::{Entry, EntryKind, Result, hash_content};
use super::ignore::IgnorePatterns;

/// Manages the source directory — the directory containing dotfile templates.
pub struct SourceDir {
    pub root: PathBuf,
    ignore: IgnorePatterns,
}

impl SourceDir {
    /// Open an existing source directory.
    pub fn open(root: &Path) -> Result<Self> {
        let ignore_path = root.join(".dwellignore");
        let chezmoi_ignore = root.join(".chezmoiignore");

        let ignore = if ignore_path.exists() {
            IgnorePatterns::from_file(&ignore_path).map_err(|e| {
                dwell_core::DwellError::Io(e)
            })?
        } else if chezmoi_ignore.exists() {
            IgnorePatterns::from_file(&chezmoi_ignore).map_err(|e| {
                dwell_core::DwellError::Io(e)
            })?
        } else {
            IgnorePatterns::default()
        };

        Ok(SourceDir {
            root: root.to_path_buf(),
            ignore,
        })
    }

    /// Enumerate all source entries, respecting ignore patterns.
    pub fn entries(&self) -> Result<Vec<Entry>> {
        let mut entries = vec![];
        self.walk_dir(&self.root, "", &mut entries)?;
        Ok(entries)
    }

    fn walk_dir(&self, dir: &Path, prefix: &str, entries: &mut Vec<Entry>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let rel_path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };

            // Skip dotfiles and git internals
            if name.starts_with('.') || name == "templates" || name == "externals" {
                continue;
            }

            // Check ignore patterns
            if self.ignore.is_ignored(&rel_path) {
                continue;
            }

            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                entries.push(Entry {
                    source_path: rel_path.clone(),
                    kind: EntryKind::Directory,
                    content_hash: String::new(),
                    encrypted: false,
                });
                self.walk_dir(&entry.path(), &rel_path, entries)?;
            } else if file_type.is_file() || file_type.is_symlink() {
                let encrypted = name.ends_with(".age");
                let content = if encrypted {
                    Vec::new() // don't hash encrypted content during enumeration
                } else {
                    fs::read(entry.path()).unwrap_or_default()
                };
                let content_hash = if encrypted {
                    "encrypted".to_string()
                } else {
                    hash_content(&content)
                };

                entries.push(Entry {
                    source_path: rel_path.clone(),
                    kind: EntryKind::from_filename(&name),
                    content_hash,
                    encrypted,
                });
            }
        }
        Ok(())
    }

    /// Read the raw content of a source entry.
    pub fn read_entry(&self, source_path: &str) -> Result<Vec<u8>> {
        let full_path = self.root.join(source_path);
        fs::read(&full_path).map_err(|e| {
            dwell_core::DwellError::Io(e)
        })
    }

    /// Add a file from the filesystem to the source directory.
    /// Copies the file in, applying the dot_ prefix convention.
    pub fn add_file(&self, target_path: &Path, home: &Path) -> Result<Entry> {
        let relative = target_path
            .strip_prefix(home)
            .map_err(|_| dwell_core::DwellError::InvalidConfig(
                format!("Path {} is not under home directory", target_path.display())
            ))?;

        let relative_str = relative.to_string_lossy();
        // Apply prefix: .bashrc → dot_bashrc, .config/nvim/init.lua → dot_config/nvim/init.lua
        let source_name = if relative_str.starts_with('.') {
            // .config/nvim/init.lua → dot_config/nvim/init.lua
            let stripped = relative_str.strip_prefix('.').unwrap_or(&relative_str);
            format!("dot_{}", stripped)
        } else {
            relative_str.to_string()
        };

        let dest = self.root.join(&source_name);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(target_path, &dest)?;

        let content = fs::read(&dest)?;
        Ok(Entry {
            source_path: source_name,
            kind: EntryKind::File,
            content_hash: hash_content(&content),
            encrypted: false,
        })
    }

    /// Detect and load chezmoi-compatible source directory.
    /// chezmoi uses `dot_` prefix and `.chezmoi.yaml.toml`.
    pub fn detect_chezmoi(root: &Path) -> bool {
        // Check for chezmoi.yaml.toml or .chezmoi.yaml.toml
        root.join(".chezmoi.yaml.toml").exists()
            || root.join("chezmoi.yaml.toml").exists()
            || root.join(".chezmoi.toml.tmpl").exists()
    }

    /// Detect dotter-compatible source directory.
    pub fn detect_dotter(root: &Path) -> bool {
        root.join(".dotter").is_dir()
            || root.join("global.toml").exists()
    }
}
