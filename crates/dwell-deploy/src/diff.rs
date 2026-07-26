//! Diff computation between source state and filesystem state.

use std::fs;
use std::path::PathBuf;

use dwell_core::{Diff, DiffStatus, Result, source_to_target};
use dwell_store::SourceDir;

/// Computes differences between the source directory and the target filesystem.
pub struct Differ {
    pub home: PathBuf,
}

impl Differ {
    pub fn new(home: PathBuf) -> Self {
        Differ { home }
    }

    /// Compute diffs for all entries in a source directory.
    pub fn diff_all(&self, source: &SourceDir) -> Result<Vec<Diff>> {
        let entries = source.entries()?;
        let mut diffs = vec![];

        for entry in &entries {
            let target = source_to_target(&entry.source_path, &self.home);
            let current_content = fs::read(&target).ok();
            let current_hash = current_content
                .as_ref()
                .map(|c| dwell_core::hash_content(c));

            let source_content = source.read_entry(&entry.source_path).ok();
            let source_hash = source_content
                .as_ref()
                .map(|c| dwell_core::hash_content(c));

            let status = match (&current_hash, &source_hash) {
                (None, Some(_)) => DiffStatus::Added,
                (Some(_), None) => DiffStatus::Removed,
                (Some(c), Some(s)) if c == s => DiffStatus::Unchanged,
                (Some(_), Some(_)) => DiffStatus::Modified,
                (None, None) => DiffStatus::Unchanged,
            };

            let diff_text = if status == DiffStatus::Modified {
                current_content.and_then(|cur| {
                    source_content.map(|src| {
                        let cur_str = String::from_utf8_lossy(&cur);
                        let src_str = String::from_utf8_lossy(&src);
                        compute_unified_diff(&cur_str, &src_str, &target)
                    })
                })
            } else {
                None
            };

            diffs.push(Diff {
                path: target,
                status,
                old_hash: current_hash,
                new_hash: source_hash,
                diff_text,
            });
        }

        Ok(diffs)
    }
}

/// Simple unified diff for display purposes.
fn compute_unified_diff(old: &str, new: &str, _path: &PathBuf) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut diff = String::new();
    // Very basic diff — just show differences
    let max_lines = old_lines.len().max(new_lines.len());

    diff.push_str("--- current\n+++ target\n");

    for i in 0..max_lines {
        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        match (old_line, new_line) {
            (Some(o), Some(n)) if o == n => {
                diff.push_str(&format!("  {}\n", o));
            }
            (Some(o), Some(n)) => {
                diff.push_str(&format!("- {}\n", o));
                diff.push_str(&format!("+ {}\n", n));
            }
            (Some(o), None) => {
                diff.push_str(&format!("- {}\n", o));
            }
            (None, Some(n)) => {
                diff.push_str(&format!("+ {}\n", n));
            }
            (None, None) => {}
        }
    }

    diff
}
