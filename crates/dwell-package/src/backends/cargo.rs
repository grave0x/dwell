use std::process::Command;

use dwell_core::traits::{PackageInfo, PackageManager};
use dwell_core::Result;

pub struct CargoBackend;

impl Default for CargoBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CargoBackend {
    pub fn new() -> Self {
        CargoBackend
    }
}

impl PackageManager for CargoBackend {
    fn id(&self) -> &str {
        "cargo"
    }

    fn display_name(&self) -> &str {
        "Cargo (Rust)"
    }

    fn is_available(&self) -> bool {
        Command::new("cargo")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Parses `cargo install --list` output.
    ///
    /// Expected format:
    /// ```text
    /// ripgrep v14.1.0:
    ///     ripgrep v14.1.0 (checksum)
    /// bat v0.24.0:
    ///     bat v0.24.0
    /// ```
    ///
    /// Top-level lines are package entries: `name vX.Y.Z:`.
    /// Indented sub-package lines (e.g. `    clippy-driver v0.1.57`) are skipped.
    fn list_installed(&self) -> Result<Vec<PackageInfo>> {
        let output = Command::new("cargo")
            .args(["install", "--list"])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!(
                    "cargo install --list failed: {}",
                    e
                ))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        let lines = stdout.lines();

        for line in lines {
            // Skip empty lines and indented sub-package lines (e.g. "    clippy-driver v0.1.57")
            if line.is_empty() || line.starts_with(' ') {
                continue;
            }
            let line = line.trim();
            // Format: "pkg-name v0.1.0:"
            let name = line.split(' ').next().unwrap_or(line).to_string();
            let version = line
                .split(' ')
                .nth(1)
                .map(|s| s.trim_end_matches(':').to_string());
            packages.push(PackageInfo {
                name,
                version,
                description: None,
                repository: None,
                installed: true,
            });
        }

        Ok(packages)
    }

    fn install(&self, package: &str) -> Result<bool> {
        let output = Command::new("cargo")
            .args(["install", package])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("cargo install failed: {}", e))
            })?;

        Ok(output.status.success())
    }

    fn remove(&self, package: &str) -> Result<bool> {
        let output = Command::new("cargo")
            .args(["uninstall", package])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("cargo uninstall failed: {}", e))
            })?;

        Ok(output.status.success())
    }

    /// Parses `cargo search` output.
    ///
    /// Expected format:
    /// ```text
    /// serde = "1.0.0"    # Serialization framework
    /// tokio = "1.35.0"   # Async runtime
    /// ```
    ///
    /// Each line: `pkg-name = "version" # description`
    fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let output = Command::new("cargo")
            .args(["search", query])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("cargo search failed: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                // Format: pkg-name = "version" # description
                let name = line.split(" = ").next().unwrap_or(line).to_string();
                let description = line.split(" # ").nth(1).map(|s| s.trim().to_string());
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
