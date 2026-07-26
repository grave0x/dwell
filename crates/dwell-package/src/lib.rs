//! Cross-platform package management — pacman, brew, nix, apt, cargo, etc.

pub mod manager;
mod backends;

pub use manager::PackageManagerRegistry;
