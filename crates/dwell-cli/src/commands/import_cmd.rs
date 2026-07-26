use std::fs;
use std::path::Path;

use crate::commands::CliRef;
use crate::output::Output;

/// Known config directory names commonly found in dotfiles repos.
const CONFIG_DIRS: &[&str] = &[
    "ags",
    "alacritty",
    "btop",
    "cava",
    "dunst",
    "fastfetch",
    "fish",
    "foot",
    "gtk-2.0",
    "gtk-3.0",
    "gtk-4.0",
    "hypr",
    "i3",
    "k9s",
    "kitty",
    "Kvantum",
    "mako",
    "mpd",
    "mpv",
    "neofetch",
    "nvim",
    "oh-my-posh",
    "picom",
    "polybar",
    "qtile",
    "ranger",
    "rofi",
    "starship",
    "sway",
    "swaylock",
    "tmux",
    "waybar",
    "wlogout",
    "wofi",
    "xfce4",
    "yazi",
    "zathura",
];

/// Directories to skip (wallpapers, backgrounds, large media, version control).
const SKIP_DIRS: &[&str] = &[
    "backgrounds",
    "wallpapers",
    "wallpaper",
    "wall",
    "Pictures",
    "Screenshots",
    "previews",
    ".git",
    ".github",
    ".previews",
];

/// File patterns at repo root that look like home-relative dotfiles.
fn is_home_dotfile(name: &str) -> bool {
    name.starts_with('.')
        && !name.starts_with(".git")
        && !name.starts_with(".github")
        && !name.starts_with(".previews")
}

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    url: String,
    all: bool,
    dry_run: bool,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet, None);
    let source_dir = crate::commands::resolve_source(cli, None)?;

    out.title(&format!("Importing: {}", url));

    // Extract repo name from URL
    let repo_name = url
        .trim_end_matches(".git")
        .split('/')
        .last()
        .unwrap_or("dotfiles")
        .to_string();

    // Clone to temp directory
    let tmp_dir = std::env::temp_dir().join(format!("dwell-import-{}", repo_name));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).ok();
    }

    out.info(&format!("Cloning into temporary directory..."));
    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", &url, tmp_dir.to_str().unwrap()])
        .status()
        .map_err(|e| dwell_core::DwellError::Other(format!("Failed to clone: {}", e)))?;

    if !status.success() {
        return Err(dwell_core::DwellError::Other("Clone failed".into()));
    }

    out.success("Repository cloned");

    // Detect structure
    let structure = detect_structure(&tmp_dir);
    out.info(&format!("Detected structure: {}", structure.description()));

    // Discover files to import
    let candidates = discover_files(&tmp_dir, &structure, all);
    if candidates.is_empty() {
        out.warn("No importable files found.");
        fs::remove_dir_all(&tmp_dir).ok();
        return Ok(());
    }

    out.info(&format!("Found {} file(s) to import", candidates.len()));

    // Show preview
    if cli.verbose > 0 || dry_run {
        for c in &candidates {
            out.path(&c.repo_path);
        }
    }

    if dry_run {
        out.info("Dry-run — no files copied.");
        fs::remove_dir_all(&tmp_dir).ok();
        return Ok(());
    }

    // Copy files into dwell source directory
    let mut copied = 0;
    let mut skipped = 0;
    for c in &candidates {
        let src = tmp_dir.join(&c.repo_path);
        let dst = source_dir.join(&c.dwell_path);

        if dst.exists() {
            if !all {
                out.info(&format!("Skipping existing: {}", &c.dwell_path));
                skipped += 1;
                continue;
            }
            // When --all, overwrite
        }

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).ok();
        }

        match fs::copy(&src, &dst) {
            Ok(_) => {
                if cli.verbose > 0 {
                    out.success(&format!("  {}", &c.dwell_path));
                }
                copied += 1;
            }
            Err(e) => {
                out.warn(&format!("  Failed: {}: {}", &c.dwell_path, e));
            }
        }
    }

    // Stage in git
    if let Ok(repo) = dwell_store::GitRepo::open(&source_dir.join(".git")) {
        repo.add_all().ok();
        let msg = format!("dwell: import {} — {} files", repo_name, copied);
        repo.commit(&msg).ok();
        out.success(&format!("Committed {} file(s)", copied));
    }

    out.success(&format!(
        "Import complete: {} copied, {} skipped",
        copied, skipped
    ));

    // Cleanup
    fs::remove_dir_all(&tmp_dir).ok();

    // Summary
    out.info(&format!(
        "Run 'dwell apply' to deploy, or 'dwell diff' to preview"
    ));
    Ok(())
}

/// Describes the layout structure of a dotfiles repo.
enum RepoStructure {
    /// Direct .config/ subdirectories at root (garuda-hyprdots style)
    DirectConfig,
    /// Files in a home-configs/ or configs/ subdirectory
    HomeConfigsDir,
    /// Files in a dotfiles/ subdirectory
    DotfilesDir,
    /// Home-relative dotfiles at root (.zshrc, .bashrc etc.)
    HomeRoot,
}

impl RepoStructure {
    fn description(&self) -> &str {
        match self {
            RepoStructure::DirectConfig => "config/ directories at repo root",
            RepoStructure::HomeConfigsDir => "home-configs/ or configs/ subdirectories",
            RepoStructure::DotfilesDir => "dotfiles/ subdirectory",
            RepoStructure::HomeRoot => "home-relative dotfiles at root",
        }
    }
}

