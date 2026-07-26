//! Deployment state tracking — what has been deployed and where.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Tracks which files have been deployed and their hashes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployState {
    /// Map from target path → deployed content hash.
    pub entries: HashMap<PathBuf, DeployEntry>,
    /// Generation counter (incremented on each apply).
    pub generation: u64,
    /// Timestamp of last deployment (Unix epoch seconds).
    pub last_deployed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployEntry {
    /// SHA-256 of content when deployed.
    pub hash: String,
    /// Source path that generated this target.
    pub source: String,
    /// When this was deployed (Unix epoch seconds).
    pub deployed_at: u64,
    /// The applied action.
    pub action: String,
}

impl DeployState {
    /// Load deployment state from disk.
    pub fn load(path: &PathBuf) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let state: DeployState = serde_json::from_str(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DeployState::new()),
            Err(e) => Err(e),
        }
    }

    /// Create a fresh, empty deployment state.
    pub fn new() -> Self {
        DeployState {
            entries: HashMap::new(),
            generation: 0,
            last_deployed: 0,
        }
    }

    /// Save deployment state to disk.
    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }

    /// Record a successful deployment.
    pub fn record(&mut self, target: &PathBuf, source: &str, hash: &str, action: &str) {
        self.entries.insert(
            target.clone(),
            DeployEntry {
                hash: hash.to_string(),
                source: source.to_string(),
                deployed_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                action: action.to_string(),
            },
        );
        self.last_deployed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get the hash of a previously deployed target.
    pub fn get_hash(&self, target: &PathBuf) -> Option<&str> {
        self.entries.get(target).map(|e| e.hash.as_str())
    }

    /// Check if a target is currently deployed.
    pub fn is_deployed(&self, target: &PathBuf) -> bool {
        self.entries.contains_key(target)
    }

    /// Remove a target from tracking (after undeploy).
    pub fn forget(&mut self, target: &PathBuf) {
        self.entries.remove(target);
    }

    /// Increment the generation counter.
    pub fn next_generation(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }
}
