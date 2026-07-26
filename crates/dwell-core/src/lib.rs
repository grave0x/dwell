//! Shared types, errors, and traits for the dwell dotfile manager.

pub mod config;
pub mod error;
pub mod traits;
pub mod types;

pub use config::*;
pub use error::{DwellError, Result};
pub use types::*;
