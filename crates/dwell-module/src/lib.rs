//! Declarative module system — TOML-defined program config generators.
//!
//! Modules generate dotfile entries from typed, validated user options,
//! inspired by home-manager's module system but using TOML + Rhai/Lua
//! instead of Nix. Full implementation in Phase 2.

pub mod module;
pub mod validation;

pub use module::ModuleRegistry;
pub use validation::Validator;
