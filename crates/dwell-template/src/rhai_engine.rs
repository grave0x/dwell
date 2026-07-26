use dwell_core::traits::TemplateEngine;
use dwell_core::Result;

/// Rhai scripting template engine.
/// Note: rhai::Engine is not Send+Sync, so we recreate it per-operation.
pub struct RhaiEngine;

impl Default for RhaiEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiEngine {
    pub fn new() -> Self {
        RhaiEngine
    }

    fn make_engine() -> rhai::Engine {
        rhai::Engine::new()
    }
}

impl TemplateEngine for RhaiEngine {
    fn name(&self) -> &str {
        "rhai"
    }

    fn render(&self, template: &str, data: &serde_json::Value) -> Result<String> {
        let engine = Self::make_engine();
        let mut scope = rhai::Scope::new();

        // Inject top-level keys as scope variables
        if let Some(obj) = data.as_object() {
            for (key, value) in obj {
                inject_value(&mut scope, key, value);
            }
        }

        let result: rhai::Dynamic = engine
            .eval_with_scope(&mut scope, template)
            .map_err(|e| dwell_core::DwellError::Template(format!("Rhai error: {}", e)))?;

        Ok(result.to_string())
    }

    fn validate(&self, template: &str) -> std::result::Result<(), String> {
        Self::make_engine()
            .compile(template)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn file_extension(&self) -> &str {
        "rhai"
    }
}

fn inject_value(scope: &mut rhai::Scope, key: &str, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => { scope.push(key, ()); }
        serde_json::Value::Bool(b) => { scope.push(key, *b); }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                scope.push(key, i);
            } else if let Some(f) = n.as_f64() {
                scope.push(key, f);
            }
        }
        serde_json::Value::String(s) => { scope.push(key, s.clone()); }
        serde_json::Value::Array(_) => { scope.push(key, value.to_string()); }
        serde_json::Value::Object(_) => { scope.push(key, value.to_string()); }
    }
}
