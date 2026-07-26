use crate::commands::CliRef;
use crate::output::Output;

/// Run package commands — dispatches to the appropriate sub-command.
pub fn run(
    cli: &CliRef,
    cfg: &dwell_core::Config,
    action: crate::PackageCommand,
) -> dwell_core::Result<()> {
    match action {
        crate::PackageCommand::Install => cmd_install(cli, cfg),
        crate::PackageCommand::List => cmd_list(cli, cfg),
        crate::PackageCommand::Remove => cmd_remove(cli, cfg),
        crate::PackageCommand::Diff => cmd_diff(cli, cfg),
    }
}

/// Install all declared system packages.
fn cmd_install(cli: &CliRef, cfg: &dwell_core::Config) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let registry = dwell_package::PackageManagerRegistry::new();

    let backend = registry.detect().ok_or_else(|| {
        dwell_core::DwellError::PackageManager(
            "No package manager detected (tried apt, pacman, brew, nix, cargo)".into(),
        )
    })?;

    out.info(&format!("Using backend: {}", backend.display_name()));

    let packages = &cfg.packages.system;
    if packages.is_empty() {
        out.info("No packages declared in config.");
        return Ok(());
    }

    let installed = backend.list_installed()?;
    let installed_names: Vec<&str> = installed.iter().map(|p| p.name.as_str()).collect();

    let mut installed_count = 0;
    let mut skipped_count = 0;

    for pkg in packages {
        if installed_names.contains(&pkg.as_str()) {
            out.info(&format!("  ~ {} (already installed)", pkg));
            skipped_count += 1;
            continue;
        }
        out.info(&format!("  + installing {}...", pkg));
        if !cli.dry_run {
            backend.install(pkg)?;
        }
        installed_count += 1;
    }

    out.success(&format!(
        "{} installed, {} skipped, {} total",
        installed_count,
        skipped_count,
        packages.len()
    ));

    Ok(())
}

/// List all installed packages per backend, marking declared ones with `*`.
fn cmd_list(cli: &CliRef, cfg: &dwell_core::Config) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let registry = dwell_package::PackageManagerRegistry::new();
    let declared: Vec<&str> = cfg.packages.system.iter().map(|s| s.as_str()).collect();

    for backend in registry.all() {
        if !backend.is_available() {
            continue;
        }
        out.info(&format!("[{}]", backend.display_name()));
        match backend.list_installed() {
            Ok(packages) => {
                for pkg in &packages {
                    let marker = if declared.contains(&pkg.name.as_str()) {
                        " *"
                    } else {
                        "  "
                    };
                    if cli.json {
                        // JSON output uses the Output helper
                        let _ = marker;
                    }
                    println!("  {}{}", pkg.name, marker);
                }
            }
            Err(e) => {
                out.warn(&format!("  Error listing: {}", e));
            }
        }
        println!();
    }

    Ok(())
}

/// Remove all declared packages via the first available backend.
fn cmd_remove(cli: &CliRef, cfg: &dwell_core::Config) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let registry = dwell_package::PackageManagerRegistry::new();

    let backend = registry.detect().ok_or_else(|| {
        dwell_core::DwellError::PackageManager("No package manager detected".into())
    })?;

    out.info(&format!("Using backend: {}", backend.display_name()));

    let packages = &cfg.packages.system;
    if packages.is_empty() {
        out.info("No packages declared in config.");
        return Ok(());
    }

    let installed = backend.list_installed()?;
    let installed_names: Vec<&str> = installed.iter().map(|p| p.name.as_str()).collect();

    let mut removed_count = 0;
    let mut skipped_count = 0;

    for pkg in packages {
        if !installed_names.contains(&pkg.as_str()) {
            out.info(&format!("  ~ {} (not installed)", pkg));
            skipped_count += 1;
            continue;
        }
        out.info(&format!("  - removing {}...", pkg));
        if !cli.dry_run {
            backend.remove(pkg)?;
        }
        removed_count += 1;
    }

    out.success(&format!(
        "{} removed, {} skipped, {} total",
        removed_count,
        skipped_count,
        packages.len()
    ));

    Ok(())
}

/// Show package drift — declared vs installed.
fn cmd_diff(cli: &CliRef, cfg: &dwell_core::Config) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let registry = dwell_package::PackageManagerRegistry::new();

    let backend = registry.detect().ok_or_else(|| {
        dwell_core::DwellError::PackageManager("No package manager detected".into())
    })?;

    out.info(&format!("Using backend: {}", backend.display_name()));

    let declared: Vec<&str> = cfg.packages.system.iter().map(|s| s.as_str()).collect();
    let installed = backend.list_installed()?;
    let installed_names: Vec<&str> = installed.iter().map(|p| p.name.as_str()).collect();

    let mut missing = Vec::new();
    let mut extra = Vec::new();

    for pkg in &declared {
        if !installed_names.contains(pkg) {
            missing.push(*pkg);
        }
    }

    for pkg in &installed_names {
        if !declared.contains(pkg) {
            extra.push(*pkg);
        }
    }

    if missing.is_empty() && extra.is_empty() {
        out.success("No drift — all declared packages are installed.");
        return Ok(());
    }

    if !missing.is_empty() {
        out.info("Missing (declared but not installed):");
        for pkg in &missing {
            println!("  + {}", pkg);
        }
    }

    if !extra.is_empty() {
        out.info("Extra (installed but not declared):");
        for pkg in &extra {
            println!("  - {}", pkg);
        }
    }

    Ok(())
}
