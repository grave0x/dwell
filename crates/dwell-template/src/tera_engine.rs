use dwell_core::traits::TemplateEngine;
use dwell_core::Result;
use tera::{Context, Tera};

/// Tera template engine backend.
pub struct TeraEngine;

impl Default for TeraEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TeraEngine {
    pub fn new() -> Self {
        TeraEngine
    }
}

impl TemplateEngine for TeraEngine {
    fn name(&self) -> &str {
        "tera"
    }

    fn render(&self, template: &str, data: &serde_json::Value) -> Result<String> {
        let context = Context::from_serialize(data)
            .map_err(|e| dwell_core::DwellError::Template(format!(
                "Failed to create Tera context: {}", e
            )))?;

        Tera::one_off(template, &context, false)
            .map_err(|e| dwell_core::DwellError::Template(format!(
                "Tera render error: {}", e
            )))
    }

    fn validate(&self, template: &str) -> std::result::Result<(), String> {
        let context = Context::new();
        Tera::one_off(template, &context, false)
            .map(|_| ())
            .map_err(|e| format!("{}", e))
    }

    fn file_extension(&self) -> &str {
        "tera"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_variable_interpolation() {
        let engine = TeraEngine::new();
        let template = "Hello {{ name }}";
        let data = json!({"name": "World"});
        let result = engine.render(template, &data).unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_validation_rejects_invalid_syntax() {
        let engine = TeraEngine::new();
        // An unclosed `{% if %}` block is genuinely invalid Tera syntax
        let result = engine.validate("{% if x %}");
        assert!(result.is_err(), "Expected validation error for unclosed if block");
    }

    #[test]
    fn test_validation_accepts_valid_template() {
        let engine = TeraEngine::new();
        // Plain text with no variable references passes validation
        let result = engine.validate("Hello World");
        assert!(result.is_ok(), "Expected valid template to pass validation");
    }

    #[test]
    fn test_render_with_nested_data() {
        let engine = TeraEngine::new();
        let template = "{{ user.name }} is {{ user.age }} years old";
        let data = json!({"user": {"name": "Alice", "age": 30}});
        let result = engine.render(template, &data).unwrap();
        assert_eq!(result, "Alice is 30 years old");
    }

    #[test]
    fn test_render_with_list_iteration() {
        let engine = TeraEngine::new();
        let template = "{% for item in items %}{{ item }},{% endfor %}";
        let data = json!({"items": ["a", "b", "c"]});
        let result = engine.render(template, &data).unwrap();
        assert_eq!(result, "a,b,c,");
    }
}
