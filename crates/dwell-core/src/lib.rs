//! Shared types, errors, and traits for the dwell dotfile manager.

pub mod config;
pub mod deps;
pub mod error;
pub mod scanner;
pub mod traits;
pub mod types;

pub use config::*;
pub use error::{DwellError, Result};
pub use types::*;
