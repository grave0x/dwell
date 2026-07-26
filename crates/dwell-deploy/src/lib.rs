//! Deployment engine — apply, diff, symlink, and atomic file operations.

pub mod apply;
pub mod diff;
pub mod link;
pub mod watch;

pub use apply::Deployer;
pub use diff::Differ;
pub use link::{link_file, unlink_target, verify_symlink};
