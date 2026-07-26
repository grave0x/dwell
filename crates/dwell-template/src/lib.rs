//! Pluggable template engine with Handlebars, Rhai, and Tera backends.

use dwell_core::traits::TemplateEngine;

mod handlebars_engine;
mod rhai_engine;
mod tera_engine;

pub use handlebars_engine::HandlebarsEngine;
pub use rhai_engine::RhaiEngine;
pub use tera_engine::TeraEngine;

/// Supported template dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateDialect {
    Handlebars,
    Rhai,
    GoTemplate,    // planned
    Tera,          // planned
}

impl TemplateDialect {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "hbs" | "handlebars" => Some(TemplateDialect::Handlebars),
            "rhai" => Some(TemplateDialect::Rhai),
            "gotmpl" | "tmpl" => Some(TemplateDialect::GoTemplate),
            "tera" => Some(TemplateDialect::Tera),
            _ => None,
        }
    }
}

/// Registry of available template engines.
pub struct TemplateRegistry {
    engines: Vec<Box<dyn TemplateEngine>>,
    default: usize,
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRegistry {
    pub fn new() -> Self {
        let engines: Vec<Box<dyn TemplateEngine>> = vec![
            Box::new(HandlebarsEngine::new()),
            Box::new(RhaiEngine::new()),
            Box::new(TeraEngine::new()),
        ];
        TemplateRegistry {
            default: 0, // Handlebars
            engines,
        }
    }

    /// Get an engine by name or extension.
    pub fn get(&self, name: &str) -> Option<&dyn TemplateEngine> {
        self.engines
            .iter()
            .find(|e| e.name() == name || e.file_extension() == name)
            .map(|e| e.as_ref())
    }

    /// Get the default engine.
    pub fn default_engine(&self) -> &dyn TemplateEngine {
        self.engines[self.default].as_ref()
    }

    /// Get the engine most appropriate for a source file.
    pub fn engine_for_file(&self, filename: &str) -> &dyn TemplateEngine {
        // Check explicit extension markers
        for engine in &self.engines {
            let ext = engine.file_extension();
            if filename.ends_with(&format!(".{}", ext)) {
                return engine.as_ref();
            }
            // Check for .hbs.tmpl, .rhai.tmpl, etc.
            if filename.contains(&format!(".{}.", ext)) {
                return engine.as_ref();
            }
        }
        // Default: Handlebars for .tmpl, no engine for non-template files
        if filename.ends_with(".tmpl") {
            self.default_engine()
        } else {
            self.engines[0].as_ref()
        }
    }

    /// Check if a file is a template (should be rendered).
    pub fn is_template(&self, filename: &str) -> bool {
        filename.ends_with(".tmpl")
            || filename.ends_with(".hbs")
            || filename.ends_with(".rhai")
            || filename.contains(".hbs.")
            || filename.contains(".rhai.")
    }
}
