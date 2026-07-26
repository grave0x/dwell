use dwell_core::traits::TemplateEngine;
use dwell_core::Result;
use handlebars::Handlebars;

/// Handlebars template engine backend.
pub struct HandlebarsEngine {
    registry: Handlebars<'static>,
}

impl Default for HandlebarsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlebarsEngine {
    pub fn new() -> Self {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        HandlebarsEngine { registry: hb }
    }

    /// Register a template string under a name for later rendering.
    pub fn register_template(
        &mut self,
        name: &str,
        template: &str,
    ) -> std::result::Result<(), String> {
        self.registry
            .register_template_string(name, template)
            .map_err(|e| format!("Template parse error: {}", e))
    }
}

impl TemplateEngine for HandlebarsEngine {
    fn name(&self) -> &str {
        "handlebars"
    }

    fn render(&self, template: &str, data: &serde_json::Value) -> Result<String> {
        // Register as a temporary named template and render
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        hb.register_template_string("__inline__", template)
            .map_err(|e| dwell_core::DwellError::Template(format!("Parse error: {}", e)))?;
        hb.render("__inline__", data)
            .map_err(|e| dwell_core::DwellError::Template(format!("Render error: {}", e)))
    }

    fn validate(&self, template: &str) -> std::result::Result<(), String> {
        let mut hb = Handlebars::new();
        hb.set_strict_mode(false);
        hb.register_template_string("__validate__", template)
            .map_err(|e| format!("{}", e))
    }

    fn file_extension(&self) -> &str {
        "hbs"
    }
}
