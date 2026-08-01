use std::path::PathBuf;

use dwell_store::{DeployState, GenerationManager};

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    cfg: &dwell_core::Config,
    action: crate::GenerationCommand,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet, cli.log_level.clone());
    let manager = GenerationManager::new();

    match action {
        crate::GenerationCommand::List => {
            let snapshots = manager.list()?;
            if snapshots.is_empty() {
                out.info("No saved generations.");
                return Ok(());
            }

            out.info(&format!("Generations ({})", snapshots.len()));
            for s in snapshots {
                out.info(&format!(
                    "  #{}: {} entries (created_at={})",
                    s.id,
                    s.entries.len(),
                    s.created_at
                ));
            }
            Ok(())
        }
        crate::GenerationCommand::Rollback { id } => {
            let target_id = match id {
                Some(x) => x,
                None => manager
                    .list()?
                    .into_iter()
                    .max_by_key(|s| s.id)
                    .map(|s| s.id)
                    .ok_or_else(|| {
                        dwell_core::DwellError::NotFound(PathBuf::from("No generations found"))
                    })?,
            };

            let restored = manager.rollback(target_id)?;
            out.success(&format!(
                "Rolled back generation #{} ({} path(s) restored)",
                target_id, restored
            ));

            let state_path = state_path();
            let mut state = DeployState::load(&state_path).unwrap_or_else(|_| DeployState::new());
            state.generation = target_id.saturating_sub(1);
            state
                .save(&state_path)
                .map_err(dwell_core::DwellError::Io)?;
            Ok(())
        }
        crate::GenerationCommand::Prune { keep } => {
            let keep = keep.unwrap_or(cfg.generations.keep);
            let removed = manager.prune(keep)?;
            out.success(&format!(
                "Pruned {} generation(s), keeping {}",
                removed, keep
            ));
            Ok(())
        }
    }
}

fn state_path() -> PathBuf {
    dirs::data_dir()
        .map(|d| d.join("dwell").join("state.json"))
        .unwrap_or_else(|| PathBuf::from("/tmp/dwell-state.json"))
}
