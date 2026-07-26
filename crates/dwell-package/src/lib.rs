//! Cross-platform package management — pacman, brew, nix, apt, cargo, etc.

mod backends;
pub mod manager;

pub use manager::PackageManagerRegistry;
