//! Ignore pattern handling — .dwellignore and .chezmoiignore compatibility.

use std::path::Path;

/// Matches source paths against ignore patterns.
#[derive(Default)]
pub struct IgnorePatterns {
    patterns: Vec<Pattern>,
}

enum Pattern {
    Glob(String),
    Exact(String),
    Prefix(String),
    // Special chezmoi constructs
    TemplatesDir,
    ExternalsDir,
}

impl IgnorePatterns {
    /// Parse ignore patterns from a .dwellignore (or .chezmoiignore) file.
    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(IgnorePatterns { patterns: vec![] });
            }
            Err(e) => return Err(e),
        };
        Ok(Self::parse(&content))
    }

    pub fn parse(content: &str) -> Self {
        let mut patterns = vec![];
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            patterns.push(Self::classify(trimmed));
        }
        IgnorePatterns { patterns }
    }

    fn classify(raw: &str) -> Pattern {
        match raw {
            "{{ .templatesDir }}" | "templates/" => Pattern::TemplatesDir,
            "{{ .externalsDir }}" | "externals/" => Pattern::ExternalsDir,
            _ if raw.contains('*') || raw.contains('?') => Pattern::Glob(raw.to_string()),
            _ if raw.ends_with('/') => Pattern::Prefix(raw.trim_end_matches('/').to_string()),
            _ if raw.starts_with('/') => Pattern::Exact(raw[1..].to_string()),
            _ => Pattern::Exact(raw.to_string()),
        }
    }

    /// Check if a source-relative path should be ignored.
    pub fn is_ignored(&self, path: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(path))
    }
}

impl Pattern {
    fn matches(&self, path: &str) -> bool {
        match self {
            Pattern::Glob(g) => {
                let glob = g
                    .replace("**", ".*")
                    .replace('*', "[^/]*")
                    .replace('?', ".");
                regex::Regex::new(&format!("^{}$", glob))
                    .map(|re| re.is_match(path))
                    .unwrap_or(false)
            }
            Pattern::Exact(e) => path == e || path.starts_with(&format!("{}/", e)),
            Pattern::Prefix(p) => path.starts_with(p),
            Pattern::TemplatesDir => path.starts_with("templates/"),
            Pattern::ExternalsDir => path.starts_with("externals/"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_ignore() {
        let ig = IgnorePatterns::default();
        assert!(!ig.is_ignored("anything"));
    }

    #[test]
    fn test_exact_ignore() {
        let ig = IgnorePatterns::parse("README.md\n*.log\n");
        assert!(ig.is_ignored("README.md"));
        assert!(ig.is_ignored("foo.log"));
        assert!(!ig.is_ignored("bar.rs"));
    }

    #[test]
    fn test_glob_ignore() {
        let ig = IgnorePatterns::parse("*.tmp\n**/cache/**\n");
        assert!(ig.is_ignored("file.tmp"));
        assert!(ig.is_ignored("dir/cache/foo"));
        assert!(!ig.is_ignored("src/main.rs"));
    }
}
