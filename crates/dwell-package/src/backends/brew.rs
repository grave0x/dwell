use std::process::Command;

use dwell_core::traits::{PackageInfo, PackageManager};
use dwell_core::Result;

pub struct BrewBackend;

impl Default for BrewBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl BrewBackend {
    pub fn new() -> Self {
        BrewBackend
    }
}

impl PackageManager for BrewBackend {
    fn id(&self) -> &str {
        "brew"
    }

    fn display_name(&self) -> &str {
        "Homebrew"
    }

    fn is_available(&self) -> bool {
        Command::new("brew")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<PackageInfo>> {
        let output = Command::new("brew")
            .args(["list", "--formula", "-1"])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("brew list failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|name| PackageInfo {
                name: name.to_string(),
                version: None,
                description: None,
                repository: None,
                installed: true,
            })
            .collect();

        Ok(packages)
    }

    fn install(&self, package: &str) -> Result<bool> {
        let output = Command::new("brew")
            .args(["install", package])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("brew install failed: {}", e)))?;

        Ok(output.status.success())
    }

    fn remove(&self, package: &str) -> Result<bool> {
        let output = Command::new("brew")
            .args(["uninstall", package])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("brew uninstall failed: {}", e)))?;

        Ok(output.status.success())
    }

    fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let output = Command::new("brew")
            .args(["search", query])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(format!("brew search failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| {
                let l = l.trim();
                !l.is_empty() && !l.starts_with("==>")
            })
            .map(|name| PackageInfo {
                name: name.trim().to_string(),
                version: None,
                description: None,
                repository: None,
                installed: false,
            })
            .collect();

        Ok(packages)
    }
}
