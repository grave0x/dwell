use std::path::PathBuf;

use dwell_core::deps::DepsConfig;
use dwell_core::scanner;

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    action: crate::DepsCommand,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);

    match action {
        crate::DepsCommand::Scan { source } => cmd_scan(&out, source),
        crate::DepsCommand::Install { source, tools } => cmd_install(&out, source, tools),
        crate::DepsCommand::Audit { source } => cmd_audit(&out, source),
    }
}

fn resolve_source(source: Option<PathBuf>) -> dwell_core::Result<PathBuf> {
    source.or_else(|| {
        dirs::data_dir().map(|d| d.join("dwell"))
    }).ok_or_else(|| dwell_core::DwellError::InvalidConfig(
        "Could not determine source directory. Use --source to specify.".into()
    ))
}

fn cmd_scan(out: &Output, source: Option<PathBuf>) -> dwell_core::Result<()> {
    let source_dir = resolve_source(source)?;
    out.info(&format!("Scanning {} for dependencies...", source_dir.display()));

    let deps = scanner::scan_source(&source_dir)?;
    let count = deps.deps.len();

    if count == 0 {
        out.warn("No dependencies detected. Create .dwell/deps.toml manually if needed.");
        return Ok(());
    }

    // Save
    deps.save(&source_dir)?;
    out.success(&format!("Scanned {} entries, generated .dwell/deps.toml", count));

    // Summary
    for (entry, dep) in &deps.deps {
        let mut parts = vec![];
        if !dep.requires.is_empty() {
            parts.push(format!("needs: {}", dep.requires.join(", ")));
        }
        if !dep.sources.is_empty() {
            parts.push(format!("sources: {}", dep.sources.join(", ")));
        }
        if !dep.tools.is_empty() {
            parts.push(format!("tools: {}", dep.tools.join(", ")));
        }
        if !parts.is_empty() {
            out.info(&format!("  {} — {}", entry, parts.join(" | ")));
        }
    }

    // Show aggregate
    let all_requires = deps.all_requires();
    let all_tools = deps.all_tools();
    if !all_requires.is_empty() {
        out.info(&format!("Packages needed: {}", all_requires.join(", ")));
    }
    if !all_tools.is_empty() {
        out.info(&format!("Tools needed: {}", all_tools.join(", ")));
    }

    Ok(())
}

fn cmd_install(
    out: &Output,
    source: Option<PathBuf>,
    include_tools: bool,
) -> dwell_core::Result<()> {
    let source_dir = resolve_source(source)?;
    let deps = DepsConfig::load(&source_dir)
        .map_err(|e| dwell_core::DwellError::InvalidConfig(
            format!("Failed to load .dwell/deps.toml: {}", e)
        ))?;

    let requires = deps.all_requires();
    let mut targets = requires.clone();

    if include_tools {
        targets.extend(deps.all_tools());
    }

    if targets.is_empty() {
        out.info("No dependencies to install. Run `dwell deps scan` first.");
        return Ok(());
    }

    // Use the package manager registry to install
    let reg = dwell_package::PackageManagerRegistry::new();
    let backend = reg.detect();

    match backend {
        Some(backend) => {
            out.info(&format!("Using package manager: {}", backend.display_name()));

            let installed = backend.list_installed()?;
            let installed_names: std::collections::HashSet<String> = installed
                .iter().map(|p| p.name.clone()).collect();

            let mut count = 0;
            for pkg in &targets {
                if installed_names.contains(pkg) {
                    out.info(&format!("Already installed: {}", pkg));
                } else {
                    out.info(&format!("Installing: {}", pkg));
                    backend.install(pkg)?;
                    out.success(&format!("Installed: {}", pkg));
                    count += 1;
                }
            }
            out.success(&format!("{} package(s) installed.", count));
        }
        None => {
            out.warn("No package manager found on this system.");
            out.info(&format!("Required packages: {}", targets.join(", ")));
        }
    }

    Ok(())
}

fn cmd_audit(out: &Output, source: Option<PathBuf>) -> dwell_core::Result<()> {
    let source_dir = resolve_source(source)?;
    let deps = DepsConfig::load(&source_dir)
        .unwrap_or_default();

    let requires = deps.all_requires();
    let tools = deps.all_tools();

    if requires.is_empty() && tools.is_empty() {
        out.info("No dependencies tracked. Run `dwell deps scan` first.");
        return Ok(());
    }

    out.info("Checking system for required tools...");

    let mut found = 0;
    let mut missing = 0;

    for tool in requires.iter().chain(tools.iter()) {
        let available = std::process::Command::new(tool)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if available {
            out.info(&format!("  ✓ {}", tool));
            found += 1;
        } else {
            out.warn(&format!("  ✗ {} — not found on PATH", tool));
            missing += 1;
        }
    }

    out.success(&format!("{} found, {} missing", found, missing));
    Ok(())
}
