//! Core deployment logic — render templates and write targets.

use std::fs;
use std::path::{Path, PathBuf};

use dwell_core::{ApplyAction, ApplyResult, Entry, EntryKind, Result, hash_content, source_to_target};
use dwell_store::DeployState;
use dwell_template::TemplateRegistry;

use super::link;

/// Main deployer: takes entries, renders templates, applies to filesystem.
pub struct Deployer {
    pub home: PathBuf,
    pub dry_run: bool,
    pub force: bool,
    state: DeployState,
    templates: TemplateRegistry,
}

impl Deployer {
    pub fn new(home: PathBuf, state_path: &Path, dry_run: bool) -> Result<Self> {
        let state = DeployState::load(state_path).unwrap_or_else(|_| DeployState::new());
        Ok(Deployer {
            home,
            dry_run,
            force: false,
            state,
            templates: TemplateRegistry::new(),
        })
    }

    /// Set force mode (overwrite even if content differs).
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Apply a single entry to the target filesystem.
    pub fn apply_entry(
        &mut self,
        entry: &Entry,
        source_root: &Path,
        data: &serde_json::Value,
    ) -> Result<ApplyResult> {
        let target = source_to_target(&entry.source_path, &self.home);

        match entry.kind {
            EntryKind::Directory => {
                if !self.dry_run {
                    fs::create_dir_all(&target)?;
                }
                Ok(ApplyResult {
                    path: target,
                    action: ApplyAction::Created,
                    success: true,
                    error: None,
                })
            }
            EntryKind::Remove => {
                if target.exists() {
                    if !self.dry_run {
                        fs::remove_file(&target)?;
                    }
                    Ok(ApplyResult {
                        path: target,
                        action: ApplyAction::Deleted,
                        success: true,
                        error: None,
                    })
                } else {
                    Ok(ApplyResult {
                        path: target,
                        action: ApplyAction::Skipped,
                        success: true,
                        error: None,
                    })
                }
            }
            EntryKind::Symlink => {
                let source = source_root.join(&entry.source_path);
                match link::link_file(&source, &target, self.dry_run) {
                    Ok(_) => Ok(ApplyResult {
                        path: target,
                        action: ApplyAction::Symlinked,
                        success: true,
                        error: None,
                    }),
                    Err(e) => Ok(ApplyResult {
                        path: target,
                        action: ApplyAction::Skipped,
                        success: false,
                        error: Some(e.to_string()),
                    }),
                }
            }
            _ => {
                // File, RunOnce, RunBefore, RunAfter — all involve content
                let raw = std::fs::read(source_root.join(&entry.source_path))?;
                let content = if entry.encrypted {
                    raw // decrypt later
                } else if self.templates.is_template(&entry.source_path) {
                    let template_str =
                        String::from_utf8(raw).unwrap_or_default();
                    self.templates
                        .engine_for_file(&entry.source_path)
                        .render(&template_str, data)?
                        .into_bytes()
                } else {
                    raw
                };

                let new_hash = hash_content(&content);
                let old_hash = self.state.get_hash(&target).map(|s| s.to_string());

                let action = if !target.exists() {
                    ApplyAction::Created
                } else if old_hash.as_ref() == Some(&new_hash) {
                    ApplyAction::Skipped
                } else if self.force || old_hash.is_none() {
                    ApplyAction::Updated
                } else {
                    ApplyAction::Skipped // would conflict without force
                };

                if action != ApplyAction::Skipped && !self.dry_run {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&target, &content)?;
                    self.state
                        .record(&target, &entry.source_path, &new_hash, "file");
                }

                Ok(ApplyResult {
                    path: target,
                    action,
                    success: true,
                    error: None,
                })
            }
        }
    }

    /// Apply all entries from a source directory.
    pub fn apply_all(
        &mut self,
        entries: &[Entry],
        source_root: &Path,
        data: &serde_json::Value,
    ) -> Result<Vec<ApplyResult>> {
        let mut results = vec![];
        for entry in entries {
            match self.apply_entry(entry, source_root, data) {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(ApplyResult {
                        path: source_to_target(&entry.source_path, &self.home),
                        action: ApplyAction::Skipped,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
        self.state.next_generation();
        Ok(results)
    }

    /// Save deployment state to disk.
    pub fn save_state(&self, path: &Path) -> std::io::Result<()> {
        self.state.save(path)
    }

    /// Get a reference to the deploy state.
    pub fn state(&self) -> &DeployState {
        &self.state
    }
}
