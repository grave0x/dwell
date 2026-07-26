use dwell_module::ModuleRegistry;

use crate::commands::CliRef;
use crate::output::Output;

pub fn run(
    cli: &CliRef,
    _cfg: &dwell_core::Config,
    action: crate::ModuleCommand,
) -> dwell_core::Result<()> {
    let out = Output::new(cli.json, cli.verbose, cli.quiet);
    let reg = ModuleRegistry::new();

    match action {
        crate::ModuleCommand::List => cmd_list(&out, &reg),
        crate::ModuleCommand::Info { name } => cmd_info(&out, &reg, &name),
        crate::ModuleCommand::Validate => cmd_validate(&out, &reg, _cfg),
    }
}

fn cmd_list(out: &Output, reg: &ModuleRegistry) -> dwell_core::Result<()> {
    let modules = reg.list();
    if modules.is_empty() {
        out.info("No modules registered.");
        return Ok(());
    }

    out.info(&format!("Available modules ({}):", modules.len()));
    for id in &modules {
        if let Some(m) = reg.get(id) {
            out.info(&format!("  {} — {}", id, m.description()));
        }
    }
    Ok(())
}

fn cmd_info(out: &Output, reg: &ModuleRegistry, name: &str) -> dwell_core::Result<()> {
    let module = reg.get(name).ok_or_else(|| {
        dwell_core::DwellError::Module(format!(
            "Unknown module: {}. Available: {}", name, reg.list().join(", ")
        ))
    })?;

    out.info(&format!("Module: {}", module.id()));
    out.info(&format!("Description: {}", module.description()));
    out.info(&format!("Options ({}):", module.options().len()));

    for opt in module.options() {
        let req = if opt.required { " [required]" } else { "" };
        let default = opt.default
            .as_ref()
            .map(|v| format!(" [default: {}]", v))
            .unwrap_or_default();
        let example = opt.example
            .as_ref()
            .map(|e| format!(" [e.g. {}]", e))
            .unwrap_or_default();
        out.info(&format!("  --{}: {}{}{}{}",
            opt.name, opt.description, req, default, example));
    }

    Ok(())
}

fn cmd_validate(out: &Output, reg: &ModuleRegistry, cfg: &dwell_core::Config) -> dwell_core::Result<()> {
    let validator = dwell_module::Validator::new();
    let module_config = &cfg.modules;

    if module_config.enabled.is_empty() {
        out.info("No module configuration found in dwell.toml.");
        return Ok(());
    }

    out.info("Validating module configuration...");
    let mut valid = 0;
    let mut errors = 0;

    for (key, values) in &module_config.enabled {
        // key like "programs.git" maps to module id
        if let Some(module) = reg.get(key) {
            let values_val = to_json_value(values);
            match validator.validate(module, &values_val) {
                Ok(_) => {
                    out.success(&format!("  ✓ {} — valid", key));
                    valid += 1;
                }
                Err(e) => {
                    out.warn(&format!("  ✗ {} — {}", key, e));
                    errors += 1;
                }
            }
        }
    }

    if errors == 0 {
        out.success(&format!("All {} module(s) valid.", valid));
    } else {
        out.warn(&format!("{} valid, {} with errors.", valid, errors));
    }

    Ok(())
}

fn to_json_value(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(i),
        toml::Value::Float(f) => serde_json::json!(f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(to_json_value).collect())
        }
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                map.insert(k.clone(), to_json_value(v));
            }
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(_) => serde_json::Value::String(value.to_string()),
    }
}
