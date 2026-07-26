use std::process::Command;

use dwell_core::traits::{PackageInfo, PackageManager};
use dwell_core::Result;

pub struct PacmanBackend;

impl Default for PacmanBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PacmanBackend {
    pub fn new() -> Self {
        PacmanBackend
    }
}

impl PackageManager for PacmanBackend {
    fn id(&self) -> &str {
        "pacman"
    }

    fn display_name(&self) -> &str {
        "Pacman (Arch Linux)"
    }

    fn is_available(&self) -> bool {
        Command::new("pacman")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<PackageInfo>> {
        let output = Command::new("pacman").args(["-Q"]).output().map_err(|e| {
            dwell_core::DwellError::PackageManager(format!("pacman -Q failed: {}", e))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                let name = parts[0].to_string();
                let version = parts.get(1).map(|s| s.to_string());
                PackageInfo {
                    name,
                    version,
                    description: None,
                    repository: None,
                    installed: true,
                }
            })
            .collect();

        Ok(packages)
    }

    fn install(&self, package: &str) -> Result<bool> {
        let output = Command::new("pacman")
            .args(["-S", "--noconfirm", package])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("pacman install failed: {}", e))
            })?;

        Ok(output.status.success())
    }

    fn remove(&self, package: &str) -> Result<bool> {
        let output = Command::new("pacman")
            .args(["-R", "--noconfirm", package])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("pacman remove failed: {}", e))
            })?;

        Ok(output.status.success())
    }

    fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let output = Command::new("pacman")
            .args(["-Ss", query])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("pacman -Ss failed: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        let mut lines = stdout.lines().peekable();

        while let Some(line) = lines.next() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Format: repo/name version
            // Followed by a description line indented with a space
            let (name, version) = if let Some((full_name, ver)) = line.split_once(' ') {
                if let Some((_repo, pkg_name)) = full_name.split_once('/') {
                    (pkg_name.to_string(), Some(ver.to_string()))
                } else {
                    (full_name.to_string(), Some(ver.to_string()))
                }
            } else {
                (line.to_string(), None)
            };

            let description = lines.next().map(|d| d.trim().to_string());

            packages.push(PackageInfo {
                name,
                version,
                description,
                repository: None,
                installed: false,
            });
        }

        Ok(packages)
    }
}
