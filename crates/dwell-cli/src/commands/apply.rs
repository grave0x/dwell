use std::path::{Path, PathBuf};

use crate::commands::{resolve_home, resolve_source, CliRef};
use crate::output::Output;
use dialoguer::Confirm;
use dwell_core::{hash_content, source_to_target, Entry, EntryKind};
use dwell_secrets::AgeBackend;
use dwell_store::{DeployState, GenerationManager};

pub fn run(
    cli: &CliRef,
    cfg: &dwell_core::Config,
    source: Option<PathBuf>,
    force: bool,
    interactive: bool,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet, cli.log_level.clone());
    let source_dir = resolve_source(cli, source)?;
    let home = resolve_home();

    out.info(&format!("Applying dotfiles from {}", source_dir.display()));

    let sd = dwell_store::SourceDir::open(&source_dir)?;
    let entries = sd.entries()?;
    out.info(&format!("Found {} entries", entries.len()));

    let state_path = dirs::data_dir()
        .map(|d| d.join("dwell").join("state.json"))
        .unwrap_or_else(|| PathBuf::from("/tmp/dwell-state.json"));

    let data = load_template_data(&home, cfg, &out)?;
    let state_before = DeployState::load(&state_path).unwrap_or_else(|_| DeployState::new());
    let entries = if interactive {
        filter_entries_for_interactive_apply(&entries, &source_dir, &home, &state_before, &out)?
    } else {
        entries
    };

    if entries.is_empty() {
        out.info("No entries selected for apply.");
        return Ok(());
    }

    if !cli.dry_run {
        let snapshot_targets: Vec<PathBuf> = entries
            .iter()
            .filter(|e| !e.kind.is_script() && e.kind != EntryKind::Directory)
            .map(|e| source_to_target(&e.source_path, &home))
            .collect();
        let generation_id = state_before.generation + 1;
        let _ = GenerationManager::new().capture_snapshot(generation_id, &snapshot_targets)?;
    }
    let mut deployer =
        dwell_deploy::Deployer::new(home.clone(), state_path.as_path(), cli.dry_run)?
            .with_force(force);

    let results = deployer.apply_all(&entries, &source_dir, &data)?;

    for result in &results {
        out.apply_result(result);
    }

    deployer.save_state(&state_path)?;
    if !cli.dry_run {
        let pruned = GenerationManager::new().prune(cfg.generations.keep)?;
        if pruned > 0 {
            out.info(&format!(
                "Pruned {} old generation snapshot(s), keep={}",
                pruned, cfg.generations.keep
            ));
        }
    }
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

fn load_template_data(
    home: &Path,
    cfg: &dwell_core::Config,
    out: &Output,
) -> dwell_core::Result<serde_json::Value> {
    let mut data = serde_json::Map::new();
    data.insert(
        "home".into(),
        serde_json::Value::String(home.to_string_lossy().to_string()),
    );
    data.insert(
        "hostname".into(),
        serde_json::Value::String(cfg.data.hostname.clone()),
    );
    data.insert(
        "os".into(),
        serde_json::Value::String(cfg.data.platform.clone()),
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
    for (key, value) in &cfg.data.extra {
        let json_value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        data.insert(key.clone(), json_value);
    }

    let mut secrets = serde_json::Map::new();
    for (key, value) in std::env::vars() {
        if let Some(secret_key) = key.strip_prefix("DWELL_SECRET_") {
            secrets.insert(
                secret_key.to_ascii_lowercase(),
                serde_json::Value::String(value),
            );
        }
    }

    let identity = super::secret_cmd::default_identity_path();
    for secret_path in &cfg.secrets.files {
        let path = expand_tilde(secret_path);
        let secret_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("secret")
            .trim_end_matches(".age")
            .to_string();

        if !path.exists() {
            out.warn(&format!("Secret file not found: {}", path.display()));
            continue;
        }

        let value = if path.extension().and_then(|e| e.to_str()) == Some("age") {
            if !identity.exists() {
                out.warn(&format!(
                    "Skipping encrypted secret {}: identity not found at {}",
                    path.display(),
                    identity.display()
                ));
                continue;
            }
            let backend = AgeBackend::new(identity.clone());
            let ciphertext = std::fs::read(&path).map_err(dwell_core::DwellError::Io)?;
            match dwell_core::traits::SecretBackend::decrypt(&backend, &secret_name, &ciphertext) {
                Ok(v) => String::from_utf8_lossy(&v).to_string(),
                Err(e) => {
                    out.warn(&format!("Failed to decrypt {}: {}", path.display(), e));
                    continue;
                }
            }
        } else {
            std::fs::read_to_string(&path).map_err(dwell_core::DwellError::Io)?
        };

        secrets.insert(
            secret_name,
            serde_json::Value::String(value.trim_end().to_string()),
        );
    }
    data.insert("secrets".into(), serde_json::Value::Object(secrets));

    Ok(serde_json::Value::Object(data))
}

fn filter_entries_for_interactive_apply(
    entries: &[Entry],
    source_root: &Path,
    home: &Path,
    state: &DeployState,
    out: &Output,
) -> dwell_core::Result<Vec<Entry>> {
    let mut selected = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.kind.is_script()
            || entry.kind == EntryKind::Directory
            || entry.kind == EntryKind::Remove
        {
            selected.push(entry.clone());
            continue;
        }

        let target = source_to_target(&entry.source_path, home);
        if !target.exists() || !target.is_file() {
            selected.push(entry.clone());
            continue;
        }

        let existing = std::fs::read(&target).map_err(dwell_core::DwellError::Io)?;
        let existing_hash = hash_content(&existing);
        let tracked_hash = state.get_hash(&target).unwrap_or("");
        if tracked_hash.is_empty() || tracked_hash == existing_hash {
            selected.push(entry.clone());
            continue;
        }

        let prompt = format!(
            "Conflict for {} (locally modified since last deploy). Overwrite?",
            target.display()
        );
        if Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .unwrap_or(false)
        {
            selected.push(entry.clone());
        } else {
            out.info(&format!("Skipping conflict: {}", target.display()));
        }
    }
    let _ = source_root;
    Ok(selected)
}

fn expand_tilde(input: &str) -> PathBuf {
    if input == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    }
    if let Some(rest) = input.strip_prefix("~/") {
        return dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/home"))
            .join(rest);
    }
    PathBuf::from(input)
}
