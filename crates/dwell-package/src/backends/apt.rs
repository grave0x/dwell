use std::process::Command;

use dwell_core::traits::{PackageInfo, PackageManager};
use dwell_core::Result;

pub struct AptBackend;

impl Default for AptBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AptBackend {
    pub fn new() -> Self {
        AptBackend
    }
}

impl PackageManager for AptBackend {
    fn id(&self) -> &str {
        "apt"
    }

    fn display_name(&self) -> &str {
        "APT (Debian/Ubuntu)"
    }

    fn is_available(&self) -> bool {
        Command::new("apt-get")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn list_installed(&self) -> Result<Vec<PackageInfo>> {
        let output = Command::new("apt")
            .args(["list", "--installed"])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("apt list failed: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for (i, line) in stdout.lines().enumerate() {
            if i == 0 {
                // Skip the first line: "Listing..."
                continue;
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Format: pkgname/arch version amd64 [...]
            if let Some(name) = line.split('/').next() {
                packages.push(PackageInfo {
                    name: name.to_string(),
                    version: None,
                    description: None,
                    repository: None,
                    installed: true,
                });
            }
        }

        Ok(packages)
    }

    fn install(&self, package: &str) -> Result<bool> {
        let output = Command::new("apt-get")
            .args(["-y", "install", package])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("apt install failed: {}", e))
            })?;

        Ok(output.status.success())
    }

    fn remove(&self, package: &str) -> Result<bool> {
        let output = Command::new("apt-get")
            .args(["-y", "remove", package])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("apt remove failed: {}", e))
            })?;

        Ok(output.status.success())
    }

    fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        let output = Command::new("apt-cache")
            .args(["search", query])
            .output()
            .map_err(|e| {
                dwell_core::DwellError::PackageManager(format!("apt-cache search failed: {}", e))
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| {
                let (name, description) = match line.split_once(" - ") {
                    Some((n, d)) => (n.to_string(), Some(d.to_string())),
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
