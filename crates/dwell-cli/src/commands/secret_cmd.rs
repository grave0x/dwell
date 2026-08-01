use std::path::{Path, PathBuf};

use dwell_secrets::AgeBackend;

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    action: crate::SecretCommand,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet, cli.log_level.clone());

    match action {
        crate::SecretCommand::Encrypt { path } => cmd_encrypt(&out, &path),
        crate::SecretCommand::Decrypt { path } => cmd_decrypt(&out, &path),
        crate::SecretCommand::Keygen => cmd_keygen(&out),
    }
}

fn cmd_encrypt(out: &Output, path: &Path) -> dwell_core::Result<()> {
    let identity = default_identity_path();
    if !identity.exists() {
        return Err(dwell_core::DwellError::Encryption(format!(
            "Identity not found at {}. Run `dwell secret keygen` first.",
            identity.display()
        )));
    }

    let backend = AgeBackend::new(identity);
    let plaintext = std::fs::read(path).map_err(dwell_core::DwellError::Io)?;
    let ciphertext = dwell_core::traits::SecretBackend::encrypt(&backend, "", &plaintext)?;

    let out_path = PathBuf::from(format!("{}.age", path.to_string_lossy()));
    std::fs::write(&out_path, ciphertext).map_err(dwell_core::DwellError::Io)?;
    out.success(&format!("Encrypted to {}", out_path.display()));
    Ok(())
}

fn cmd_decrypt(out: &Output, path: &Path) -> dwell_core::Result<()> {
    let identity = default_identity_path();
    if !identity.exists() {
        return Err(dwell_core::DwellError::Encryption(format!(
            "Identity not found at {}. Run `dwell secret keygen` first.",
            identity.display()
        )));
    }

    let backend = AgeBackend::new(identity);
    let ciphertext = std::fs::read(path).map_err(dwell_core::DwellError::Io)?;
    let plaintext = dwell_core::traits::SecretBackend::decrypt(&backend, "", &ciphertext)?;

    let out_path = if path.extension().and_then(|e| e.to_str()) == Some("age") {
        path.with_extension("")
    } else {
        PathBuf::from(format!("{}.dec", path.to_string_lossy()))
    };

    std::fs::write(&out_path, plaintext).map_err(dwell_core::DwellError::Io)?;
    out.success(&format!("Decrypted to {}", out_path.display()));
    Ok(())
}

fn cmd_keygen(out: &Output) -> dwell_core::Result<()> {
    let identity = default_identity_path();
    if let Some(parent) = identity.parent() {
        std::fs::create_dir_all(parent).map_err(dwell_core::DwellError::Io)?;
    }

    AgeBackend::generate_identity(&identity)
        .map_err(|e| dwell_core::DwellError::Encryption(e.to_string()))?;
    out.success(&format!("Generated age identity: {}", identity.display()));
    Ok(())
}

pub fn default_identity_path() -> PathBuf {
    if let Ok(custom) = std::env::var("DWELL_AGE_IDENTITY") {
        return PathBuf::from(custom);
    }
    dirs::config_dir()
        .map(|d| d.join("dwell").join("age.key"))
        .unwrap_or_else(|| PathBuf::from("/tmp/dwell-age.key"))
}
