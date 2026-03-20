//! Semantic search using embeddings
//!
//! Default: text-based substring search (no extra deps).
//! With `semantic` feature: local BGE embeddings via fastembed v5.
//! NOTE: `semantic` feature adds ~50MB to binary via ONNX runtime.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::layers::CallGraphLayer;
use crate::types::{FileAnalysis, SearchResult};

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Mutex;

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
}

/// Internal entry type shared between functions and structs
type Entry = (String, PathBuf, usize, String, EntryKind);

pub struct SemanticIndex {
    entries: Vec<Entry>,
    embeddings: Option<Vec<Vec<f32>>>,
    model: Option<Mutex<TextEmbedding>>,
}

impl SemanticIndex {
    pub fn new() -> Result<Self> {
        Ok(Self {
            entries: Vec::new(),
            embeddings: None,
            model: None,
        })
    }

    /// Build index from pre-computed file analyses
    pub async fn build(
        &mut self,
        analyses: &[FileAnalysis],
        call_graph: &CallGraphLayer,
        cache_dir: &Path,
    ) -> Result<()> {
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
        self.entries = entries;

        let model_cache = cache_dir.join("fastembed");
        match TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_cache_dir(model_cache),
        ) {
            Ok(mut model) => {
                let texts: Vec<String> = self
                    .entries
                    .iter()
                    .filter(|(_, _, _, _, kind)| matches!(kind, EntryKind::Function))
                    .map(|(name, _, _, sig, _)| {
                        let mut text = format!("{} {}", name, sig);
                        if let Ok(callers) = call_graph.get_backward_calls(name) {
                            let names: Vec<&str> =
                                callers.iter().map(|c| c.function.as_str()).collect();
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

                match model.embed(texts, None) {
                    Ok(embeddings) => {
                        self.embeddings = Some(embeddings);
                        self.model = Some(Mutex::new(model));
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

    /// Backward-compatible warm using ASTLayer (delegates to build)
    pub async fn warm(
        &mut self,
        ast: &crate::layers::ASTLayer,
        call_graph: &CallGraphLayer,
        cache_dir: &Path,
    ) -> Result<()> {
        let analyses: Vec<FileAnalysis> = ast.file_analyses().into_iter().cloned().collect();
        self.build(&analyses, call_graph, cache_dir).await
    }

    /// Whether embeddings were generated (only save if true)
    pub fn should_save(&self) -> bool {
        self.embeddings.is_some()
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

        let cache = SemanticCache {
            version: CACHE_VERSION.to_string(),
            entries,
            embeddings: self.embeddings.clone(),
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
                    _ => return None,
                };
                Some((name, file, line, sig, kind))
            })
            .collect();

        self.embeddings = cache.embeddings;

        true
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        if let (Some(model), Some(embeddings)) = (&self.model, &self.embeddings) {
            let mut model = model
                .lock()
                .map_err(|e| crate::error::Error::Semantic(e.to_string()))?;
            match model.embed(vec![query.to_string()], None) {
                Ok(query_emb) => {
                    let q = &query_emb[0];
                    let mut scored: Vec<(usize, f32)> = embeddings
                        .iter()
                        .enumerate()
                        .map(|(i, emb)| (i, cosine_similarity(q, emb)))
                        .collect();
                    scored
                        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                    return Ok(scored
                        .into_iter()
                        .take(limit)
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
        index.embeddings = Some(vec![vec![0.1; 384]]);

        assert!(index.should_save());
        index.save(dir.path()).unwrap();

        let mut loaded = SemanticIndex::new().unwrap();
        assert!(loaded.load(dir.path()));
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].0, "handler");
        assert_eq!(loaded.entries[1].0, "struct Config");

        // Embeddings and save flag preserved through roundtrip
        assert!(loaded.should_save());
    }

    #[test]
    fn load_nonexistent_returns_false() {
        let dir = tempfile::tempdir().unwrap();
        let mut index = SemanticIndex::new().unwrap();
        assert!(!index.load(dir.path()));
        assert!(index.entries.is_empty());
    }
}
