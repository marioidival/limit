//! Semantic search using embeddings
//!
//! Default: text-based substring search (no extra deps).
//! With `semantic` feature: local BGE embeddings via fastembed v5.
//! NOTE: `semantic` feature adds ~50MB to binary via ONNX runtime.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::Result;
use crate::layers::CallGraphLayer;
use crate::types::{FileAnalysis, SearchResult};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use serde::{Deserialize, Serialize};

const CACHE_VERSION: &str = "v3";

/// Serialized semantic index for persistence
#[derive(Serialize, Deserialize)]
struct SemanticCache {
    version: String,
    entries: Vec<(String, PathBuf, usize, String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeddings: Option<Vec<Vec<f32>>>,
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
    /// Pre-computed embedding texts (populated by populate_entries, used by build_embeddings)
    embedding_texts: Vec<String>,
    inner: Mutex<SemanticInner>,
}

impl SemanticIndex {
    pub fn new() -> Result<Self> {
        Ok(Self {
            entries: Vec::new(),
            embedding_texts: Vec::new(),
            inner: Mutex::new(SemanticInner {
                embeddings: None,
                model: None,
            }),
        })
    }

    /// Populate entries from file analyses (fast, ~0ms).
    /// Enables text_search() immediately. Call build_embeddings() separately for semantic search.
    pub fn populate_entries(&mut self, analyses: &[FileAnalysis], call_graph: &CallGraphLayer) {
        // Index functions
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

        // Index structs/classes
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

        // Pre-compute embedding texts for functions (using call graph context)
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

        // Clear stale embeddings if entry count changed (prevents index mismatch)
        let inner = self.inner.get_mut().unwrap();
        if let Some(ref embeddings) = inner.embeddings {
            if embeddings.len() != self.embedding_texts.len() {
                tracing::info!(
                    "semantic: clearing stale embeddings ({} vs {} entries)",
                    embeddings.len(),
                    self.embedding_texts.len()
                );
                inner.embeddings = None;
                inner.model = None;
            }
        }
    }

    /// Build embeddings from pre-computed texts (slow, ~90s for 1500+ functions).
    /// Thread-safe via &self — writes to inner through Mutex.
    /// Intended to be called from a background thread after populate_entries().
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
                tracing::info!("semantic: model loaded, generating embeddings...");
                tracing::info!(
                    "semantic: embedding {} texts...",
                    self.embedding_texts.len()
                );
                match model.embed(self.embedding_texts.clone(), None) {
                    Ok(embeddings) => {
                        tracing::info!(
                            "semantic: embeddings generated ({} vectors)",
                            embeddings.len()
                        );
                        let mut inner = self.inner.lock().unwrap();
                        inner.embeddings = Some(embeddings);
                        inner.model = Some(SendEmbedding(model));
                    }
                    Err(e) => {
                        tracing::warn!("Semantic embedding generation failed: {}", e);
                    }
                }
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

    /// Backward-compatible build (synchronous, blocking)
    pub async fn build(
        &mut self,
        analyses: &[FileAnalysis],
        call_graph: &CallGraphLayer,
        cache_dir: &Path,
    ) -> Result<()> {
        self.populate_entries(analyses, call_graph);
        self.build_embeddings(cache_dir)
    }

    /// Whether embeddings were generated (only save if true)
    pub fn should_save(&self) -> bool {
        self.inner.lock().unwrap().embeddings.is_some()
    }

    /// Check if loaded entries match current analyses (to skip embedding rebuild)
    pub fn needs_rebuild(&self, analyses: &[FileAnalysis]) -> bool {
        if self.entries.is_empty() || self.inner.lock().unwrap().embeddings.is_none() {
            return true;
        }
        let current: Vec<_> = analyses
            .iter()
            .flat_map(|a| {
                a.functions.iter().map(|f| {
                    (
                        f.name.clone(),
                        f.file.clone(),
                        f.line,
                        f.signature.clone(),
                        "fn".to_string(),
                    )
                })
            })
            .chain(analyses.iter().flat_map(|a| {
                a.classes.iter().map(|c| {
                    (
                        format!("struct {}", c.name),
                        c.file.clone(),
                        c.line,
                        format!("struct {} {{ /* {} fields */ }}", c.name, c.fields.len()),
                        "struct".to_string(),
                    )
                })
            }))
            .collect();

        if current.len() != self.entries.len() {
            return true;
        }
        current.iter().zip(self.entries.iter()).any(|(c, loaded)| {
            c.0 != loaded.0 || c.1 != loaded.1 || c.2 != loaded.2 || c.3 != loaded.3
        })
    }

    /// Save semantic index to disk
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
        let cache = SemanticCache {
            version: CACHE_VERSION.to_string(),
            entries,
            embeddings: inner.embeddings.clone(),
        };

        let path = cache_dir.join("semantic_index.json");
        let json = serde_json::to_string_pretty(&cache).map_err(|e| {
            crate::error::Error::Cache(format!("Failed to serialize semantic: {}", e))
        })?;

        std::fs::write(&path, json)
            .map_err(|e| crate::error::Error::Cache(format!("Failed to write semantic: {}", e)))?;

        Ok(())
    }

    /// Load semantic index from disk. Returns true if loaded successfully.
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

        // Store embeddings in inner; model is NOT serialized (load separately via load_model)
        let inner = self.inner.get_mut().unwrap();
        inner.embeddings = cache.embeddings;
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
        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .filter_map(|(name, file, line, signature, _)| {
                let name_lower = name.to_lowercase();
                let name_match = name_lower.contains(&query_lower);
                let file_match = file.to_string_lossy().to_lowercase().contains(&query_lower);

                if !name_match && !file_match {
                    return None;
                }

                // Score: name matches rank higher than file-path-only matches
                let score = if name_lower == query_lower {
                    1.0 // exact name match
                } else if name_lower.starts_with(&query_lower) {
                    0.9 // name starts with query
                } else if name_match {
                    0.7 // name contains query
                } else {
                    0.3 // file path only
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

        // Sort by score descending, then alphabetically
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
            embeddings: Some(vec![vec![0.1f32; 384]]),
            model: None,
        };

        assert!(index.should_save());
        index.save(dir.path()).unwrap();

        let mut loaded = SemanticIndex::new().unwrap();
        assert!(loaded.load(dir.path()));
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].0, "handler");
        assert_eq!(loaded.entries[1].0, "struct Config");

        // Embeddings preserved through roundtrip
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
        };
        // model is None, so not ready
        assert!(!index.is_ready());
    }
}
