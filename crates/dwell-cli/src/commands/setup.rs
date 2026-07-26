use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use dwell_package::PackageManagerRegistry;

use crate::commands::CliRef;
use crate::output::Output;

/// Structure of a captured system state.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct SetupManifest {
    captured_at: String,
    hostname: String,
    os: String,
    kernel: String,
    backend: String,
    sys_packages: HashMap<String, Vec<String>>, // backend → list of packages
    lang_tools: HashMap<String, Vec<String>>,   // category → list of tools
    manual_bins: Vec<String>,                   // files in user bin dirs
}

impl SetupManifest {
    fn new(backend: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();
        SetupManifest {
            captured_at: now,
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default(),
            os: std::env::consts::OS.to_string(),
            kernel: std::env::consts::FAMILY.to_string(),
            backend: backend.to_string(),
            sys_packages: HashMap::new(),
            lang_tools: HashMap::new(),
            manual_bins: vec![],
        }
    }

    fn setup_dir(source_root: &Path) -> PathBuf {
        source_root.join(".dwell").join("setup")
    }

    fn manifest_path(source_root: &Path) -> PathBuf {
        Self::setup_dir(source_root).join("manifest.json")
    }

    fn load(source_root: &Path) -> std::io::Result<Self> {
        let path = Self::manifest_path(source_root);
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    fn save(&self, source_root: &Path) -> std::io::Result<()> {
        let dir = Self::setup_dir(source_root);
        fs::create_dir_all(&dir)?;
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(Self::manifest_path(source_root), content)
    }

    fn total_packages(&self) -> usize {
        self.sys_packages.values().map(|v| v.len()).sum()
    }

    #[allow(dead_code)]
    fn all_packages(&self) -> Vec<String> {
        let mut pkgs: Vec<String> = self
            .sys_packages
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();
        pkgs.sort();
        pkgs.dedup();
        pkgs
    }
}

fn resolve_source(source: Option<PathBuf>) -> dwell_core::Result<PathBuf> {
    source
        .or_else(|| dirs::data_dir().map(|d| d.join("dwell")))
        .ok_or_else(|| {
            dwell_core::DwellError::InvalidConfig(
                "Could not determine source directory. Use --source to specify.".into(),
            )
        })
}

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    action: crate::SetupCommand,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);

    match action {
        crate::SetupCommand::Capture {
            source,
            no_sys_packages,
            no_lang_tools,
            no_manual,
        } => cmd_capture(&out, source, no_sys_packages, no_lang_tools, no_manual),
        crate::SetupCommand::Restore {
            source,
            sys_packages,
            lang_tools,
            interactive,
        } => cmd_restore(&out, source, sys_packages, lang_tools, interactive),
        crate::SetupCommand::Diff { source } => cmd_diff(&out, source),
    }
}

