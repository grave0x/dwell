use std::path::PathBuf;

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    source: Option<PathBuf>,
    no_deps: bool,
    no_packages: bool,
    dry_run: bool,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet, cli.log_level.clone());
    let source_dir = crate::commands::resolve_source(cli, source)?;
    let home = crate::commands::resolve_home();

    out.title("dwell deploy — full system setup");

    // Phase 1: Scan dependencies
    if !no_deps {
        out.step("Phase 1/4", "Scanning dependencies...");
        let deps = dwell_core::scanner::scan_source(&source_dir)?;
        if deps.deps.is_empty() {
            out.info("  No dependencies found.");
        } else {
            deps.save(&source_dir).ok();
            let all_req = deps.all_requires();
            let all_tools = deps.all_tools();
            out.info(&format!(
                "  Found {} entries with dependencies",
                deps.deps.len()
            ));
            if !all_req.is_empty() {
                out.info(&format!("  Requires: {}", all_req.join(", ")));
            }
            if !all_tools.is_empty() {
                out.info(&format!("  Tools: {}", all_tools.join(", ")));
            }

            // Install via package backends
            let reg = dwell_package::PackageManagerRegistry::new();
            if let Some(backend) = reg.detect() {
                out.info(&format!("  Installing via {}", backend.display_name()));
                if !dry_run {
                    for pkg in &all_req {
                        if let Err(e) = backend.install(pkg) {
                            out.warn(&format!("    Failed: {}: {}", pkg, e));
                        }
                    }
                }
            }
        }
    } else {
        out.step("Phase 1/4", "Skipping dependency scan (--no-deps)");
    }

    // Phase 2: Restore packages from setup capture
    if !no_packages {
        out.step("Phase 2/4", "Restoring captured packages...");
        let manifest_path = source_dir.join(".dwell/setup/manifest.json");
        if manifest_path.exists() {
            let reg = dwell_package::PackageManagerRegistry::new();
            if let Some(backend) = reg.detect() {
                out.info(&format!(
                    "  Restoring packages via {}",
                    backend.display_name()
                ));
                if !dry_run {
                    // Read manifest and install all captured packages
                    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(sys) = data.get("sys_packages").and_then(|v| v.as_object())
                            {
                                for (_backend_id, pkgs) in sys {
                                    if let Some(list) = pkgs.as_array() {
                                        for pkg in list {
                                            if let Some(name) = pkg.as_str() {
                                                if let Err(e) = backend.install(name) {
                                                    out.warn(&format!(
                                                        "    Failed: {}: {}",
                                                        name, e
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            out.info("  No captured state found. Run 'dwell setup capture' first.");
        }
    } else {
        out.step("Phase 2/4", "Skipping package restore (--no-packages)");
    }

    // Phase 3: Apply dotfiles
    out.step("Phase 3/4", "Applying dotfiles...");
    if !dry_run {
        let sd = dwell_store::SourceDir::open(&source_dir)?;
        let entries = sd.entries()?;
        out.info(&format!("  Found {} entries", entries.len()));

        let state_path = dirs::data_dir()
            .map(|d| d.join("dwell").join("state.json"))
            .unwrap_or_else(|| PathBuf::from("/tmp/dwell-state.json"));

        let mut data = serde_json::Map::new();
        data.insert(
            "home".into(),
            serde_json::Value::String(home.to_string_lossy().to_string()),
        );
        data.insert(
            "hostname".into(),
            serde_json::Value::String(
                hostname::get()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            ),
        );
        data.insert(
            "os".into(),
            serde_json::Value::String(std::env::consts::OS.to_string()),
        );

        let mut deployer =
            dwell_deploy::Deployer::new(home.clone(), &state_path, false)?.with_force(true);
        let results =
            deployer.apply_all(&entries, &source_dir, &serde_json::Value::Object(data))?;
        deployer.save_state(&state_path)?;

        let applied = results
            .iter()
            .filter(|r| r.action != dwell_core::ApplyAction::Skipped)
            .count();
        out.success(&format!("  {} entries applied", applied));
    } else {
        out.info("  (dry-run — would apply dotfiles)");
    }

    // Phase 4: Sync to remote
    out.step("Phase 4/4", "Syncing to remote...");
    if !dry_run {
        let git_dir = source_dir.join(".git");
        if git_dir.exists() {
            if let Ok(repo) = dwell_store::GitRepo::open(&git_dir) {
                if repo.has_changes().unwrap_or(false) {
                    repo.add_all().ok();
                    match repo.commit("dwell: deploy") {
                        Ok(id) => {
                            out.success(&format!("  Committed: {}", &id[..8]));
                            if repo.has_remote().unwrap_or(false) {
                                repo.push().ok();
                                out.success("  Pushed to remote");
                            } else {
                                out.info("  No remote configured — skipped push");
                            }
                        }
                        Err(_) => out.info("  Nothing to commit"),
                    }
                } else {
                    out.info("  No changes to sync");
                }
            }
        }
    } else {
        out.info("  (dry-run — would commit and push)");
    }

    out.title("deploy complete");
    Ok(())
}
