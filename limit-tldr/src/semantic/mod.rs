//! Semantic search using embeddings
//!
//! Default: text-based substring search (no extra deps).
//! With `semantic` feature: local BGE embeddings via fastembed v5.
//! NOTE: `semantic` feature adds ~50MB to binary via ONNX runtime.

use std::path::PathBuf;

use crate::error::Result;
use crate::layers::{ASTLayer, CallGraphLayer};
use crate::types::SearchResult;

#[cfg(feature = "semantic")]
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
#[cfg(feature = "semantic")]
use std::sync::Mutex;

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
    #[cfg(feature = "semantic")]
    embeddings: Option<Vec<Vec<f32>>>,
    #[cfg(feature = "semantic")]
    model: Option<Mutex<TextEmbedding>>,
}

impl SemanticIndex {
    pub fn new() -> Result<Self> {
        Ok(Self {
            entries: Vec::new(),
            #[cfg(feature = "semantic")]
            embeddings: None,
            #[cfg(feature = "semantic")]
            model: None,
        })
    }

    pub async fn warm(&mut self, ast: &ASTLayer, call_graph: &CallGraphLayer) -> Result<()> {
        // Index functions
        let mut entries: Vec<Entry> = ast
            .all_functions()
            .into_iter()
            .map(|f| {
                (
                    f.name.clone(),
                    f.file.clone(),
                    f.line,
                    f.signature.clone(),
                    EntryKind::Function,
                )
            })
            .collect();

        // Index structs/classes (prefix with "struct " to distinguish)
        let struct_entries: Vec<Entry> = ast
            .all_classes()
            .into_iter()
            .map(|c| {
                let name = format!("struct {}", c.name);
                let sig = format!("struct {} {{ /* {} fields */ }}", c.name, c.fields.len());
                (name, c.file.clone(), c.line, sig, EntryKind::Struct)
            })
            .collect();

        entries.extend(struct_entries);
        self.entries = entries;

        #[cfg(not(feature = "semantic"))]
        let _ = call_graph;

        #[cfg(feature = "semantic")]
        {
            match TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGESmallENV15)) {
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
        }
        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        #[cfg(feature = "semantic")]
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
            .filter(|(name, file, _, _, _)| {
                name.to_lowercase().contains(&query_lower)
                    || file.to_string_lossy().to_lowercase().contains(&query_lower)
            })
            .map(|(name, file, line, signature, _)| SearchResult {
                function: name.clone(),
                file: file.clone(),
                line: *line,
                score: 1.0,
                signature: signature.clone(),
            })
            .take(limit)
            .collect();

        results.sort_by(|a, b| {
            let a_exact = a.function.to_lowercase() == query_lower;
            let b_exact = b.function.to_lowercase() == query_lower;
            let a_prefix = a.function.to_lowercase().starts_with(&query_lower);
            let b_prefix = b.function.to_lowercase().starts_with(&query_lower);
            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => match (a_prefix, b_prefix) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.function.cmp(&b.function),
                },
            }
        });
        Ok(results)
    }
}

#[cfg(feature = "semantic")]
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
}
