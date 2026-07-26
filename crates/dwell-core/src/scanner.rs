use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::deps::{DepEntry, DepsConfig};
use super::Result;

/// Analyze a single source entry for dependency clues.
fn analyze_entry(_source_path: &str, content: &str) -> DepEntry {
    let mut requires = Vec::new();
    let mut sources = Vec::new();
    let mut tools = Vec::new();

    // 1. Check shebang
    if let Some(first_line) = content.lines().next() {
        let interpreter = match first_line {
            l if l.contains("bash") => Some("bash"),
            l if l.contains("zsh") => Some("zsh"),
            l if l.contains("fish") => Some("fish"),
            l if l.contains("python3") || l.contains("python") => Some("python3"),
            l if l.contains("lua") => Some("lua"),
            l if l.contains("node") => Some("nodejs"),
            l if l.contains("perl") => Some("perl"),
            _ => None,
        };
        if let Some(pkg) = interpreter {
            requires.push(pkg.to_string());
        }
    }

    // 2. Check for `source` / `.` calls in shell scripts
    for line in content.lines() {
        let trimmed = line.trim();
        // source foo or . foo
        if let Some(sourced) = trimmed
            .strip_prefix("source ")
            .or_else(|| trimmed.strip_prefix(". "))
        {
            let path = sourced.trim_matches(&['"', '\''][..]).to_string();
            // Only track if it references another source entry (relative path)
            if path.starts_with("$HOME/.config/") || path.starts_with("~/.config/") {
                sources.push(path);
            }
            // Common tools referenced by scripts
            for known_tool in &["starship", "oh-my-posh", "zoxide", "fzf", "direnv"] {
                if trimmed.contains(known_tool) {
                    tools.push(known_tool.to_string());
                }
            }
        }
    }

    // 3. Check for command references in aliases and PATH
    for line in content.lines() {
        let trimmed = line.trim();
        // Detect alias references
        if trimmed.starts_with("alias ") {
            for known_tool in &[
                "bat", "lsd", "eza", "ripgrep", "fd", "fzf", "zoxide", "direnv",
            ] {
                if trimmed.contains(known_tool) {
                    tools.push(known_tool.to_string());
                }
            }
        }
        // Detect `command -v` checks
        if trimmed.contains("command -v") || trimmed.contains("which ") {
            if let Some(cmd) = trimmed.split_whitespace().last() {
                if !cmd.contains('$') && !cmd.contains('"') {
                    tools.push(cmd.to_string());
                }
            }
        }
        // Detect has/require binary checks
        if trimmed.contains("has ") || trimmed.contains("require ") {
            for known_tool in &[
                "bat", "lsd", "eza", "rg", "fd", "fzf", "zoxide", "direnv", "starship",
            ] {
                if trimmed.contains(known_tool) {
                    tools.push(known_tool.to_string());
                }
            }
        }
    }

    // Deduplicate
    requires.sort();
    requires.dedup();
    sources.sort();
    sources.dedup();
    tools.sort();
    tools.dedup();

    DepEntry {
        requires,
        sources,
        tools,
        custom: vec![],
    }
}

/// Scan a source directory and generate/update deps.toml.
pub fn scan_source(source_root: &Path) -> Result<DepsConfig> {
    let mut deps_map = HashMap::new();

    // Walk source directory
    for entry in walkdir::WalkDir::new(source_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(source_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip .git, .dwell, hidden files, non-source entries
        if rel.starts_with('.') || rel.starts_with("target/") {
            continue;
        }

        // Read content
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let entry = analyze_entry(&rel, &content);
        if !entry.requires.is_empty() || !entry.sources.is_empty() || !entry.tools.is_empty() {
            deps_map.insert(rel, entry);
        }
    }

    Ok(DepsConfig { deps: deps_map })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_shebang_bash() {
        let entry = analyze_entry("dot_bashrc", "#!/usr/bin/env bash\nalias ll='ls -la'\n");
        assert!(entry.requires.contains(&"bash".to_string()));
    }

    #[test]
    fn test_analyze_shebang_python() {
        let entry = analyze_entry(
            "dot_config/nvim/init.lua",
            "#!/usr/bin/env python3\nprint('hello')\n",
        );
        assert!(entry.requires.contains(&"python3".to_string()));
    }

    #[test]
    fn test_analyze_source_command() {
        let content = "source $HOME/.config/fish/config.fish\nstarship init\n";
        let entry = analyze_entry("dot_zshrc", content);
        assert!(!entry.sources.is_empty() || !entry.tools.is_empty());
    }

    #[test]
    fn test_analyze_alias_tools() {
        let content = "alias cat='bat'\nalias ls='eza --icons'\n";
        let entry = analyze_entry("dot_bashrc", content);
        assert!(entry.tools.contains(&"bat".to_string()));
        assert!(entry.tools.contains(&"eza".to_string()));
    }

    #[test]
    fn test_empty_config() {
        let deps = DepsConfig::default();
        assert!(deps.all_requires().is_empty());
        assert!(deps.all_tools().is_empty());
    }

    #[test]
    fn test_deps_aggregation() {
        let mut deps = DepsConfig::default();
        deps.deps.insert(
            "dot_bashrc".into(),
            DepEntry {
                requires: vec!["bash".into(), "git".into()],
                sources: vec![],
                tools: vec!["starship".into()],
                custom: vec![],
            },
        );
        deps.deps.insert(
            "dot_config/nvim/init.lua".into(),
            DepEntry {
                requires: vec!["neovim".into()],
                sources: vec![],
                tools: vec!["ripgrep".into()],
                custom: vec![],
            },
        );
        let all = deps.all_requires();
        assert!(all.contains(&"bash".to_string()));
        assert!(all.contains(&"git".to_string()));
        assert!(all.contains(&"neovim".to_string()));
    }
}
