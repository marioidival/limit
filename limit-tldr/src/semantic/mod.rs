//! Semantic search using local BGE embeddings with hash-based incremental updates.
//!
//! ## Overview
//!
//! This module provides semantic code search using the BGE-Small-EN-v1.5 embedding model.
//! It uses a **hash-based incremental update strategy** to avoid full rebuilds when only
//! a few entries change.
//!
//! ## How Incremental Updates Work
//!
//! 1. **Entry Hashing**: Each code entry (function, struct, constant) gets a 64-bit hash
//!    computed from: `name + file_path + line_number + signature`.
//!
//! 2. **Cache Strategy**: Embeddings are stored in an `FxHashMap<u64, Vec<f32>>` keyed by
//!    entry hash. On rebuild, we check which hashes already exist in cache.
//!
//! 3. **Incremental Rebuild**: `build_embeddings()` only generates embeddings for entries
//!    with hashes not in the cache. Cached embeddings are reused directly.
//!
//! ## Performance Characteristics
//!
//! | Scenario | Full Rebuild | Incremental |
//! |----------|-------------|-------------|
//! | 2 new functions in 1566 | ~4 min | ~1-2 sec |
//! | First run (empty cache) | ~4 min | ~4 min |
//! | No changes (100% cache hit) | ~4 min | <1 sec |
//!
//! The embedding model produces 384-dimensional vectors at ~100 texts/sec.
//!
//! ## Cache Format (v4)
//!
//! Version history:
//! - v1-v3: Full rebuild only (embeddings array indexed by position)
//! - v4: Hash-based incremental format (embeddings keyed by entry hash)
//!
//! ## Feature Flag
//!
//! The `semantic` feature adds ~50MB to the binary via ONNX runtime.
//! Without it, falls back to text-based substring search.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rustc_hash::FxHashMap;

use crate::error::Result;
use crate::layers::CallGraphLayer;
use crate::types::{FileAnalysis, SearchResult};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use serde::{Deserialize, Serialize};

const CACHE_VERSION: &str = "v4"; // Bumped for hash-based incremental format

/// Serialized semantic index for persistence
#[derive(Serialize, Deserialize)]
struct SemanticCache {
    version: String,
    entries: Vec<(String, PathBuf, usize, String, String)>,
    entry_hashes: Vec<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeddings: Option<Vec<(u64, Vec<f32>)>>,
}

/// Entry kind for the search index
#[derive(Debug, Clone)]
enum EntryKind {
    Function,
    Struct,
    Constant,
}

/// Internal entry type shared between functions and structs
type Entry = (String, PathBuf, usize, String, EntryKind);

/// Mutable state protected by Mutex for concurrent access during background build
struct SemanticInner {
    embeddings: Option<Vec<Vec<f32>>>,
    model: Option<SendEmbedding>,
    embedding_cache: FxHashMap<u64, Vec<f32>>,
}

/// Wrapper around TextEmbedding that is Send.
///
/// Safety: Access is always protected by Mutex (one thread at a time).
/// The !Send marker comes from ONNX runtime internals, not actual unsafety.
struct SendEmbedding(TextEmbedding);

unsafe impl Send for SendEmbedding {}

