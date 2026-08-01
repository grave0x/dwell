use std::path::PathBuf;

use dwell_plugin::PluginRegistry;

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    action: crate::PluginCommand,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet, cli.log_level.clone());
    let registry = PluginRegistry::new();

    match action {
        crate::PluginCommand::Install { source } => {
            let source_path = PathBuf::from(&source);
            let installed = registry.install_from_source(&source_path)?;
            out.success(&format!(
                "Installed plugin {} {} ({:?})",
                installed.manifest.name, installed.manifest.version, installed.manifest.kind
            ));
            out.info(&format!("Plugin registry: {}", registry.root().display()));
            Ok(())
        }
        crate::PluginCommand::List => {
            let plugins = registry.list()?;
            if plugins.is_empty() {
                out.info("No plugins installed.");
                return Ok(());
            }
            out.info(&format!("Installed plugins ({})", plugins.len()));
            for plugin in plugins {
                out.info(&format!(
                    "  {} {} [{:?}] -> {}",
                    plugin.manifest.name,
                    plugin.manifest.version,
                    plugin.manifest.kind,
                    plugin.manifest.entry
                ));
            }
            Ok(())
        }
        crate::PluginCommand::Init { name } => {
            let dir = registry.scaffold(&name)?;
            out.success(&format!("Initialized plugin scaffold at {}", dir.display()));
            Ok(())
        }
    }
}
