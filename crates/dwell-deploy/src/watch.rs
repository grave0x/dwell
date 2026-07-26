//! File watcher for auto-redeploy on changes.

use std::path::PathBuf;

use dwell_core::{DwellError, Result};

fn watch_err(e: notify::Error) -> DwellError {
    DwellError::Other(format!("Watch error: {}", e))
}

/// Watches a source directory and invokes a callback on changes.
pub struct Watcher {
    source_dir: PathBuf,
}

impl Watcher {
    pub fn new(source_dir: PathBuf) -> Self {
        Watcher { source_dir }
    }

    /// Start watching — blocks until an error or channel close.
    pub fn watch<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(&str) + Send + 'static,
    {
        use notify::{Config, Event, EventKind, RecursiveMode, Watcher as _};

        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }).map_err(watch_err)?;

        watcher.configure(Config::default().with_poll_interval(std::time::Duration::from_secs(2)))
            .map_err(watch_err)?;
        watcher.watch(&self.source_dir, RecursiveMode::Recursive)
            .map_err(watch_err)?;

        tracing::info!("Watching {} for changes...", self.source_dir.display());

        for event in rx {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    for path in &event.paths {
                        if let Ok(rel) = path.strip_prefix(&self.source_dir) {
                            callback(&rel.to_string_lossy());
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