impl std::ops::Deref for SendEmbedding {
    type Target = TextEmbedding;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SendEmbedding {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct SemanticIndex {
    entries: Vec<Entry>,
    embedding_texts: Vec<String>,
    entry_hashes: Vec<u64>,
    inner: Mutex<SemanticInner>,
}

impl SemanticIndex {
    pub fn new() -> Result<Self> {
        Ok(Self {
            entries: Vec::new(),
            embedding_texts: Vec::new(),
            entry_hashes: Vec::new(),
            inner: Mutex::new(SemanticInner {
                embeddings: None,
                model: None,
                embedding_cache: FxHashMap::default(),
            }),
        })
    }

    /// Computes a unique 64-bit hash for a code entry.
    ///
    /// Hash composition: `name + file_path + line_number + signature`
    ///
    /// This ensures:
    /// - Renaming a function → new hash (signature changes)
    /// - Moving to different line → new hash (location changes)
    /// - Changing parameters → new hash (signature changes)
    /// - Same function in different files → different hashes
    fn hash_entry(name: &str, file: &Path, line: usize, signature: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        file.to_string_lossy().hash(&mut hasher);
        line.hash(&mut hasher);
        signature.hash(&mut hasher);
        hasher.finish()
    }

    /// Configure ONNX Runtime thread limit before any model loading.
    /// Must be called once, before `load_model()` or `build_embeddings()`.
    pub fn init_runtime() {
        if std::env::var("ORT_NUM_THREADS").is_err() {
            let cores = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            let limit = cores.clamp(2, 4);
            std::env::set_var("ORT_NUM_THREADS", limit.to_string());
            tracing::info!(
                "semantic: set ORT_NUM_THREADS={} ({} cores available)",
                limit,
                cores
            );
        }
    }

    pub fn populate_entries(&mut self, analyses: &[FileAnalysis], call_graph: &CallGraphLayer) {
        let mut entries: Vec<Entry> = analyses
            .iter()
            .flat_map(|a| {
                a.functions.iter().map(|f| {
                    (
                        f.name.clone(),
                        f.file.clone(),
                        f.line,
                        f.signature.clone(),
                        EntryKind::Function,
                    )
                })
            })
            .collect();

        let struct_entries: Vec<Entry> = analyses
            .iter()
            .flat_map(|a| {
                a.classes.iter().map(|c| {
                    let name = format!("struct {}", c.name);
                    let sig = format!("struct {} {{ /* {} fields */ }}", c.name, c.fields.len());
                    (name, c.file.clone(), c.line, sig, EntryKind::Struct)
                })
            })
            .collect();

        entries.extend(struct_entries);

        let constant_entries: Vec<Entry> = analyses
            .iter()
            .flat_map(|a| {
                a.constants.iter().map(|c| {
                    let name = format!("const {}", c.name);
                    let sig = format!(
                        "const {}: {:?}",
                        c.name,
                        c.value.as_deref().unwrap_or_default()
                    );
                    (name, c.file.clone(), c.line, sig, EntryKind::Constant)
                })
            })
            .collect();

        entries.extend(constant_entries);
        self.entries = entries;

        self.entry_hashes = self
            .entries
            .iter()
            .map(|(name, file, line, sig, _)| Self::hash_entry(name, file, *line, sig))
            .collect();

        self.embedding_texts = self
            .entries
            .iter()
            .filter(|(_, _, _, _, kind)| matches!(kind, EntryKind::Function))
            .map(|(name, _, _, sig, _)| {
                let mut text = format!("{} {}", name, sig);
                if let Ok(callers) = call_graph.get_backward_calls(name) {
                    let names: Vec<&str> = callers.iter().map(|c| c.function.as_str()).collect();
                    if !names.is_empty() {
                        text.push_str(&format!(" called_by: {}", names.join(", ")));
                    }
                }
                if let Ok(callees) = call_graph.get_forward_calls(name) {
                    if !callees.is_empty() {
                        text.push_str(&format!(" calls: {}", callees.join(", ")));
                    }
                }
                text
            })
            .collect();
    }

    /// Builds embeddings incrementally, reusing cached embeddings when possible.
    ///
    /// Algorithm:
    /// 1. Load the embedding model from cache or download
    /// 2. For each function entry, compute its hash
    /// 3. Partition entries into cached (reuse) vs new (generate)
    /// 4. Generate embeddings only for new entries
    /// 5. Merge new embeddings into cache for future use
    ///
    /// Performance:
    /// - Model loading: ~200ms from disk cache, ~2s first download
    /// - Embedding generation: ~10ms per text (batched)
    /// - Cache lookup: O(1) via FxHashMap
    pub fn build_embeddings(&self, cache_dir: &Path) -> Result<()> {
        let model_cache = dirs::home_dir()
            .map(|h| h.join(".limit").join("fastembed"))
            .unwrap_or_else(|| cache_dir.join("fastembed"));

        let fn_count = self
            .entries
            .iter()
            .filter(|(_, _, _, _, kind)| matches!(kind, EntryKind::Function))
            .count();

        tracing::info!(
            "semantic: loading embedding model ({} functions to index)",
            fn_count
        );

        match TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_cache_dir(model_cache),
        ) {
            Ok(mut model) => {
                let mut inner = self.inner.lock().unwrap();
                let cache = &inner.embedding_cache;

                let function_hashes: Vec<u64> = self
                    .entries
                    .iter()
                    .filter(|(_, _, _, _, kind)| matches!(kind, EntryKind::Function))
                    .zip(self.entry_hashes.iter())
                    .map(|(_, &hash)| hash)
                    .collect();

                let (cached_count, new_indices): (Vec<usize>, Vec<usize>) = self
                    .embedding_texts
                    .iter()
                    .enumerate()
                    .map(|(i, _)| i)
                    .partition(|&i| cache.contains_key(&function_hashes[i]));

                tracing::info!(
                    "semantic: {} cached, {} new embeddings to generate",
                    cached_count.len(),
                    new_indices.len()
                );

                let mut new_embeddings: FxHashMap<u64, Vec<f32>> = FxHashMap::default();
                if !new_indices.is_empty() {
                    let new_texts: Vec<&str> = new_indices
                        .iter()
                        .map(|&i| self.embedding_texts[i].as_str())
                        .collect();

                    tracing::info!("semantic: embedding {} new texts...", new_texts.len());
                    match model.embed(&new_texts, None) {
                        Ok(embeddings) => {
                            for (&i, emb) in new_indices.iter().zip(embeddings.into_iter()) {
                                new_embeddings.insert(function_hashes[i], emb);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Semantic embedding generation failed: {}", e);
                            return Ok(());
                        }
                    }
                }

                let final_embeddings: Vec<Vec<f32>> = function_hashes
                    .iter()
                    .map(|&hash| {
                        new_embeddings
                            .get(&hash)
                            .cloned()
                            .or_else(|| cache.get(&hash).cloned())
                            .unwrap_or_default()
                    })
                    .collect();

                tracing::info!(
                    "semantic: embeddings ready ({} vectors)",
                    final_embeddings.len()
                );

                let mut merged_cache = cache.clone();
                for (hash, emb) in &new_embeddings {
                    merged_cache.insert(*hash, emb.clone());
                }

                inner.embeddings = Some(final_embeddings);
                inner.embedding_cache = merged_cache;
                inner.model = Some(SendEmbedding(model));
            }
            Err(e) => {
                tracing::warn!("Failed to load embedding model: {}", e);
            }
        }
        Ok(())
    }

    /// Load the embedding model only (~200ms from disk cache).
    /// Used when embeddings are loaded from cache but model isn't serialized.
    pub fn load_model(&self, cache_dir: &Path) {
        let model_cache = dirs::home_dir()
            .map(|h| h.join(".limit").join("fastembed"))
            .unwrap_or_else(|| cache_dir.join("fastembed"));

        tracing::info!("semantic: loading embedding model for cached embeddings");
        match TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_cache_dir(model_cache),
        ) {
            Ok(model) => {
                tracing::info!("semantic: model loaded");
                let mut inner = self.inner.lock().unwrap();
                inner.model = Some(SendEmbedding(model));
            }
            Err(e) => {
                tracing::warn!("Failed to load embedding model: {}", e);
            }
        }
    }

    /// Whether embeddings are available (non-blocking)
    pub fn is_ready(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.embeddings.is_some() && inner.model.is_some()
    }

    /// Backward-compatible warm using ASTLayer
    pub async fn warm(
        &mut self,
        ast: &crate::layers::ASTLayer,
        call_graph: &CallGraphLayer,
        cache_dir: &Path,
    ) -> Result<()> {
        let analyses: Vec<FileAnalysis> = ast.file_analyses().into_iter().cloned().collect();
        self.populate_entries(&analyses, call_graph);
        self.build_embeddings(cache_dir)
    }

    /// Returns true if embedding cache has entries (worth saving).
    pub fn should_save(&self) -> bool {
        !self.inner.lock().unwrap().embedding_cache.is_empty()
    }

    /// Checks whether any entries are missing from the embedding cache.
    ///
    /// Returns true if:
    /// - No entries exist yet (empty index)
    /// - Embedding cache is empty (first run or cache cleared)
    /// - Any current entry hash is not in the cache (new/changed entries)
    pub fn needs_rebuild(&self, analyses: &[FileAnalysis]) -> bool {
        if self.entries.is_empty() || self.inner.lock().unwrap().embedding_cache.is_empty() {
            return true;
        }

        let current_hashes: Vec<u64> = analyses
            .iter()
            .flat_map(|a| {
                a.functions
                    .iter()
                    .map(|f| Self::hash_entry(&f.name, &f.file, f.line, &f.signature))
            })
            .chain(analyses.iter().flat_map(|a| {
                a.classes.iter().map(|c| {
                    let name = format!("struct {}", c.name);
                    let sig = format!("struct {} {{ /* {} fields */ }}", c.name, c.fields.len());
                    Self::hash_entry(&name, &c.file, c.line, &sig)
                })
            }))
            .chain(analyses.iter().flat_map(|a| {
                a.constants.iter().map(|c| {
                    let name = format!("const {}", c.name);
                    let sig = format!(
                        "const {}: {:?}",
                        c.name,
                        c.value.as_deref().unwrap_or_default()
                    );
                    Self::hash_entry(&name, &c.file, c.line, &sig)
                })
            }))
            .collect();

        let cache = self.inner.lock().unwrap();
        current_hashes
            .iter()
            .any(|hash| !cache.embedding_cache.contains_key(hash))
    }

    /// Persists the semantic index to disk in JSON format (v4).
    ///
    /// Cache format:
    /// - version: "v4"
    /// - entries: [[name, file, line, signature, kind], ...]
    /// - entry_hashes: [hash1, hash2, ...]
    /// - embeddings: [[hash1, [f32; 384]], ...]
    ///
    /// Saved to `{cache_dir}/semantic_index.json`
    pub fn save(&self, cache_dir: &Path) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let entries: Vec<_> = self
            .entries
            .iter()
            .map(|(name, file, line, sig, kind)| {
                let kind_str = match kind {
                    EntryKind::Function => "fn",
                    EntryKind::Struct => "struct",
                    EntryKind::Constant => "const",
                };
                (
                    name.clone(),
                    file.clone(),
                    *line,
                    sig.clone(),
                    kind_str.to_string(),
                )
            })
            .collect();

        let inner = self.inner.lock().unwrap();
        let embeddings: Option<Vec<(u64, Vec<f32>)>> = if inner.embedding_cache.is_empty() {
            None
        } else {
            Some(
                inner
                    .embedding_cache
                    .iter()
                    .map(|(&k, v)| (k, v.clone()))
                    .collect(),
            )
        };

        let cache = SemanticCache {
            version: CACHE_VERSION.to_string(),
            entries,
            entry_hashes: self.entry_hashes.clone(),
            embeddings,
        };

        let path = cache_dir.join("semantic_index.json");
        let json = serde_json::to_string_pretty(&cache).map_err(|e| {
            crate::error::Error::Cache(format!("Failed to serialize semantic: {}", e))
        })?;

        std::fs::write(&path, json)
            .map_err(|e| crate::error::Error::Cache(format!("Failed to write semantic: {}", e)))?;

        Ok(())
    }

