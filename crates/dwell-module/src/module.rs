pub struct ModuleRegistry;

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self
    }
}

impl ModuleRegistry {
    pub fn new() -> Self {
        ModuleRegistry
    }

    pub fn list(&self) -> Vec<&str> {
        vec![]
    }
}
