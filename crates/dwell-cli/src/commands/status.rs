use std::path::PathBuf;

use crate::commands::{resolve_home, resolve_source, CliRef};
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    source: Option<PathBuf>,
) -> dwell_core::Result<()> {
    let _ = _cfg;
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let source_dir = resolve_source(cli, source)?;

    out.info(&format!("Checking status of {}", source_dir.display()));

    let sd = dwell_store::SourceDir::open(&source_dir)?;
    let entries = sd.entries()?;

    let state_path = dirs::data_dir()
        .map(|d| d.join("dwell").join("state.json"))
        .unwrap_or_else(|| PathBuf::from("/tmp/dwell-state.json"));

    let state = dwell_store::DeployState::load(&state_path)
        .unwrap_or_else(|_| dwell_store::DeployState::new());

    let home = resolve_home();
    let mut in_sync = 0;
    let mut modified = 0;
    let mut missing = 0;

    for entry in &entries {
        let target = dwell_core::source_to_target(&entry.source_path, &home);
        let deployed_hash = state.get_hash(&target);

        if !target.exists() {
            out.warn(&format!("Missing: {}", target.display()));
            missing += 1;
        } else if let Some(hash) = deployed_hash {
            let current = std::fs::read(&target).unwrap_or_default();
            let current_hash = dwell_core::hash_content(&current);
            if current_hash != hash {
                out.warn(&format!("Modified: {}", target.display()));
                modified += 1;
            } else {
                in_sync += 1;
            }
        } else {
            out.warn(&format!("Untracked: {}", target.display()));
            modified += 1;
        }
    }

    out.success(&format!(
        "{} in sync, {} modified/missing, {} total",
        in_sync,
        modified + missing,
        entries.len()
    ));

    Ok(())
}
