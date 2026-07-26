//! Package manager registry — discovers available backends.

use dwell_core::traits::PackageManager;
use std::collections::HashMap;

pub struct PackageManagerRegistry {
    managers: HashMap<String, Box<dyn PackageManager>>,
}

impl Default for PackageManagerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageManagerRegistry {
    pub fn new() -> Self {
        let mut registry = PackageManagerRegistry {
            managers: HashMap::new(),
        };

        registry.register(Box::new(crate::backends::apt::AptBackend::new()));
        registry.register(Box::new(crate::backends::pacman::PacmanBackend::new()));
        registry.register(Box::new(crate::backends::brew::BrewBackend::new()));
        registry.register(Box::new(crate::backends::nix::NixBackend::new()));
        registry.register(Box::new(crate::backends::cargo::CargoBackend::new()));
        registry.register(Box::new(crate::backends::apk::ApkBackend::new()));

        registry
    }

    pub fn register(&mut self, manager: Box<dyn PackageManager>) {
        self.managers.insert(manager.id().to_string(), manager);
    }

    /// Get a specific manager by ID.
    pub fn get(&self, id: &str) -> Option<&dyn PackageManager> {
        self.managers.get(id).map(|m| m.as_ref())
    }

    /// Auto-detect the first available package manager.
    pub fn detect(&self) -> Option<&dyn PackageManager> {
        self.managers
            .values()
            .find(|m| m.is_available())
            .map(|m| m.as_ref())
    }

    /// Get all registered managers.
    pub fn all(&self) -> Vec<&dyn PackageManager> {
        self.managers.values().map(|m| m.as_ref()).collect()
    }

    pub fn available(&self) -> Vec<&str> {
        self.managers
            .iter()
            .filter(|(_, m)| m.is_available())
            .map(|(id, _)| id.as_str())
            .collect()
    }
}
