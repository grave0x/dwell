use std::process::Command;

use dwell_core::traits::{PackageInfo, PackageManager};
use dwell_core::Result;

pub struct NixBackend;

impl Default for NixBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl NixBackend {
    pub fn new() -> Self {
        NixBackend
    }
}

impl PackageManager for NixBackend {
    fn id(&self) -> &str {
        "nix"
    }

    fn display_name(&self) -> &str {
        "Nix"
    }

    fn is_available(&self) -> bool {
        Command::new("nix")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<PackageInfo>> {
        let output = Command::new("nix")
            .args(["profile", "list"])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("nix profile list failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                // Format: legacyPackages.x86_64-linux.pkg-name ...
                let name = if let Some((_store_path, rest)) = line.split_once(" flake:") {
                    // Try to extract package name from flake reference
                    rest.trim().to_string()
                } else if let Some((_store_path, rest)) = line.split_once(" path:") {
                    rest.trim().to_string()
                } else {
                    // Fallback: use first whitespace-delimited token
                    line.split_whitespace().next().unwrap_or(line).to_string()
                };
                PackageInfo {
                    name,
                    version: None,
                    description: None,
                    repository: None,
                    installed: true,
                }
            })
            .collect();

        Ok(packages)
    }

    fn install(&self, package: &str) -> Result<bool> {
        let output = Command::new("nix")
            .args(["profile", "install", &format!("nixpkgs#{}", package)])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("nix profile install failed: {}", e)))?;

        Ok(output.status.success())
    }

    fn remove(&self, package: &str) -> Result<bool> {
        let output = Command::new("nix")
            .args(["profile", "remove", package])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("nix profile remove failed: {}", e)))?;

        Ok(output.status.success())
    }

    fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let output = Command::new("nix")
            .args(["search", "nixpkgs", query])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("nix search failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| {
                let l = l.trim();
                !l.is_empty() && l.starts_with('*')
            })
            .map(|line| {
                let line = line.trim_start_matches("* ");
                let (name, description) = match line.split_once(' ') {
                    Some((n, d)) => (n.to_string(), Some(d.trim().to_string())),
                    None => (line.to_string(), None),
                };
                PackageInfo {
                    name,
                    version: None,
                    description,
                    repository: None,
                    installed: false,
                }
            })
            .collect();

        Ok(packages)
    }
}
