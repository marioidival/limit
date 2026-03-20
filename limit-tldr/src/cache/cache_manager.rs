//! Cache management for TLDR
//!
//! New format (v3): single cache file per source file at `cache/<blake3_hash>.json`.
//! Hash in filename = no separate registry needed.
//! Version inside each file = per-file invalidation on version bump.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::FileAnalysis;

/// Cache version - bump when parser logic changes to invalidate stale cache
const CACHE_VERSION: &str = "v3";

/// Serialized cache entry
#[derive(Serialize, Deserialize)]
struct CacheEntry {
    version: String,
    hash: String,
    file: String,
    analysis: FileAnalysis,
}

/// Cache manager for storing analysis results
pub struct CacheManager {
    cache_dir: PathBuf,
}

impl CacheManager {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| Error::Cache(format!("Failed to create cache dir: {}", e)))?;

        Ok(Self { cache_dir })
    }

    /// Compute content hash for a file
    fn content_hash(file: &Path) -> Result<String> {
        let content =
            std::fs::read(file).map_err(|e| Error::Cache(format!("Failed to read file: {}", e)))?;
        Ok(blake3::hash(&content).to_hex().to_string())
    }

    /// Cache path derived from file's blake3 content hash
    fn cache_path(&self, file: &Path) -> Result<PathBuf> {
        let hash = Self::content_hash(file)?;
        Ok(self.cache_dir.join(format!("{}.json", hash)))
    }

    /// Check if a file's cache is fresh (hash matches)
    pub fn is_cached(&self, file: &Path) -> Result<bool> {
        let cache_path = self.cache_path(file)?;

        if !cache_path.exists() {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&cache_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache: {}", e)))?;

        let entry: CacheEntry = match serde_json::from_str(&content) {
            Ok(e) => e,
            Err(_) => return Ok(false),
        };

        // Version mismatch → stale
        if entry.version != CACHE_VERSION {
            return Ok(false);
        }

        // Content hash match → fresh
        let current_hash = Self::content_hash(file)?;
        Ok(entry.hash == current_hash)
    }

    /// Store file analysis in cache
    pub fn store_file(&mut self, file: &Path, analysis: &FileAnalysis) -> Result<()> {
        let hash = Self::content_hash(file)?;

        let entry = CacheEntry {
            version: CACHE_VERSION.to_string(),
            hash,
            file: file.display().to_string(),
            analysis: analysis.clone(),
        };

        let cache_path = self.cache_path(file)?;
        let json = serde_json::to_string_pretty(&entry)
            .map_err(|e| Error::Cache(format!("Failed to serialize: {}", e)))?;

        std::fs::write(&cache_path, json)
            .map_err(|e| Error::Cache(format!("Failed to write cache: {}", e)))?;

        Ok(())
    }

    /// Get cached file analysis
    pub fn get_file(&self, file: &Path) -> Result<Option<FileAnalysis>> {
        let cache_path = self.cache_path(file)?;

        if !cache_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&cache_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache: {}", e)))?;

        let entry: CacheEntry = match serde_json::from_str(&content) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };

        // Skip if version mismatch
        if entry.version != CACHE_VERSION {
            return Ok(None);
        }

        Ok(Some(entry.analysis))
    }

    /// Delete old-version cache files on startup
    pub fn cleanup(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                // Skip non-analysis cache files (e.g., semantic_index.json)
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == "semantic_index.json" || file_name.starts_with('.') {
                    continue;
                }
                // Check version inside
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(entry) = serde_json::from_str::<CacheEntry>(&content) {
                        if entry.version != CACHE_VERSION {
                            let _ = std::fs::remove_file(&path);
                        }
                        continue;
                    }
                }
                // Can't parse as CacheEntry — remove orphaned file
                let _ = std::fs::remove_file(&path);
            }
        }

        // Remove old cache structure (v2)
        let _ = std::fs::remove_file(self.cache_dir.join("file_hashes.json"));
        let _ = std::fs::remove_dir_all(self.cache_dir.join("parse_cache"));
        let _ = std::fs::remove_file(self.cache_dir.join("call_graph.json"));
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FunctionInfo, Language};
    use std::path::PathBuf;

    fn make_analysis(path: &str) -> FileAnalysis {
        FileAnalysis {
            file: PathBuf::from(path),
            functions: vec![FunctionInfo {
                name: "test_fn".to_string(),
                signature: "fn test_fn()".to_string(),
                params: Vec::new(),
                return_type: None,
                is_async: false,
                line: 1,
                end_line: 5,
                file: PathBuf::from(path),
                docstring: None,
                complexity: None,
            }],
            classes: Vec::new(),
            imports: Vec::new(),
            call_expressions: Vec::new(),
            language: Language::Rust,
        }
    }

    #[test]
    fn store_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = CacheManager::new(dir.path().to_path_buf()).unwrap();

        let source_file = dir.path().join("src/main.rs");
        std::fs::create_dir_all(source_file.parent().unwrap()).unwrap();
        std::fs::write(&source_file, "fn test_fn() {}").unwrap();

        let analysis = make_analysis(source_file.to_str().unwrap());
        cache.store_file(&source_file, &analysis).unwrap();

        assert!(cache.is_cached(&source_file).unwrap());

        let loaded = cache.get_file(&source_file).unwrap().unwrap();
        assert_eq!(loaded.functions.len(), 1);
        assert_eq!(loaded.functions[0].name, "test_fn");
    }

    #[test]
    fn cache_invalidation_on_content_change() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = CacheManager::new(dir.path().to_path_buf()).unwrap();

        let source_file = dir.path().join("test.rs");
        std::fs::write(&source_file, "fn v1() {}").unwrap();

        let analysis = make_analysis(source_file.to_str().unwrap());
        cache.store_file(&source_file, &analysis).unwrap();
        assert!(cache.is_cached(&source_file).unwrap());

        // Modify source
        std::fs::write(&source_file, "fn v2() {}").unwrap();
        assert!(!cache.is_cached(&source_file).unwrap());
    }

    #[test]
    fn cleanup_removes_old_format() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CacheManager::new(dir.path().to_path_buf()).unwrap();

        // Create old-format files
        std::fs::write(dir.path().join("file_hashes.json"), "{}").unwrap();
        std::fs::create_dir_all(dir.path().join("parse_cache")).unwrap();
        std::fs::write(dir.path().join("call_graph.json"), "{}").unwrap();

        cache.cleanup();

        assert!(!dir.path().join("file_hashes.json").exists());
        assert!(!dir.path().join("parse_cache").exists());
        assert!(!dir.path().join("call_graph.json").exists());
    }
}
