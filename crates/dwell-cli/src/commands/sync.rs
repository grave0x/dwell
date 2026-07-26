use std::path::PathBuf;

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    message: Option<String>,
    source: Option<PathBuf>,
    dry_run: bool,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let source_dir = crate::commands::resolve_source(cli, source)?;
    let git_dir = source_dir.join(".git");

    let repo = dwell_store::GitRepo::open(&git_dir)?;

    // Check for remote
    let remote_url = repo.remote_url()?;
    if remote_url.is_none() {
        out.warn("No remote configured. Set one with: git remote add origin <url>");
    }

    // Check for changes
    if !repo.has_changes()? {
        out.success("Nothing to sync — working tree is clean.");
        return Ok(());
    }

    // Show status
    let status = repo.status_summary()?;
    if !status.is_empty() {
        out.info("Changes to sync:");
        for line in status.lines() {
            eprintln!("  {}", line);
        }
    }

    if dry_run {
        out.info("Dry-run — no changes committed or pushed.");
        return Ok(());
    }

    // Stage all
    repo.add_all()?;
    out.info("Staged all changes");

    // Commit
    let commit_msg = message.unwrap_or_else(|| {
        let file_count = status.lines().count();
        format!("dwell: sync {} file(s)", file_count)
    });
    let commit_id = repo.commit(&commit_msg)?;
    out.success(&format!("Committed: {}", &commit_id[..8]));

    // Push if remote is configured
    if let Some(url) = remote_url {
        repo.push()?;
        out.success(&format!("Pushed to {}", url));
    }

    Ok(())
}
