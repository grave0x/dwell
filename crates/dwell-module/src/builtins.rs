use dwell_core::traits::{Module, ModuleOption, OptionType};
use dwell_core::{Entry, EntryKind, Result};

/// Built-in module: programs.git — generates ~/.gitconfig from typed options.
pub struct GitModule;

impl Default for GitModule {
    fn default() -> Self {
        Self::new()
    }
}

impl GitModule {
    pub fn new() -> Self {
        GitModule
    }
}

impl Module for GitModule {
    fn id(&self) -> &str {
        "programs.git"
    }

    fn description(&self) -> &str {
        "Generate ~/.gitconfig from typed options"
    }

    fn options(&self) -> Vec<ModuleOption> {
        vec![
            ModuleOption {
                name: "user.name".into(),
                option_type: OptionType::String,
                default: None,
                description: "Git user name".into(),
                example: Some("John Doe".into()),
                required: true,
            },
            ModuleOption {
                name: "user.email".into(),
                option_type: OptionType::String,
                default: None,
                description: "Git user email".into(),
                example: Some("john@example.com".into()),
                required: true,
            },
            ModuleOption {
                name: "init.defaultBranch".into(),
                option_type: OptionType::String,
                default: Some(serde_json::json!("main")),
                description: "Default branch name for new repositories".into(),
                example: None,
                required: false,
            },
            ModuleOption {
                name: "core.editor".into(),
                option_type: OptionType::String,
                default: None,
                description: "Default text editor for git".into(),
                example: Some("nvim".into()),
                required: false,
            },
            ModuleOption {
                name: "core.pager".into(),
                option_type: OptionType::String,
                default: None,
                description: "Default pager".into(),
                example: Some("delta".into()),
                required: false,
            },
            ModuleOption {
                name: "alias".into(),
                option_type: OptionType::Dict(std::collections::HashMap::new()),
                default: None,
                description: "Git aliases as key-value pairs".into(),
                example: Some(r#"{"st": "status", "co": "checkout"}"#.into()),
                required: false,
            },
            ModuleOption {
                name: "diff.tool".into(),
                option_type: OptionType::String,
                default: None,
                description: "Diff tool to use".into(),
                example: Some("difftastic".into()),
                required: false,
            },
            ModuleOption {
                name: "merge.tool".into(),
                option_type: OptionType::String,
                default: None,
                description: "Merge tool to use".into(),
                example: Some("nvimdiff".into()),
                required: false,
            },
            ModuleOption {
                name: "credential.helper".into(),
                option_type: OptionType::String,
                default: Some(serde_json::json!("libsecret")),
                description: "Credential helper".into(),
                example: None,
                required: false,
            },
            ModuleOption {
                name: "signingkey".into(),
                option_type: OptionType::String,
                default: None,
                description: "GPG signing key".into(),
                example: Some("ABC123DEF".into()),
                required: false,
            },
            ModuleOption {
                name: "extra".into(),
                option_type: OptionType::List(Box::new(OptionType::String)),
                default: None,
                description: "Extra git config lines to include verbatim".into(),
                example: Some(r#"["url.git@github.com:.insteadOf = https://github.com/"]"#.into()),
                required: false,
            },
        ]
    }

    fn generate(&self, values: &serde_json::Value) -> Result<Vec<Entry>> {
        let obj = values.as_object().ok_or_else(|| {
            dwell_core::DwellError::Module("git module: values must be a JSON object".into())
        })?;

        let mut lines = vec![];
        lines.push("[user]".to_string());
        lines.push(format!("\tname = {}", get_str(obj, "user.name", "?")));
        lines.push(format!("\temail = {}", get_str(obj, "user.email", "?")));

        let sections = [
            ("init", &["defaultBranch"] as &[&str]),
            ("core", &["editor", "pager"]),
            ("diff", &["tool"]),
            ("merge", &["tool"]),
            ("credential", &["helper"]),
            ("user", &["signingkey"]),
        ];

        for (section, keys) in &sections {
            let mut section_lines = vec![];
            for key in *keys {
                let full_key = format!("{}.{}", section, key);
                if let Some(val) = obj.get(&full_key).and_then(|v| v.as_str()) {
                    section_lines.push(format!("\t{} = {}", key, val));
                }
            }
            if !section_lines.is_empty() {
                lines.push(format!("\n[{}]", section));
                lines.extend(section_lines);
            }
        }

        // Aliases
        if let Some(aliases) = obj.get("alias").and_then(|v| v.as_object()) {
            lines.push("\n[alias]".to_string());
            for (name, val) in aliases {
                if let Some(cmd) = val.as_str() {
                    lines.push(format!("\t{} = {}", name, cmd));
                }
            }
        }

        // Signing key (top-level fallback)
        if let Some(key) = obj.get("signingkey").and_then(|v| v.as_str()) {
            lines.push(format!("\nsigningkey = {}", key));
        }

        // Extra lines
        if let Some(extra) = obj.get("extra").and_then(|v| v.as_array()) {
            for line in extra {
                if let Some(s) = line.as_str() {
                    lines.push(s.to_string());
                }
            }
        }

        let content = lines.join("\n") + "\n";

        Ok(vec![Entry {
            source_path: "dot_gitconfig".into(),
            kind: EntryKind::File,
            content_hash: dwell_core::hash_content(content.as_bytes()),
            encrypted: false,
        }])
    }
}

fn get_str<'a>(
    obj: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
    default: &'a str,
) -> String {
    obj.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Try the short key (e.g. "name" instead of "user.name")
            let short = key.split('.').next_back().unwrap_or(key);
            obj.get(short)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or(default.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_config_generation() {
        let module = GitModule::new();
        let values = serde_json::json!({
            "user.name": "Alice",
            "user.email": "alice@example.com",
            "init.defaultBranch": "main",
            "core.editor": "nvim",
            "alias": {
                "st": "status",
                "co": "checkout"
            }
        });
        let entries = module.generate(&values).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_path, "dot_gitconfig");
        assert!(!entries[0].content_hash.is_empty());
    }

    #[test]
    fn test_git_config_minimal() {
        let module = GitModule::new();
        let values = serde_json::json!({
            "user.name": "Bob",
            "user.email": "bob@test.com"
        });
        let entries = module.generate(&values).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source_path, "dot_gitconfig");
    }

    #[test]
    fn test_options_count() {
        let module = GitModule::new();
        let opts = module.options();
        assert!(opts.len() >= 10, "Should have at least 10 options");
    }
}
