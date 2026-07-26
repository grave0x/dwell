use std::path::PathBuf;

use crate::commands::{resolve_source, CliRef};
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    source: Option<PathBuf>,
) -> dwell_core::Result<()> {
    let _ = _cfg;
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let source_dir = resolve_source(cli, source)?;

    out.info(&format!("Watching {}", source_dir.display()));

    let watcher = dwell_deploy::watch::Watcher::new(source_dir);
    watcher.watch(|path| {
        eprintln!("  changed: {}", path);
    })?;

    Ok(())
}