fn detect_structure(repo_root: &Path) -> RepoStructure {
    let has_config_subdirs = CONFIG_DIRS.iter().any(|d| repo_root.join(d).is_dir());
    let has_dotfiles_at_root = fs::read_dir(repo_root)
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                is_home_dotfile(&name) && !e.file_type().map(|t| t.is_dir()).unwrap_or(false)
            })
        })
        .unwrap_or(false);

    let home_configs = ["home-configs", "configs", "home_configs", "Configs"];
    let has_home_configs_dir = home_configs.iter().any(|d| repo_root.join(d).is_dir());

    let has_dotfiles_dir = repo_root.join("dotfiles").is_dir();

    match (
        has_config_subdirs,
        has_home_configs_dir,
        has_dotfiles_dir,
        has_dotfiles_at_root,
    ) {
        (true, _, _, _) => RepoStructure::DirectConfig,
        (_, true, _, _) => RepoStructure::HomeConfigsDir,
        (_, _, true, _) => RepoStructure::DotfilesDir,
        (_, _, _, true) => RepoStructure::HomeRoot,
        _ => {
            // Check if ANY subdirs exist with config-like names
            if has_config_looking_dirs(repo_root) {
                RepoStructure::DirectConfig
            } else {
                RepoStructure::HomeRoot
            }
        }
    }
}

fn has_config_looking_dirs(repo_root: &Path) -> bool {
    fs::read_dir(repo_root)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .take(10)
                .count()
        })
        .unwrap_or(0)
        > 2
}

#[derive(Debug)]
struct ImportCandidate {
    repo_path: String,
    dwell_path: String,
}

fn discover_files(repo_root: &Path, structure: &RepoStructure, _all: bool) -> Vec<ImportCandidate> {
    let mut candidates = vec![];

    match structure {
        RepoStructure::DirectConfig => {
            // Each top-level dir maps to dot_config/<name>/
            for dir in CONFIG_DIRS {
                let dir_path = repo_root.join(dir);
                if dir_path.is_dir() {
                    // Check for doubled dir: name/name/
                    let doubled = dir_path.join(dir);
                    let source_dir = if doubled.is_dir() {
                        &doubled
                    } else {
                        &dir_path
                    };
                    collect_dir(source_dir, &format!("dot_config/{}", dir), &mut candidates);
                }
            }
            // Also collect any other non-skipped directories
            if let Ok(entries) = fs::read_dir(repo_root) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && !name.starts_with('.')
                        && !SKIP_DIRS.contains(&name.as_str())
                        && !CONFIG_DIRS.contains(&name.as_str())
                    {
                        collect_dir(
                            &entry.path(),
                            &format!("dot_config/{}", name),
                            &mut candidates,
                        );
                    }
                }
            }
        }
        RepoStructure::HomeConfigsDir => {
            for cfg_dir_name in &["home-configs", "configs", "home_configs", "Configs"] {
                let cfg_dir = repo_root.join(cfg_dir_name);
                if cfg_dir.is_dir() {
                    // Collect files, then convert dotfile names to dwell convention
                    let mut raw = vec![];
                    collect_dir(&cfg_dir, "", &mut raw);
                    for c in raw {
                        let dwell_name = if c.dwell_path.starts_with('.') {
                            format!("dot_{}", &c.dwell_path[1..])
                        } else {
                            c.dwell_path
                        };
                        candidates.push(ImportCandidate {
                            repo_path: c.repo_path,
                            dwell_path: dwell_name,
                        });
                    }
                }
            }
        }
        RepoStructure::DotfilesDir => {
            collect_dir(&repo_root.join("dotfiles"), "", &mut candidates);
            // Also check for home dotfiles in dotfiles/home/ or dotfiles/config/
            for sub in &["home", "config"] {
                let sub_path = repo_root.join("dotfiles").join(sub);
                if sub_path.is_dir() {
                    collect_dir(&sub_path, "", &mut candidates);
                }
            }
        }
        RepoStructure::HomeRoot => {
            if let Ok(entries) = fs::read_dir(repo_root) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.file_type().map(|t| t.is_file()).unwrap_or(false)
                        && is_home_dotfile(&name)
                    {
                        // .zshrc → dot_zshrc
                        let dwell_name = format!("dot_{}", &name[1..]);
                        candidates.push(ImportCandidate {
                            repo_path: name.clone(),
                            dwell_path: dwell_name,
                        });
                    }
                }
            }
        }
    }

    candidates
}

/// Recursively collect files from a source directory into the dwell source.
fn collect_dir(src_dir: &Path, prefix: &str, candidates: &mut Vec<ImportCandidate>) {
    if !src_dir.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(src_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip version control and media dirs, but NOT dotfiles
            // (home-configs directories contain .zshrc, .p10k.zsh etc.)
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            // Skip .git specifically
            if name == ".git" || name == ".github" {
                continue;
            }

            let rel = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };

            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                collect_dir(&entry.path(), &rel, candidates);
            } else {
                // Only import text/config files (skip large binaries)
                candidates.push(ImportCandidate {
                    repo_path: rel.clone(),
                    dwell_path: rel,
                });
            }
        }
    }
}
