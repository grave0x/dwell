//! dwell-cli — Unified dotfile manager command-line interface.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod commands;
pub mod output;

/// dwell — unify your dotfiles across machines with type-safe modules,
/// cross-platform packages, and a plugin system.
#[derive(Parser)]
#[command(name = "dwell", version, about, long_about = None)]
pub struct Cli {
    /// Path to dwell.toml configuration file
    #[arg(short, long, default_value = "~/.config/dwell/dwell.toml")]
    pub config: PathBuf,

    /// Preview changes without applying
    #[arg(short = 'd', long)]
    pub dry_run: bool,

    /// Increase verbosity (use -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long)]
    pub quiet: bool,

    /// Machine-readable JSON output
    #[arg(long)]
    pub json: bool,

    /// Log file path (default: stderr)
    #[arg(long)]
    pub log_file: Option<PathBuf>,

    /// Log level override [possible values: error, warn, info, debug, trace]
    #[arg(long)]
    pub log_level: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Deploy dotfiles to target locations
    Apply {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Force overwrite even if content differs
        #[arg(short, long)]
        force: bool,
    },

    /// Show differences between source and filesystem
    Diff {
        #[arg(short, long)]
        source: Option<PathBuf>,
    },

    /// Initialize a new dwell configuration
    Init {
        /// Source directory path [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Clone from remote repository URL
        #[arg(long)]
        clone: Option<String>,

        /// Remote URL to set as origin (e.g. https://github.com/user/dotfiles.git)
        #[arg(long)]
        remote: Option<String>,
    },

    /// Add a file from the filesystem to the source directory
    Add {
        /// Path to add (relative to home or absolute)
        path: PathBuf,
    },

    /// Show deployment status — what's in sync, what's not
    Status {
        #[arg(short, long)]
        source: Option<PathBuf>,
    },

    /// Watch source directory and auto-apply on changes
    Watch {
        #[arg(short, long)]
        source: Option<PathBuf>,
    },

    /// Manage declared packages
    Package {
        #[command(subcommand)]
        action: PackageCommand,
    },

    /// Manage and validate modules
    Module {
        #[command(subcommand)]
        action: ModuleCommand,
    },

    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        action: PluginCommand,
    },

    /// Encrypt, decrypt, or manage secrets
    Secret {
        #[command(subcommand)]
        action: SecretCommand,
    },

    /// List, rollback, or manage generations
    Generation {
        #[command(subcommand)]
        action: GenerationCommand,
    },

    /// Show system health and configuration status
    Doctor,

    /// Commit and push changes to the configured remote
    Sync {
        /// Custom commit message [default: auto-generated]
        #[arg(short, long)]
        message: Option<String>,

        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Preview what would be synced without actually syncing
        #[arg(long)]
        dry_run: bool,
    },
    /// Manage dotfile dependencies (scan, install, audit, bundle)
    Deps {
        #[command(subcommand)]
        action: DepsCommand,
    },

    /// Capture, restore, or diff system packages and tools
    Setup {
        #[command(subcommand)]
        action: SetupCommand,
    },

    /// Full system setup: scan deps → install packages → apply dotfiles → sync
    Deploy {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Skip dependency installation
        #[arg(long)]
        no_deps: bool,

        /// Skip package restore from setup capture
        #[arg(long)]
        no_packages: bool,

        /// Preview without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Reset deployed dotfiles to source state
    Reset {
        /// Path to reset (file or directory relative to home)
        path: Option<PathBuf>,

        /// Reset all managed files
        #[arg(long)]
        all: bool,

        /// Remove targets not in source (cleanup orphans)
        #[arg(long)]
        stray: bool,

        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Preview without modifying
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completions for [possible values: bash, zsh, fish, powershell, elvish]
        shell: clap_complete::Shell,
    },

    /// Import dotfiles from a remote GitHub repository
    Import {
        /// Repository URL (e.g. https://github.com/user/dotfiles)
        url: String,

        /// Overwrite existing files without asking
        #[arg(long)]
        all: bool,

        /// Preview what would be imported without copying
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum DepsCommand {
    /// Auto-detect dependencies from source entries and generate deps.toml
    Scan {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,
    },

    /// Install required packages from deps.toml
    Install {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Also install declared tools (not just requires)
        #[arg(long)]
        tools: bool,
    },

    /// Check current system for missing required tools
    Audit {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum SetupCommand {
    /// Capture all installed packages and tools to a portable manifest
    Capture {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Skip system package enumeration
        #[arg(long)]
        no_sys_packages: bool,

        /// Skip language tool enumeration
        #[arg(long)]
        no_lang_tools: bool,

        /// Skip manual binary enumeration
        #[arg(long)]
        no_manual: bool,
    },

    /// Restore packages and tools from a captured manifest
    Restore {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,

        /// Restore system packages
        #[arg(long)]
        sys_packages: bool,

        /// Restore language tools
        #[arg(long)]
        lang_tools: bool,

        /// Ask before each step
        #[arg(short, long)]
        interactive: bool,
    },

    /// Compare current system against captured state
    Diff {
        /// Source directory [default: ~/.local/share/dwell]
        #[arg(short, long)]
        source: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum PackageCommand {
    /// Install declared packages
    Install,
    /// List installed vs declared packages
    List,
    /// Remove declared packages
    Remove,
    /// Show package drift (declared vs installed)
    Diff,
}

#[derive(Subcommand)]
pub enum ModuleCommand {
    /// List available modules
    List,
    /// Show module details
    Info { name: String },
    /// Validate module configuration
    Validate,
}

#[derive(Subcommand)]
pub enum PluginCommand {
    /// Install a plugin
    Install { name: String },
    /// List installed plugins
    List,
    /// Initialize a new plugin project
    Init { name: String },
}

#[derive(Subcommand)]
pub enum SecretCommand {
    /// Encrypt a file
    Encrypt { path: PathBuf },
    /// Decrypt a file
    Decrypt { path: PathBuf },
    /// Generate a new age identity
    Keygen,
}

#[derive(Subcommand)]
pub enum GenerationCommand {
    /// List all saved generations
    List,
    /// Roll back to a previous generation
    Rollback { id: Option<u64> },
    /// Delete old generations
    Prune { keep: Option<usize> },
}

fn main() {
    let cli = Cli::parse();

    // Determine log level
    let level = cli.log_level.clone().unwrap_or_else(|| {
        match cli.verbose {
            0 => if cli.quiet { "error" } else { "warn" },
            1 => "info",
            2 => "debug",
            _ => "trace",
        }.to_string()
    });

    // Configure tracing subscriber with optional file output
    let builder = tracing_subscriber::fmt()
        .with_env_filter(format!("dwell={}", level))
        .with_target(false)
        .without_time();

    if let Some(ref log_path) = cli.log_file {
        use std::fs::OpenOptions;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .unwrap_or_else(|e| {
                eprintln!("dwell: warning: cannot open log file {}: {}", log_path.display(), e);
                // Fallback to stderr using /dev/null as a dummy that won't matter
                std::fs::File::create("/dev/null").unwrap()
            });
        builder.with_writer(std::sync::Mutex::new(file)).init();
    } else {
        builder.with_writer(std::io::stderr).init();
    }

    if let Err(e) = commands::run(cli) {
        // Always print fatal errors to stderr regardless of log config
        eprintln!("dwell: error: {}", e);
        std::process::exit(1);
    }
}
