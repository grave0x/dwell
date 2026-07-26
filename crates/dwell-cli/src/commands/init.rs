use std::fs;
use std::path::PathBuf;

use crate::commands::{resolve_home, CliRef};
use crate::output::Output;
use dwell_core::DwellError;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    source: Option<PathBuf>,
    clone_url: Option<String>,
    _remote_url: Option<String>,
) -> dwell_core::Result<()> {
    let _ = _cfg;
    let _ = _remote_url;
    let out = Output::new(cli.json, cli.verbose, cli.quiet);

    let source_dir = source.unwrap_or_else(|| {
        dirs::data_dir()
            .map(|d| d.join("dwell"))
            .unwrap_or_else(|| PathBuf::from("dwell"))
    });

    if let Some(url) = clone_url {
        out.info(&format!("Cloning from {}", url));
        dwell_store::GitRepo::clone(&url, &source_dir)?;
        out.success("Repository cloned successfully");
        return Ok(());
    }

    // Create source directory
    fs::create_dir_all(&source_dir)?;
    out.info(&format!(
        "Initialized source directory: {}",
        source_dir.display()
    ));

    // Init git repo inside the source directory (regular repo, not bare)
    let git_dir = source_dir.join(".git");
    if !git_dir.exists() {
        let repo = git2::Repository::init(&source_dir)
            .map_err(|e| DwellError::Git(format!("Failed to init git repo: {}", e)))?;
        // Set default config
        let mut config = repo
            .config()
            .map_err(|e| DwellError::Git(format!("Failed to open config: {}", e)))?;
        config.set_str("status.showUntrackedFiles", "no").ok();
        drop(config);
        out.success("Initialized git repository");
    }

    // Set remote if provided
    if let Some(url) = _remote_url {
        let git_repo = dwell_store::GitRepo::open(&git_dir)?;
        git_repo.set_remote(&url)?;
        out.info(&format!("Set remote origin: {}", url));
    }

    // Create dwell.toml
    let config_dir = dirs::config_dir()
        .map(|d| d.join("dwell"))
        .unwrap_or_else(|| PathBuf::from("dwell-config"));
    fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("dwell.toml");
    if !config_path.exists() {
        let default_config = generate_default_config();
        fs::write(&config_path, default_config)?;
        out.info(&format!("Created config: {}", config_path.display()));
    }

    // Create .dwellignore
    let ignore_path = source_dir.join(".dwellignore");
    if !ignore_path.exists() {
        fs::write(
            &ignore_path,
            "# dwell ignore patterns\n# templates/\n# externals/\n",
        )?;
    }

    out.success("dwell is ready! Add files with `dwell add <path>`.");
    Ok(())
}

fn generate_default_config() -> String {
    let home = resolve_home();
    format!(
        r#"# dwell.toml — Unified dotfile configuration
[meta]
name = "dotfiles"
version = "0.1.0"

[data]
home = "{}"
hostname = "{}"
platform = "{}"

[modules]

[packages]
system = []

[secrets]
files = []

[scripts]
run_once = []
run_before = []
run_after = []

[plugins]
wasm = []
lua = []

[generations]
keep = 10
"#,
        home.display(),
        hostname::get()
            .unwrap_or_else(|_| "unknown".into())
            .to_string_lossy(),
        std::env::consts::OS,
    )
}