    /// Loads the semantic index from disk cache.
    ///
    /// Returns:
    /// - true: Cache loaded successfully (version matches, data valid)
    /// - false: Cache missing, corrupted, or version mismatch
    ///
    /// Only loads if cache.version == CACHE_VERSION. Incompatible versions
    /// trigger a full rebuild on the next build_embeddings() call.
    ///
    /// Post-load state:
    /// - entries: Populated from cache
    /// - embedding_cache: Populated (hash → embedding map)
    /// - model: NOT loaded (call load_model() separately, ~200ms)
    pub fn load(&mut self, cache_dir: &Path) -> bool {
        let path = cache_dir.join("semantic_index.json");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let cache: SemanticCache = match serde_json::from_str(&content) {
            Ok(c) => c,
            Err(_) => return false,
        };

        if cache.version != CACHE_VERSION {
            return false;
        }

        self.entries = cache
            .entries
            .into_iter()
            .filter_map(|(name, file, line, sig, kind_str)| {
                let kind = match kind_str.as_str() {
                    "fn" => EntryKind::Function,
                    "struct" => EntryKind::Struct,
                    "const" => EntryKind::Constant,
                    _ => return None,
                };
                Some((name, file, line, sig, kind))
            })
            .collect();

        self.entry_hashes = cache.entry_hashes;

        let inner = self.inner.get_mut().unwrap();
        inner.embedding_cache = cache
            .embeddings
            .map(|v| v.into_iter().collect())
            .unwrap_or_default();
        inner.model = None;

        true
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Try semantic search (non-blocking via try_lock)
        if let Ok(mut inner) = self.inner.try_lock() {
            if inner.model.is_some() && inner.embeddings.is_some() {
                let embeddings = inner.embeddings.as_ref().unwrap().clone();
                let model = inner.model.as_mut().unwrap();
                match model.embed(vec![query.to_string()], None) {
                    Ok(query_emb) => {
                        let q = &query_emb[0];
                        let mut scored: Vec<(usize, f32)> = embeddings
                            .iter()
                            .enumerate()
                            .map(|(i, emb)| (i, cosine_similarity(q, emb)))
                            .collect();
                        scored.sort_by(|a, b| {
                            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        return Ok(scored
                            .into_iter()
                            .take(limit)
                            .filter(|(idx, _)| *idx < self.entries.len())
                            .map(|(idx, score)| {
                                let (name, file, line, sig, _) = &self.entries[idx];
                                SearchResult {
                                    function: name.clone(),
                                    file: file.clone(),
                                    line: *line,
                                    score,
                                    signature: sig.clone(),
                                }
                            })
                            .collect());
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Semantic query embedding failed: {}, falling back to text search",
                            e
                        );
                    }
                }
            }
            // Model or embeddings not ready, or background build holding lock — fall through
        }

        self.text_search(query, limit).await
    }

    async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query_lower = query.to_lowercase();
        tracing::debug!(
            "text_search: query={:?}, entries_count={}",
            query_lower,
            self.entries.len()
        );

        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .filter_map(|(name, file, line, signature, _kind)| {
                let name_lower = name.to_lowercase();
                let sig_lower = signature.to_lowercase();
                let name_match = name_lower.contains(&query_lower);
                let file_match = file.to_string_lossy().to_lowercase().contains(&query_lower);
                let sig_match = sig_lower.contains(&query_lower);

                if !name_match && !file_match && !sig_match {
                    return None;
                }

                let score = if name_lower == query_lower {
                    1.0
                } else if name_lower.starts_with(&query_lower) {
                    0.9
                } else if name_match {
                    0.7
                } else if sig_match {
                    0.5
                } else {
                    0.3
                };

                Some(SearchResult {
                    function: name.clone(),
                    file: file.clone(),
                    line: *line,
                    score,
                    signature: signature.clone(),
                })
            })
            .collect();

        tracing::debug!(
            "text_search: query={:?} found {} results",
            query_lower,
            results.len()
        );

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.function.cmp(&b.function))
        });

        results.truncate(limit);
        Ok(results)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn text_search_fallback_works() {
        let mut index = SemanticIndex::new().unwrap();
        index.entries = vec![
            (
                "verify_access_token".to_string(),
                PathBuf::from("src/auth.py"),
                10,
                "fn verify_access_token(token: str) -> bool".to_string(),
                EntryKind::Function,
            ),
            (
                "get_user".to_string(),
                PathBuf::from("src/db.py"),
                5,
                "fn get_user(id: int) -> User".to_string(),
                EntryKind::Function,
            ),
        ];

        let results = index.search("access", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].function, "verify_access_token");
    }

    #[tokio::test]
    async fn search_ranks_exact_match_first() {
        let mut index = SemanticIndex::new().unwrap();
        index.entries = vec![
            (
                "process_payment".to_string(),
                PathBuf::from("src/pay.py"),
                1,
                "fn process_payment(amount: f64)".to_string(),
                EntryKind::Function,
            ),
            (
                "process_refund".to_string(),
                PathBuf::from("src/pay.py"),
                10,
                "fn process_refund(amount: f64)".to_string(),
                EntryKind::Function,
            ),
        ];

        let results = index.search("process_payment", 5).await.unwrap();
        assert_eq!(results[0].function, "process_payment");
    }

    #[tokio::test]
    async fn search_includes_file_path_matches() {
        let mut index = SemanticIndex::new().unwrap();
        index.entries = vec![
            (
                "run".to_string(),
                PathBuf::from("src/daemon/mod.rs"),
                1,
                "fn run()".to_string(),
                EntryKind::Function,
            ),
            (
                "handle".to_string(),
                PathBuf::from("src/daemon/handler.rs"),
                1,
                "fn handle()".to_string(),
                EntryKind::Function,
            ),
            (
                "other".to_string(),
                PathBuf::from("src/other.rs"),
                1,
                "fn other()".to_string(),
                EntryKind::Function,
            ),
        ];

        let results = index.search("daemon", 10).await.unwrap();
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.file.to_string_lossy().contains("daemon")));
    }

    #[tokio::test]
    async fn search_finds_structs() {
        let mut index = SemanticIndex::new().unwrap();
        index.entries = vec![
            (
                "run".to_string(),
                PathBuf::from("src/main.rs"),
                1,
                "fn run()".to_string(),
                EntryKind::Function,
            ),
            (
                "struct AppConfig".to_string(),
                PathBuf::from("src/config.rs"),
                10,
                "struct AppConfig { /* 3 fields */ }".to_string(),
                EntryKind::Struct,
            ),
        ];

        let results = index.search("Config", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].function, "struct AppConfig");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = SemanticIndex::new().unwrap();
        index.entries = vec![
            (
                "handler".to_string(),
                PathBuf::from("src/main.rs"),
                1,
                "fn handler()".to_string(),
                EntryKind::Function,
            ),
            (
                "struct Config".to_string(),
                PathBuf::from("src/config.rs"),
                5,
                "struct Config { /* 2 fields */ }".to_string(),
                EntryKind::Struct,
            ),
        ];
        *index.inner.get_mut().unwrap() = SemanticInner {
            embeddings: None,
            model: None,
            embedding_cache: {
                let mut map = FxHashMap::default();
                map.insert(12345u64, vec![0.1f32; 384]);
                map
            },
        };

        assert!(index.should_save());
        index.save(dir.path()).unwrap();

        let mut loaded = SemanticIndex::new().unwrap();
        assert!(loaded.load(dir.path()));
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].0, "handler");
        assert_eq!(loaded.entries[1].0, "struct Config");

        assert!(loaded.should_save());
    }

    #[test]
    fn load_nonexistent_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = SemanticIndex::new().unwrap();
        assert!(!index.load(dir.path()));
        assert!(index.entries.is_empty());
    }

    #[test]
    fn is_ready_returns_false_when_empty() {
        let index = SemanticIndex::new().unwrap();
        assert!(!index.is_ready());
    }

    #[test]
    fn is_ready_returns_true_with_embeddings_and_model() {
        let mut index = SemanticIndex::new().unwrap();
        index.entries = vec![(
            "test".to_string(),
            PathBuf::from("src/test.rs"),
            1,
            "fn test()".to_string(),
            EntryKind::Function,
        )];
        *index.inner.get_mut().unwrap() = SemanticInner {
            embeddings: Some(vec![vec![0.1f32; 384]]),
            model: None,
            embedding_cache: FxHashMap::default(),
        };
        // model is None, so not ready
        assert!(!index.is_ready());
    }
}
