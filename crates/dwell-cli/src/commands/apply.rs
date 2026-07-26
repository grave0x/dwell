use std::path::{Path, PathBuf};

use crate::commands::{resolve_home, resolve_source, CliRef};
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    source: Option<PathBuf>,
    force: bool,
) -> dwell_core::Result<()> {
    let _ = _cfg;
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let source_dir = resolve_source(cli, source)?;
    let home = resolve_home();

    out.info(&format!("Applying dotfiles from {}", source_dir.display()));

    let sd = dwell_store::SourceDir::open(&source_dir)?;
    let entries = sd.entries()?;
    out.info(&format!("Found {} entries", entries.len()));

    let state_path = dirs::data_dir()
        .map(|d| d.join("dwell").join("state.json"))
        .unwrap_or_else(|| PathBuf::from("/tmp/dwell-state.json"));

    let data = load_template_data(&home);
    let mut deployer =
        dwell_deploy::Deployer::new(home.clone(), state_path.as_path(), cli.dry_run)?
            .with_force(force);

    let results = deployer.apply_all(&entries, &source_dir, &data)?;

    for result in &results {
        out.apply_result(result);
    }

    deployer.save_state(&state_path)?;
    let applied = results
        .iter()
        .filter(|r| r.action != dwell_core::ApplyAction::Skipped)
        .count();
    out.success(&format!(
        "{} entries applied ({} skipped)",
        applied,
        results.len() - applied
    ));

    Ok(())
}

fn load_template_data(home: &Path) -> serde_json::Value {
    let mut data = serde_json::Map::new();
    data.insert(
        "home".into(),
        serde_json::Value::String(home.to_string_lossy().to_string()),
    );
    data.insert(
        "hostname".into(),
        serde_json::Value::String(
            hostname::get()
                .unwrap_or_else(|_| "unknown".into())
                .to_string_lossy()
                .to_string(),
        ),
    );
    data.insert(
        "os".into(),
        serde_json::Value::String(std::env::consts::OS.to_string()),
    );
    data.insert(
        "arch".into(),
        serde_json::Value::String(std::env::consts::ARCH.to_string()),
    );

    // Platform detection
    let platform = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };
    data.insert(
        "platform".into(),
        serde_json::Value::String(platform.to_string()),
    );

    serde_json::Value::Object(data)
}
