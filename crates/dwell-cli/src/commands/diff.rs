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
    let home = resolve_home();

    out.info(&format!(
        "Diffing {} against {}",
        source_dir.display(),
        home.display()
    ));

    let sd = dwell_store::SourceDir::open(&source_dir)?;
    let differ = dwell_deploy::Differ::new(home);
    let diffs = differ.diff_all(&sd)?;

    let changed: Vec<_> = diffs
        .iter()
        .filter(|d| d.status != dwell_core::DiffStatus::Unchanged)
        .collect();

    if changed.is_empty() {
        out.success("No changes — everything is in sync.");
        return Ok(());
    }

    for diff in &changed {
        let icon = match diff.status {
            dwell_core::DiffStatus::Added => "+",
            dwell_core::DiffStatus::Modified => "~",
            dwell_core::DiffStatus::Removed => "-",
            dwell_core::DiffStatus::Conflict => "!",
            dwell_core::DiffStatus::Unchanged => " ",
        };
        out.info(&format!("{} {}", icon, diff.path.display()));

        if cli.verbose > 0 {
            if let Some(ref text) = diff.diff_text {
                for line in text.lines() {
                    eprintln!("  {}", line);
                }
            }
        }
    }

    out.info(&format!(
        "{} files changed, {} unchanged",
        changed.len(),
        diffs.len() - changed.len()
    ));

    Ok(())
}
