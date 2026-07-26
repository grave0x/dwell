use std::path::PathBuf;

use crate::commands::{resolve_home, resolve_source, CliRef};
use crate::output::Output;

pub fn run(cli: &CliRef, _cfg: &dwell_core::Config, path: PathBuf) -> dwell_core::Result<()> {
    let _ = _cfg;
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let source_dir = resolve_source(cli, None)?;
    let home = resolve_home();

    // Resolve path relative to home if not absolute
    let full_path = if path.is_absolute() {
        path
    } else {
        home.join(&path)
    };

    if !full_path.exists() {
        return Err(dwell_core::DwellError::NotFound(full_path));
    }

    let sd = dwell_store::SourceDir::open(&source_dir)?;
    let entry = sd.add_file(&full_path, &home)?;

    out.success(&format!(
        "Added: {} → {}",
        full_path.display(),
        entry.source_path
    ));

    // Also stage in git
    match dwell_store::GitRepo::open(&source_dir.join(".git")) {
        Ok(repo) => {
            repo.add(&entry.source_path)?;
            out.info("Staged in git repository");
        }
        Err(_) => out.warn("No git repository found — file added to source dir only"),
    }

    Ok(())
}
