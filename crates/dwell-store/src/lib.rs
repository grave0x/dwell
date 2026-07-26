//! Source store — git-backed dotfile repository management.

pub mod git;
pub mod ignore;
pub mod source;
pub mod state;

pub use git::GitRepo;
pub use ignore::IgnorePatterns;
pub use source::SourceDir;
pub use state::DeployState;
