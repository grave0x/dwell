use std::path::{Path, PathBuf};

use dialoguer::Confirm;
use dwell_core::{source_to_target, Entry, EntryKind};
use dwell_store::SourceDir;

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    path: Option<PathBuf>,
    all: bool,
    stray: bool,
    source: Option<PathBuf>,
    dry_run: bool,
    interactive: bool,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet, cli.log_level.clone());
    let source_dir = crate::commands::resolve_source(cli, source)?;
    let home = crate::commands::resolve_home();

    let sd = SourceDir::open(&source_dir)?;
    let entries = sd.entries()?;

    if !all && !stray && path.is_none() {
        out.warn("Specify what to reset: --all, --stray, or a file path");
        return Ok(());
    }

    // ── Reset specific path ──────────────────────────────────────────
    if let Some(target_path) = path {
        let target_path = if target_path.is_absolute() {
            target_path
        } else {
            home.join(&target_path)
        };

        // Find the source entry for this target
        let entry = entries.iter().find(|e| {
            let t = source_to_target(&e.source_path, &home);
            t == target_path
        });

        match entry {
            Some(entry) => {
                out.info(&format!("Resetting: {}", target_path.display()));
                if dry_run {
                    out.info("  (dry-run — would re-apply from source)");
                } else {
                    reset_entry(&sd, entry, &source_dir, &home, &out, interactive)?;
                    out.success(&format!("  Reset: {}", target_path.display()));
                }
            }
            None => {
                out.warn(&format!("Not a managed file: {}", target_path.display()));
            }
        }
        return Ok(());
    }

    // ── Reset all ────────────────────────────────────────────────────
    if all {
        out.info("Resetting all managed files to source state...");
        let mut count = 0;
        for entry in &entries {
            if entry.kind.is_script() || entry.kind == EntryKind::Directory {
                continue;
            }
            if dry_run {
                let t = source_to_target(&entry.source_path, &home);
                out.info(&format!("  Would reset: {}", t.display()));
            } else {
                match reset_entry(&sd, entry, &source_dir, &home, &out, interactive) {
                    Ok(_) => count += 1,
                    Err(e) => out.warn(&format!("  Failed: {}: {}", entry.source_path, e)),
                }
            }
        }
        if !dry_run {
            out.success(&format!("  Reset {} file(s)", count));
        }
        return Ok(());
    }

    // ── Stray cleanup ────────────────────────────────────────────────
    if stray {
        out.info("Checking for stray deployed files...");
        let state_path = dirs::data_dir()
            .map(|d| d.join("dwell").join("state.json"))
            .unwrap_or_else(|| PathBuf::from("/tmp/dwell-state.json"));

        let state = dwell_store::DeployState::load(&state_path)
            .unwrap_or_else(|_| dwell_store::DeployState::new());

        let managed_targets: std::collections::HashSet<PathBuf> = entries
            .iter()
            .map(|e| source_to_target(&e.source_path, &home))
            .collect();

        let mut stray_count = 0;
        for target in state.entries.keys() {
            if !managed_targets.contains(target) && target.exists() {
                out.warn(&format!("  Stray: {}", target.display()));
                if !dry_run {
                    if target.is_dir() {
                        std::fs::remove_dir_all(target).ok();
                    } else {
                        std::fs::remove_file(target).ok();
                    }
                    out.success(&format!("  Removed: {}", target.display()));
                }
                stray_count += 1;
            }
        }

        if stray_count == 0 {
            out.success("No stray files found.");
        }
    }

    Ok(())
}

fn reset_entry(
    sd: &SourceDir,
    entry: &Entry,
    _source_root: &Path,
    home: &Path,
    out: &Output,
    interactive: bool,
) -> dwell_core::Result<()> {
    let target = source_to_target(&entry.source_path, home);
    if interactive && target.exists() && target.is_file() {
        let prompt = format!("Reset {} from source?", target.display());
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            out.info(&format!("  Skipped: {}", target.display()));
            return Ok(());
        }
    }
    let raw = sd.read_entry(&entry.source_path)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, &raw)?;
    out.success(&format!("  Reset: {}", target.display()));
    Ok(())
}
