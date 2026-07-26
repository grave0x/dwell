//! Symlink management utilities.

use std::fs;
use std::os::unix;
use std::path::Path;

use dwell_core::{DwellError, Result};

/// Create a symlink from source to target.
pub fn link_file(source: &Path, target: &Path, dry_run: bool) -> Result<()> {
    if dry_run {
        tracing::info!(
            "[dry-run] Would symlink: {} → {}",
            source.display(),
            target.display()
        );
        return Ok(());
    }

    // Remove existing file/symlink if present
    if target.exists() || target.is_symlink() {
        fs::remove_file(target)?;
    }

    // Create parent directories
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    unix::fs::symlink(source, target).map_err(|e| DwellError::Io(e))
}

/// Remove a symlink target if it's a symlink managed by dwell.
pub fn unlink_target(target: &Path) -> Result<()> {
    if target.is_symlink() {
        fs::remove_file(target)?;
    }
    Ok(())
}

/// Verify that a symlink points to the expected source.
pub fn verify_symlink(target: &Path, expected_source: &Path) -> Result<bool> {
    if !target.is_symlink() {
        return Ok(false);
    }
    let actual = fs::read_link(target)?;
    Ok(actual == expected_source)
}