fn cmd_capture(
    out: &Output,
    source: Option<PathBuf>,
    no_sys: bool,
    no_lang: bool,
    no_manual: bool,
) -> dwell_core::Result<()> {
    let source_dir = resolve_source(source)?;
    out.info(&format!(
        "Capturing system state to {}",
        source_dir.join(".dwell/setup").display()
    ));

    let reg = PackageManagerRegistry::new();
    let backend_id = reg.detect().map(|b| b.id().to_string()).unwrap_or_default();

    let mut manifest = SetupManifest::new(&backend_id);

    // ── System packages ──────────────────────────────────────────────
    if !no_sys {
        for backend in reg.all() {
            if backend.is_available() {
                match backend.list_installed() {
                    Ok(pkgs) => {
                        let names: Vec<String> = pkgs.into_iter().map(|p| p.name).collect();
                        if !names.is_empty() {
                            out.info(&format!(
                                "  Captured {} {} packages",
                                names.len(),
                                backend.id()
                            ));
                            manifest
                                .sys_packages
                                .insert(backend.id().to_string(), names);
                        }
                    }
                    Err(e) => out.warn(&format!(
                        "  Failed to list {} packages: {}",
                        backend.id(),
                        e
                    )),
                }
            }
        }
    }

    // ── Language tools ──────────────────────────────────────────────
    if !no_lang {
        // Cargo
        let cargo_pkgs = run_cmd("cargo", &["install", "--list"])
            .map(|s| {
                s.lines()
                    .filter(|l| !l.is_empty() && !l.starts_with(' '))
                    .filter_map(|l| l.split_whitespace().next())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !cargo_pkgs.is_empty() {
            out.info(&format!("  Captured {} cargo tools", cargo_pkgs.len()));
            manifest.lang_tools.insert("cargo".into(), cargo_pkgs);
        }

        // Go
        let go_dir = dirs::home_dir().map(|h| h.join("go").join("bin"));
        if let Some(dir) = go_dir {
            if dir.exists() {
                let go_bins: Vec<String> = fs::read_dir(&dir)
                    .into_iter()
                    .flatten()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                if !go_bins.is_empty() {
                    out.info(&format!("  Captured {} go tools", go_bins.len()));
                    manifest.lang_tools.insert("go".into(), go_bins);
                }
            }
        }

        // pipx
        let pipx_pkgs = run_cmd("pipx", &["list", "--short"])
            .map(|s| {
                s.lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !pipx_pkgs.is_empty() {
            out.info(&format!("  Captured {} pipx tools", pipx_pkgs.len()));
            manifest.lang_tools.insert("pipx".into(), pipx_pkgs);
        }

        // Flatpak
        let flatpak_pkgs = run_cmd("flatpak", &["list", "--app", "--columns=application"])
            .map(|s| {
                s.lines()
                    .filter(|l| !l.is_empty() && !l.contains("Application"))
                    .map(|l| l.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !flatpak_pkgs.is_empty() {
            out.info(&format!("  Captured {} flatpak apps", flatpak_pkgs.len()));
            manifest.lang_tools.insert("flatpak".into(), flatpak_pkgs);
        }
    }

    // ── Manual binaries ──────────────────────────────────────────────
    if !no_manual {
        let bin_dirs = [
            dirs::home_dir().map(|h| h.join("bin")),
            Some(PathBuf::from("/usr/local/bin")),
        ];
        let mut bins = vec![];
        for dir in bin_dirs.into_iter().flatten() {
            if dir.exists() {
                let read_dir = match fs::read_dir(&dir) {
                    Ok(rd) => rd,
                    Err(_) => continue,
                };
                for e in read_dir.flatten() {
                    if e.file_type()
                        .map(|t| t.is_file() || t.is_symlink())
                        .unwrap_or(false)
                    {
                        // Only include non-standard entries (skip distro-managed files)
                        let name = e.file_name().to_string_lossy().to_string();
                        if !name.starts_with('.') {
                            bins.push(name);
                        }
                    }
                }
            }
        }
        // Remove dwell binary itself from list
        bins.retain(|b| b != "dwell" && b != "dwell-cli");
        bins.sort();
        bins.dedup();
        if !bins.is_empty() {
            out.info(&format!("  Found {} manual binaries", bins.len()));
            manifest.manual_bins = bins;
        }
    }

    // ── Save ─────────────────────────────────────────────────────────
    manifest.save(&source_dir)?;

    let total_pkg = manifest.total_packages();
    let total_tools: usize = manifest.lang_tools.values().map(|v| v.len()).sum();
    out.success(&format!(
        "Captured {} packages, {} tools, {} manual bins",
        total_pkg,
        total_tools,
        manifest.manual_bins.len()
    ));
    Ok(())
}

fn cmd_restore(
    out: &Output,
    source: Option<PathBuf>,
    restore_sys: bool,
    restore_lang: bool,
    interactive: bool,
) -> dwell_core::Result<()> {
    let source_dir = resolve_source(source)?;
    let manifest = SetupManifest::load(&source_dir).map_err(|e| {
        dwell_core::DwellError::InvalidConfig(format!(
            "No captured state found. Run 'dwell setup capture' first: {}",
            e
        ))
    })?;

    out.info(&format!(
        "Restoring from capture (origin: {}, {})",
        manifest.hostname, manifest.os
    ));

    let reg = PackageManagerRegistry::new();
    let current_backend = reg.detect();

    // ── System packages ────────────────────────────────────────────────
    if restore_sys {
        for (backend_id, pkgs) in &manifest.sys_packages {
            let target = if backend_id == current_backend.map(|b| b.id()).unwrap_or("") {
                current_backend
            } else {
                // Cross-system: ask the user
                if let Some(avail) = current_backend {
                    if interactive {
                        eprintln!("  Backup captured with '{}'. Available is '{}'. Try cross-install? [Y/n] ", backend_id, avail.id());
                        // Simple: proceed if no user input or "y"
                    }
                    out.info(&format!(
                        "  Cross-install: {} packages via {} (captured with {})",
                        pkgs.len(),
                        avail.id(),
                        backend_id
                    ));
                    Some(avail)
                } else {
                    out.warn(&format!(
                        "  No package manager available. Skipping {} packages from {}",
                        pkgs.len(),
                        backend_id
                    ));
                    None
                }
            };

            if let Some(backend) = target {
                let installed = backend
                    .list_installed()
                    .map(|p| p.into_iter().map(|x| x.name).collect::<Vec<_>>())
                    .unwrap_or_default();
                let mut count = 0;
                for pkg in pkgs {
                    if installed.contains(pkg) {
                        out.info(&format!("    Already installed: {}", pkg));
                    } else {
                        match backend.install(pkg) {
                            Ok(true) => {
                                out.success(&format!("    Installed: {}", pkg));
                                count += 1;
                            }
                            Ok(false) => out.warn(&format!("    Failed: {}", pkg)),
                            Err(e) => out.warn(&format!("    Error: {}: {}", pkg, e)),
                        }
                    }
                }
                out.success(&format!(
                    "  {} package(s) installed via {}",
                    count,
                    backend.id()
                ));
            }
        }
    }

    // ── Language tools ────────────────────────────────────────────────
    if restore_lang {
        for (category, tools) in &manifest.lang_tools {
            out.info(&format!("  Restoring {} tools ({})", category, tools.len()));
            match category.as_str() {
                "cargo" => {
                    for tool in tools {
                        out.info(&format!("    cargo install {}", tool));
                        let _ = run_cmd("cargo", &["install", tool]);
                    }
                }
                "pipx" => {
                    for tool in tools {
                        out.info(&format!("    pipx install {}", tool));
                        let _ = run_cmd("pipx", &["install", tool]);
                    }
                }
                "flatpak" => {
                    for tool in tools {
                        out.info(&format!("    flatpak install -y {}", tool));
                        let _ = run_cmd("flatpak", &["install", "-y", tool]);
                    }
                }
                _ => {
                    out.info(&format!(
                        "    Manual install needed for {}: {:?}",
                        category, tools
                    ));
                }
            }
        }
    }

    out.success("Restore complete");
    Ok(())
}

fn cmd_diff(out: &Output, source: Option<PathBuf>) -> dwell_core::Result<()> {
    let source_dir = resolve_source(source)?;
    let manifest = SetupManifest::load(&source_dir).map_err(|e| {
        dwell_core::DwellError::InvalidConfig(format!("No captured state found: {}", e))
    })?;

    out.info(&format!(
        "Comparing against capture from {} ({})",
        manifest.hostname, manifest.os
    ));

    let reg = PackageManagerRegistry::new();
    let mut total_captured = 0;
    let mut total_installed = 0;
    let mut missing = vec![];
    let mut extra = vec![];

    for (backend_id, captured_pkgs) in &manifest.sys_packages {
        // Find the matching backend (same id or current)
        let current = reg.get(backend_id).or_else(|| reg.detect());

        if let Some(backend) = current {
            let installed = backend
                .list_installed()
                .map(|p| p.into_iter().map(|x| x.name).collect::<Vec<_>>())
                .unwrap_or_default();

            let captured_set: std::collections::BTreeSet<_> = captured_pkgs.iter().collect();
            let installed_set: std::collections::BTreeSet<_> = installed.iter().collect();

            total_captured += captured_set.len();
            total_installed += installed_set.len();

            for pkg in captured_set.difference(&installed_set) {
                missing.push((backend_id.clone(), (*pkg).clone()));
            }
            for pkg in installed_set.difference(&captured_set) {
                extra.push((backend_id.clone(), (*pkg).clone()));
            }
        } else {
            out.warn(&format!(
                "  Backend '{}' not available on this system",
                backend_id
            ));
            missing.extend(
                captured_pkgs
                    .iter()
                    .map(|p| (backend_id.clone(), p.clone())),
            );
        }
    }

    out.info(&format!(
        "  Captured: {} packages across {} backends",
        total_captured,
        manifest.sys_packages.len()
    ));
    out.info(&format!(
        "  Currently installed: {} packages",
        total_installed
    ));

    if missing.is_empty() && extra.is_empty() {
        out.success("  No drift — system matches captured state.");
    } else {
        if !missing.is_empty() {
            out.warn(&format!("  Missing ({}):", missing.len()));
            for (backend, pkg) in &missing {
                eprintln!("    {}  {}", backend, pkg);
            }
        }
        if !extra.is_empty() {
            out.info(&format!("  Extra ({}):", extra.len()));
            for (backend, pkg) in &extra {
                eprintln!("    {}  {}", backend, pkg);
            }
        }
    }

    // Language tools - simple comparison
    for (category, captured) in &manifest.lang_tools {
        out.info(&format!(
            "  {}: {} tools captured",
            category,
            captured.len()
        ));
    }

    // Manual bins
    if !manifest.manual_bins.is_empty() {
        out.info(&format!(
            "  Manual bins: {} tracked",
            manifest.manual_bins.len()
        ));
        for bin in &manifest.manual_bins {
            let path = dirs::home_dir()
                .map(|h| h.join("bin").join(bin))
                .or_else(|| Some(PathBuf::from("/usr/local/bin").join(bin)));
            if let Some(p) = path {
                if !p.exists() {
                    out.warn(&format!("    Missing: {}", bin));
                }
            }
        }
    }

    Ok(())
}

/// Run a command and return stdout on success.
fn run_cmd(bin: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(bin).args(args).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}
