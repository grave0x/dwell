use crate::Cli;

mod apply;
mod diff;
mod init;
mod add;
mod status;
mod watch;
mod package;
mod sync;
mod deps;
mod setup;

/// A cheap reference-like view of CLI options, avoiding partial moves.
pub struct CliRef {
    pub dry_run: bool,
    pub verbose: u8,
    pub quiet: bool,
    pub json: bool,
}

pub fn run(cli: Cli) -> dwell_core::Result<()> {
    let Cli { config: _config, dry_run, verbose, quiet, json, command } = cli;
    let cli_ref = CliRef { dry_run, verbose, quiet, json };
    let cfg = dwell_core::Config::load(&_config)?;

    match command {
        crate::Commands::Apply { source, force } => apply::run(&cli_ref, &cfg, source, force),
        crate::Commands::Diff { source } => diff::run(&cli_ref, &cfg, source),
        crate::Commands::Init { source, clone, remote } => init::run(&cli_ref, &cfg, source, clone, remote),
        crate::Commands::Add { path } => add::run(&cli_ref, &cfg, path),
        crate::Commands::Status { source } => status::run(&cli_ref, &cfg, source),
        crate::Commands::Watch { source } => watch::run(&cli_ref, &cfg, source),
        crate::Commands::Package { action } => package::run(&cli_ref, &cfg, action),
        crate::Commands::Deps { action } => deps::run(&cli_ref, &cfg, action),
        crate::Commands::Setup { action } => setup::run(&cli_ref, &cfg, action),
        crate::Commands::Module { .. } => {
            eprintln!("Module management — Phase 2 (coming soon)");
            Ok(())
        }
        crate::Commands::Plugin { .. } => {
            eprintln!("Plugin management — Phase 4 (coming soon)");
            Ok(())
        }
        crate::Commands::Secret { .. } => {
            eprintln!("Secret management — Phase 3 (coming soon)");
            Ok(())
        }
        crate::Commands::Generation { .. } => {
            eprintln!("Generation management — Phase 5 (coming soon)");
            Ok(())
        }
        crate::Commands::Doctor => {
            doctor::run(&cli_ref);
            Ok(())
        }
        crate::Commands::Sync { message, source, dry_run } => sync::run(&cli_ref, &cfg, message, source, dry_run),
    }
}

pub fn resolve_source(_cli: &CliRef, source: Option<std::path::PathBuf>) -> dwell_core::Result<std::path::PathBuf> {
    source.or_else(|| {
        dirs::data_dir().map(|d| d.join("dwell"))
    }).ok_or_else(|| dwell_core::DwellError::InvalidConfig(
        "Could not determine source directory. Use --source to specify.".into()
    ))
}

fn resolve_home() -> std::path::PathBuf {
    dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/home"))
}

mod doctor {
    use crate::commands::CliRef;

    pub fn run(_cli: &CliRef) {
        println!("dwell doctor — system health check");
        println!("================================");

        // Check XDG directories
        for (name, default) in &[
            ("XDG_CONFIG_HOME", dirs::config_dir()),
            ("XDG_CACHE_HOME", dirs::cache_dir()),
            ("XDG_DATA_HOME", dirs::data_dir()),
        ] {
            match default {
                Some(path) => println!("  ✓ {} = {}", name, path.display()),
                None => println!("  ✗ {} not set", name),
            }
        }

        // Check home
        match dirs::home_dir() {
            Some(home) => println!("  ✓ HOME = {}", home.display()),
            None => println!("  ✗ HOME not set"),
        }

        // Check git availability
        match std::process::Command::new("git").arg("--version").output() {
            Ok(o) if o.status.success() => {
                let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                println!("  ✓ git available: {}", ver);
            }
            _ => println!("  ✗ git not found"),
        }

        // Check existing dwell config
        let config_path = dirs::config_dir()
            .map(|d| d.join("dwell").join("dwell.toml"));
        match &config_path {
            Some(p) if p.exists() => println!("  ✓ dwell.toml found at {}", p.display()),
            Some(p) => println!("  - dwell.toml not yet created at {}", p.display()),
            None => println!("  ✗ Cannot determine config directory"),
        }

        // Check source directory
        let source_path = dirs::data_dir().map(|d| d.join("dwell"));
        match &source_path {
            Some(p) if p.exists() => println!("  ✓ Source directory exists at {}", p.display()),
            Some(p) => println!("  - Source directory not yet created at {}", p.display()),
            None => println!("  ✗ Cannot determine data directory"),
        }

        println!("\nRun `dwell init` to set up your dotfile repository.");
    }
}
