//! Cache management for TLDR
//!
//! Provides incremental updates via content hashing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::types::FileAnalysis;

/// Cache manager for storing analysis results
pub struct CacheManager {
    cache_dir: PathBuf,
    file_hashes: HashMap<PathBuf, String>,
    call_graph_cache: Option<CallGraphCache>,
}

#[derive(Serialize, Deserialize)]
struct CallGraphCache {
    forward: HashMap<String, Vec<String>>,
    backward: HashMap<String, Vec<crate::types::CallerInfo>>,
}

impl CacheManager {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| Error::Cache(format!("Failed to create cache dir: {}", e)))?;
        
        let hashes_path = cache_dir.join("file_hashes.json");
        let file_hashes = if hashes_path.exists() {
            let content = std::fs::read_to_string(&hashes_path)
                .map_err(|e| Error::Cache(format!("Failed to read hashes: {}", e)))?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        Ok(Self {
            cache_dir,
            file_hashes,
            call_graph_cache: None,
        })
    }
    
    /// Check if a file is cached and unchanged
    pub fn is_cached(&self, file: &Path) -> Result<bool> {
        let content = std::fs::read(file)
            .map_err(|e| Error::Cache(format!("Failed to read file: {}", e)))?;
        
        let hash = blake3::hash(&content).to_string();
        
        Ok(self.file_hashes.get(file).map(|h| h == &hash).unwrap_or(false))
    }
    
    /// Store file analysis in cache
    pub fn store_file(&mut self, file: &Path, analysis: &FileAnalysis) -> Result<()> {
        let content = std::fs::read(file)
            .map_err(|e| Error::Cache(format!("Failed to read file: {}", e)))?;
        
        let hash = blake3::hash(&content).to_string();
        self.file_hashes.insert(file.to_path_buf(), hash);
        
        let cache_path = self.cache_dir.join(format!(
            "parse_cache/{}.json",
            file.display().to_string().replace(['/', '\\'], "_")
        ));
        
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Cache(format!("Failed to create cache dir: {}", e)))?;
        }
        
        let json = serde_json::to_string_pretty(analysis)
            .map_err(|e| Error::Cache(format!("Failed to serialize: {}", e)))?;
        
        std::fs::write(&cache_path, json)
            .map_err(|e| Error::Cache(format!("Failed to write cache: {}", e)))?;
        
        // Persist hashes
        let hashes_path = self.cache_dir.join("file_hashes.json");
        let json = serde_json::to_string(&self.file_hashes)
            .map_err(|e| Error::Cache(format!("Failed to serialize hashes: {}", e)))?;
        
        std::fs::write(&hashes_path, json)
            .map_err(|e| Error::Cache(format!("Failed to write hashes: {}", e)))?;
        
        Ok(())
    }
    
    /// Get cached file analysis
    pub fn get_file(&self, file: &Path) -> Result<Option<FileAnalysis>> {
        let cache_path = self.cache_path_for_file(file);
        
        if !cache_path.exists() {
            return Ok(None);
        }
        
        let content = std::fs::read_to_string(&cache_path)
            .map_err(|e| Error::Cache(format!("Failed to read cache: {}", e)))?;
        
        let analysis: FileAnalysis = serde_json::from_str(&content)
            .map_err(|e| Error::Cache(format!("Failed to deserialize: {}", e)))?;
        
        Ok(Some(analysis))
    }
    
    fn cache_path_for_file(&self, file: &Path) -> PathBuf {
        self.cache_dir.join(format!(
            "parse_cache/{}.json",
            file.display().to_string().replace(['/', '\\'], "_")
        ))
    }
    
    /// Store call graph
    pub fn store_call_graph(
        &mut self,
        forward: &HashMap<String, Vec<String>>,
        backward: &HashMap<String, Vec<crate::types::CallerInfo>>,
    ) -> Result<()> {
        self.call_graph_cache = Some(CallGraphCache {
            forward: forward.clone(),
            backward: backward.clone(),
        });
        
        let path = self.cache_dir.join("call_graph.json");
        let json = serde_json::to_string_pretty(&self.call_graph_cache)
            .map_err(|e| Error::Cache(format!("Failed to serialize: {}", e)))?;
        
        std::fs::write(&path, json)
            .map_err(|e| Error::Cache(format!("Failed to write call graph: {}", e)))?;
        
        Ok(())
    }
    
    /// Get call graph
    pub fn get_call_graph(&self) -> Result<Option<CallGraphRef<'_>>> {
        if let Some(ref cache) = self.call_graph_cache {
            Ok(Some(CallGraphRef {
                forward: &cache.forward,
                backward: &cache.backward,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Reference to cached call graph
pub struct CallGraphRef<'a> {
    pub forward: &'a HashMap<String, Vec<String>>,
    pub backward: &'a HashMap<String, Vec<crate::types::CallerInfo>>,
}
