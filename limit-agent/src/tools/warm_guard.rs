//! Smart detection for TLDR warm freshness.
//!
//! Tracks project metadata (path, file count, timestamp) to decide
//! whether to skip warm on startup.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing;

const FRESHNESS_SECS: u64 = 300; // 5 minutes

/// Metadata about the last warm run
#[derive(Serialize, Deserialize)]
struct WarmMeta {
    project_path: String,
    file_count: usize,
    timestamp: u64,
}

/// Determines whether a TLDR warm is still fresh.
pub struct WarmGuard {
    meta_path: PathBuf,
}

impl WarmGuard {
    /// Create a WarmGuard for a given project's cache directory.
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            meta_path: cache_dir.join(".warm_meta"),
        }
    }

    /// Check if the warm is still fresh for the given project path.
    pub fn is_fresh(&self, project_path: &Path) -> bool {
        let meta = match self.load() {
            Some(m) => m,
            None => return false,
        };

        // Different project → not fresh
        if meta.project_path != project_path.display().to_string() {
            return false;
        }

        // Recent warm → trust it, skip walkdir
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if now.saturating_sub(meta.timestamp) < FRESHNESS_SECS {
            tracing::debug!("warm_guard: fresh ({}s ago)", now - meta.timestamp);
            return true;
        }

        // Old enough to verify file count
        let current_count = count_source_files(project_path);
        if current_count == meta.file_count {
            tracing::debug!(
                "warm_guard: fresh (file_count={}, {}s ago)",
                current_count,
                now - meta.timestamp
            );
            true
        } else {
            tracing::debug!(
                "warm_guard: stale (file_count was {}, now {})",
                meta.file_count,
                current_count
            );
            false
        }
    }

    /// Save metadata after a successful warm.
    pub fn save(&self, project_path: &Path) {
        let file_count = count_source_files(project_path);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let meta = WarmMeta {
            project_path: project_path.display().to_string(),
            file_count,
            timestamp,
        };

        if let Some(parent) = self.meta_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match serde_json::to_string(&meta) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.meta_path, &json) {
                    tracing::warn!("warm_guard: failed to save meta: {}", e);
                }
            }
            Err(e) => tracing::warn!("warm_guard: failed to serialize meta: {}", e),
        }
    }

    fn load(&self) -> Option<WarmMeta> {
        let content = std::fs::read_to_string(&self.meta_path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

/// Count source files in a project (recursive, same logic as ParseCoordinator).
fn count_source_files(project_path: &Path) -> usize {
    let extensions: &[&str] = &[
        "py", "ts", "tsx", "js", "jsx", "mjs", "go", "rs", "java", "c", "h", "cpp", "cc", "cxx",
        "hpp", "hxx", "rb", "php", "cs", "kt", "kts", "scala", "swift", "lua", "ex", "exs",
    ];

    let mut count = 0;
    for entry in walkdir::WalkDir::new(project_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let relative = path.strip_prefix(project_path).unwrap_or(path);
        if relative.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s.starts_with('.')
                || s == "node_modules"
                || s == "target"
                || s == "venv"
                || s == "__pycache__"
        }) {
            continue;
        }
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| extensions.contains(&ext))
        {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_fresh_returns_false_when_no_meta() {
        let dir = tempfile::tempdir().unwrap();
        let guard = WarmGuard::new(dir.path());
        assert!(!guard.is_fresh(dir.path()));
    }

    #[test]
    fn save_and_is_fresh_within_5_min() {
        let dir = tempfile::tempdir().unwrap();
        let guard = WarmGuard::new(dir.path());
        guard.save(dir.path());
        assert!(guard.is_fresh(dir.path()));
    }

    #[test]
    fn is_fresh_returns_false_for_different_project() {
        let dir = tempfile::tempdir().unwrap();
        let guard = WarmGuard::new(dir.path());
        guard.save(dir.path());

        let other = tempfile::tempdir().unwrap();
        assert!(!guard.is_fresh(other.path()));
    }

    #[test]
    fn is_fresh_detects_file_count_change() {
        let dir = tempfile::tempdir().unwrap();

        // Create source files
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(src.join("lib.rs"), "fn lib() {}").unwrap();

        let guard = WarmGuard::new(dir.path());
        guard.save(dir.path());

        // Simulate old timestamp (beyond freshness window) so file_count check runs
        let content = std::fs::read_to_string(&guard.meta_path).unwrap();
        let mut meta: WarmMeta = serde_json::from_str(&content).unwrap();
        meta.timestamp = 0; // force old timestamp
        let updated = serde_json::to_string(&meta).unwrap();
        std::fs::write(&guard.meta_path, updated).unwrap();

        assert!(guard.is_fresh(dir.path()));

        // Add a new file
        std::fs::write(src.join("new.rs"), "fn new() {}").unwrap();
        assert!(!guard.is_fresh(dir.path()));
    }
}
