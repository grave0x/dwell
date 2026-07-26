use std::collections::HashMap;

use dwell_core::traits::{Module, ModuleOption};
use dwell_core::Result;

/// Registry of available modules — discovers, validates, and resolves modules.
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn Module>>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut reg = ModuleRegistry {
            modules: HashMap::new(),
        };

        // Register built-in modules
        reg.register(Box::new(super::builtins::GitModule::new()));

        reg
    }

    /// Register a module by its ID.
    pub fn register(&mut self, module: Box<dyn Module>) {
        self.modules.insert(module.id().to_string(), module);
    }

    /// Get a module by ID.
    pub fn get(&self, id: &str) -> Option<&dyn Module> {
        self.modules.get(id).map(|m| m.as_ref())
    }

    /// List all registered module IDs.
    pub fn list(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.modules.keys().map(|s| s.as_str()).collect();
        ids.sort();
        ids
    }

    /// Generate entries for a single module with user-provided values.
    pub fn generate(&self, id: &str, values: &serde_json::Value) -> Result<Vec<dwell_core::Entry>> {
        match self.modules.get(id) {
            Some(module) => module.generate(values),
            None => Err(dwell_core::DwellError::Module(format!(
                "Unknown module: {}. Available: {}", id, self.list().join(", ")
            ))),
        }
    }

    /// Generate entries for all modules listed in the config.
    /// Config format: { "programs.git": { "user.name": "...", ... }, ... }
    pub fn generate_all(&self, config: &serde_json::Value) -> Result<Vec<dwell_core::Entry>> {
        let mut all_entries = vec![];
        if let Some(obj) = config.as_object() {
            for (id, values) in obj {
                // Only process if we have a registered module for this ID
                if self.modules.contains_key(id) {
                    let entries = self.generate(id, values)?;
                    all_entries.extend(entries);
                }
                // Silently skip unknown module IDs (they might be parsed by config.rs separately)
            }
        }
        Ok(all_entries)
    }

    /// Get the options schema for a module.
    pub fn options(&self, id: &str) -> Option<Vec<ModuleOption>> {
        self.modules.get(id).map(|m| m.options())
    }

    /// Check if a module is registered.
    pub fn has(&self, id: &str) -> bool {
        self.modules.contains_key(id)
    }
}
