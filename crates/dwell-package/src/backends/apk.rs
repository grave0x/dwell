use std::process::Command;

use dwell_core::traits::{PackageInfo, PackageManager};
use dwell_core::Result;

/// Package manager backend for Alpine Linux (apk).
pub struct ApkBackend;

impl Default for ApkBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ApkBackend {
    pub fn new() -> Self {
        ApkBackend
    }
}

impl PackageManager for ApkBackend {
    fn id(&self) -> &str {
        "apk"
    }

    fn display_name(&self) -> &str {
        "apk (Alpine Linux)"
    }

    fn is_available(&self) -> bool {
        Command::new("apk")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<PackageInfo>> {
        let output = Command::new("apk")
            .args(["info", "--installed"])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(
                format!("apk info failed: {}", e)
            ))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // apk info --installed output: "package-version"
            // Split on last '-' to separate name from version
            let name = line.to_string();
            let version = None; // Simplistic: version embedded in name string
            packages.push(PackageInfo {
                name,
                version,
                description: None,
                repository: Some("apk".into()),
                installed: true,
            });
        }

        Ok(packages)
    }

    fn install(&self, package: &str) -> Result<bool> {
        let status = Command::new("apk")
            .args(["add", package])
            .status()
            .map_err(|e| dwell_core::DwellError::PackageManager(
                format!("apk add failed: {}", e)
            ))?;
        Ok(status.success())
    }

    fn remove(&self, package: &str) -> Result<bool> {
        let status = Command::new("apk")
            .args(["del", package])
            .status()
            .map_err(|e| dwell_core::DwellError::PackageManager(
                format!("apk del failed: {}", e)
            ))?;
        Ok(status.success())
    }

    fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let output = Command::new("apk")
            .args(["search", query])
            .output()
            .map_err(|e| dwell_core::DwellError::PackageManager(
                format!("apk search failed: {}", e)
            ))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // apk search output: "package-version"
            packages.push(PackageInfo {
                name: line.to_string(),
                version: None,
                description: None,
                repository: Some("apk".into()),
                installed: false,
            });
        }

        Ok(packages)
    }
}
