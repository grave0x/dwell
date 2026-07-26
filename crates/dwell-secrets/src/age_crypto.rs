//! Age encryption/decryption — delegates to the `age` CLI tool.
//! Falls back to the `rage` (Rust age) crate for programmatic use.

use dwell_core::traits::SecretBackend;
use dwell_core::Result;

/// Age-based secret backend that delegates to the `age` CLI.
pub struct AgeBackend {
    identity_path: std::path::PathBuf,
}

impl AgeBackend {
    pub fn new(identity_path: std::path::PathBuf) -> Self {
        AgeBackend { identity_path }
    }

    /// Generate a new age identity by invoking `age-keygen`.
    pub fn generate_identity(path: &std::path::Path) -> std::io::Result<()> {
        let output = std::process::Command::new("age-keygen")
            .output()
            .or_else(|_| {
                // Try rage-keygen as fallback
                std::process::Command::new("rage-keygen").output()
            });

        match output {
            Ok(o) if o.status.success() => {
                std::fs::write(path, &o.stdout)?;
                Ok(())
            }
            Ok(o) => Err(std::io::Error::other(
                format!("age-keygen failed: {}", String::from_utf8_lossy(&o.stderr)),
            )),
            Err(_e) => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "age-keygen or rage-keygen not found. Install age or rage.",
            )),
        }
    }
}

impl SecretBackend for AgeBackend {
    fn id(&self) -> &str {
        "age"
    }

    fn get_secret(&self, _key: &str) -> Result<String> {
        Err(dwell_core::DwellError::Encryption(
            "age backend: use encrypt/decrypt for file-level operations".into(),
        ))
    }

    fn encrypt(&self, _key: &str, plaintext: &[u8]) -> Result<Vec<u8>> {
        // Use `age -e -i <identity>` or `rage -e -i <identity>`
        let tool = find_age_tool();
        let key_file = std::fs::read_to_string(&self.identity_path)
            .map_err(dwell_core::DwellError::Io)?;

        // Extract public key from identity file
        let public_key = key_file
            .lines()
            .find(|l| l.starts_with("# public key:"))
            .and_then(|l| l.strip_prefix("# public key:"))
            .map(|s| s.trim())
            .ok_or_else(|| dwell_core::DwellError::Encryption(
                "Could not find public key in identity file".into(),
            ))?;

        let mut child = std::process::Command::new(tool)
            .args(["-e", "-r", public_key])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| dwell_core::DwellError::Encryption(
                format!("Failed to spawn {}: {}", tool, e),
            ))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(plaintext)?;
        }

        let output = child.wait_with_output()?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(dwell_core::DwellError::Encryption(
                format!("age encrypt failed: {}", String::from_utf8_lossy(&output.stderr)),
            ))
        }
    }

    fn decrypt(&self, _key: &str, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let tool = find_age_tool();

        let mut child = std::process::Command::new(tool)
            .args(["-d", "-i"])
            .arg(&self.identity_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| dwell_core::DwellError::Encryption(
                format!("Failed to spawn {}: {}", tool, e),
            ))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(ciphertext)?;
        }

        let output = child.wait_with_output()?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(dwell_core::DwellError::Encryption(
                format!("age decrypt failed: {}", String::from_utf8_lossy(&output.stderr)),
            ))
        }
    }

    fn has_secret(&self, _key: &str) -> bool {
        self.identity_path.exists()
    }
}

fn find_age_tool() -> &'static str {
    for tool in &["rage", "age"] {
        if std::process::Command::new(tool)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return tool;
        }
    }
    "age" // default, will fail with a clear error
}
