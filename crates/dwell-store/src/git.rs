//! Git repository abstraction for the source store.

use std::path::{Path, PathBuf};

use dwell_core::{DwellError, Result};

fn git_err(e: git2::Error) -> DwellError {
    DwellError::Git(format!("{}", e))
}

/// Wrapper around a git repository for dotfile version control.
pub struct GitRepo {
    pub path: PathBuf,
}

impl GitRepo {
    pub fn init(path: &Path) -> Result<Self> {
        let repo = git2::Repository::init_bare(path).map_err(|e| {
            DwellError::Git(format!(
                "Failed to init bare repo at {}: {}",
                path.display(),
                e
            ))
        })?;
        {
            let mut config = repo.config().map_err(git_err)?;
            config
                .set_str("status.showUntrackedFiles", "no")
                .map_err(git_err)?;
        }
        Ok(GitRepo {
            path: path.to_path_buf(),
        })
    }

    pub fn clone(url: &str, path: &Path) -> Result<Self> {
        git2::Repository::clone(url, path)
            .map_err(|e| DwellError::Git(format!("Clone {}: {}", url, e)))?;
        Ok(GitRepo {
            path: path.to_path_buf(),
        })
    }

    pub fn open(path: &Path) -> Result<Self> {
        git2::Repository::open(path).map_err(|e| {
            DwellError::Git(format!("Not a git repository at {}: {}", path.display(), e))
        })?;
        Ok(GitRepo {
            path: path.to_path_buf(),
        })
    }

    pub fn add(&self, relative_path: &str) -> Result<()> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        let mut index = repo.index().map_err(git_err)?;
        index.add_path(Path::new(relative_path)).map_err(git_err)?;
        index.write().map_err(git_err)?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        let signature = repo.signature().map_err(git_err)?;
        let mut index = repo.index().map_err(git_err)?;
        let tree_oid = index.write_tree().map_err(git_err)?;
        let tree = repo.find_tree(tree_oid).map_err(git_err)?;

        let parents: Vec<git2::Commit<'_>> = match repo.head() {
            Ok(head) => vec![head.peel_to_commit().map_err(git_err)?],
            Err(_) => vec![],
        };
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();

        let commit_oid = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs[..],
            )
            .map_err(git_err)?;

        Ok(commit_oid.to_string())
    }

    pub fn head_commit(&self) -> Result<Option<String>> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        let head_result = repo.head();
        match head_result {
            Ok(head) => {
                let commit_id = head.peel_to_commit().map_err(git_err)?.id().to_string();
                Ok(Some(commit_id))
            }
            Err(ref e) if e.code() == git2::ErrorCode::UnbornBranch => Ok(None),
            Err(e) => Err(DwellError::Git(format!("HEAD error: {}", e))),
        }
    }

    pub fn has_changes(&self) -> Result<bool> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        let statuses = repo.statuses(None).map_err(git_err)?;
        Ok(!statuses.is_empty())
    }

    pub fn status_summary(&self) -> Result<String> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        let statuses = repo.statuses(None).map_err(git_err)?;
        let mut summary = String::new();
        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("<unknown>");
            summary.push_str(&format!(" {} {}\n", status_to_str(entry.status()), path));
        }
        Ok(summary.trim().to_string())
    }

    /// Stage all changes (git add -A).
    pub fn add_all(&self) -> Result<()> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        let mut index = repo.index().map_err(git_err)?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(git_err)?;
        index.write().map_err(git_err)?;
        Ok(())
    }

    /// Get the URL of the origin remote, if configured.
    pub fn remote_url(&self) -> Result<Option<String>> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        let result = match repo.find_remote("origin") {
            Ok(remote) => remote.url().map(|s| s.to_string()),
            Err(_) => None,
        };
        Ok(result)
    }

    /// Check if a remote (origin) is configured.
    pub fn has_remote(&self) -> Result<bool> {
        self.remote_url().map(|r| r.is_some())
    }

    /// Push the current branch to origin.
    pub fn push(&self) -> Result<()> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;

        // Determine current branch name
        let head = repo.head().map_err(git_err)?;
        let branch = head.shorthand().unwrap_or("main").to_string();

        let mut remote = repo
            .find_remote("origin")
            .map_err(|_| DwellError::Git("No remote 'origin' configured".into()))?;

        // Push the current branch
        let refspec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
        let mut push_opts = git2::PushOptions::new();
        remote
            .push(&[&refspec], Some(&mut push_opts))
            .map_err(git_err)?;

        Ok(())
    }

    /// Set the origin remote URL.
    pub fn set_remote(&self, url: &str) -> Result<()> {
        let repo = git2::Repository::open(&self.path).map_err(git_err)?;
        // Remove existing origin if present
        if repo.find_remote("origin").is_ok() {
            repo.remote_delete("origin").map_err(git_err)?;
        }
        repo.remote("origin", url).map_err(git_err)?;
        Ok(())
    }
}

fn status_to_str(status: git2::Status) -> &'static str {
    if status.contains(git2::Status::INDEX_NEW) {
        "A"
    } else if status.contains(git2::Status::INDEX_MODIFIED) {
        "M"
    } else if status.contains(git2::Status::INDEX_DELETED) {
        "D"
    } else if status.contains(git2::Status::WT_NEW) {
        "??"
    } else if status.contains(git2::Status::WT_MODIFIED) {
        " M"
    } else if status.contains(git2::Status::WT_DELETED) {
        " D"
    } else {
        " ?"
    }
}
