use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use dwell_core::{DwellError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub target_path: String,
    pub existed: bool,
    pub is_dir: bool,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationSnapshot {
    pub id: u64,
    pub created_at: u64,
    pub entries: Vec<SnapshotEntry>,
}

pub struct GenerationManager {
    root: PathBuf,
}

impl GenerationManager {
    pub fn new() -> Self {
        let root = dirs::cache_dir()
            .map(|d| d.join("dwell").join("generations"))
            .unwrap_or_else(|| PathBuf::from("/tmp/dwell-generations"));
        Self { root }
    }

    pub fn capture_snapshot(&self, id: u64, targets: &[PathBuf]) -> Result<GenerationSnapshot> {
        std::fs::create_dir_all(&self.root).map_err(DwellError::Io)?;
        let gen_dir = self.root.join(id.to_string());
        if gen_dir.exists() {
            std::fs::remove_dir_all(&gen_dir).map_err(DwellError::Io)?;
        }
        std::fs::create_dir_all(gen_dir.join("files")).map_err(DwellError::Io)?;

        let mut entries = Vec::with_capacity(targets.len());

        for (idx, target) in targets.iter().enumerate() {
            let existed = target.exists();
            let is_dir = existed && target.is_dir();

            let backup_path = if existed && target.is_file() {
                let rel = format!("files/{}.bak", idx);
                let dst = gen_dir.join(&rel);
                std::fs::copy(target, &dst).map_err(DwellError::Io)?;
                Some(rel)
            } else {
                None
            };

            entries.push(SnapshotEntry {
                target_path: target.to_string_lossy().to_string(),
                existed,
                is_dir,
                backup_path,
            });
        }

        let snapshot = GenerationSnapshot {
            id,
            created_at: now_secs(),
            entries,
        };
        self.save_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub fn list(&self) -> Result<Vec<GenerationSnapshot>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.root).map_err(DwellError::Io)? {
            let entry = entry.map_err(DwellError::Io)?;
            if !entry.file_type().map_err(DwellError::Io)?.is_dir() {
                continue;
            }
            let manifest = entry.path().join("snapshot.json");
            if manifest.exists() {
                if let Ok(s) = self.load_snapshot_from(&manifest) {
                    out.push(s);
                }
            }
        }
        out.sort_by_key(|s| s.id);
        Ok(out)
    }

    pub fn rollback(&self, id: u64) -> Result<usize> {
        let snapshot = self.load_snapshot(id)?;
        let mut restored = 0usize;

        for item in &snapshot.entries {
            let target = PathBuf::from(&item.target_path);
            if item.existed {
                if item.is_dir {
                    std::fs::create_dir_all(&target).map_err(DwellError::Io)?;
                    restored += 1;
                } else if let Some(rel) = &item.backup_path {
                    let backup = self.root.join(id.to_string()).join(rel);
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent).map_err(DwellError::Io)?;
                    }
                    std::fs::copy(backup, &target).map_err(DwellError::Io)?;
                    restored += 1;
                }
            } else if target.exists() {
                if target.is_dir() {
                    std::fs::remove_dir_all(&target).map_err(DwellError::Io)?;
                } else {
                    std::fs::remove_file(&target).map_err(DwellError::Io)?;
                }
                restored += 1;
            }
        }

        Ok(restored)
    }

    pub fn prune(&self, keep: usize) -> Result<usize> {
        let mut snaps = self.list()?;
        if snaps.len() <= keep {
            return Ok(0);
        }

        snaps.sort_by_key(|s| s.id);
        let remove_count = snaps.len().saturating_sub(keep);
        let mut removed = 0usize;

        for snap in snaps.into_iter().take(remove_count) {
            let dir = self.root.join(snap.id.to_string());
            if dir.exists() {
                std::fs::remove_dir_all(dir).map_err(DwellError::Io)?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    fn save_snapshot(&self, snapshot: &GenerationSnapshot) -> Result<()> {
        let dir = self.root.join(snapshot.id.to_string());
        std::fs::create_dir_all(&dir).map_err(DwellError::Io)?;
        let body = serde_json::to_string_pretty(snapshot).map_err(DwellError::Serialization)?;
        std::fs::write(dir.join("snapshot.json"), body).map_err(DwellError::Io)
    }

    fn load_snapshot(&self, id: u64) -> Result<GenerationSnapshot> {
        self.load_snapshot_from(&self.root.join(id.to_string()).join("snapshot.json"))
    }

    fn load_snapshot_from(&self, path: &Path) -> Result<GenerationSnapshot> {
        let body = std::fs::read_to_string(path).map_err(DwellError::Io)?;
        serde_json::from_str(&body).map_err(DwellError::Serialization)
    }
}

impl Default for GenerationManager {
    fn default() -> Self {
        Self::new()
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
