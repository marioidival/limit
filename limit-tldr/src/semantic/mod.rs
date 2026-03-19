//! Semantic search using embeddings
//!
//! Find code by behavior, not just syntax.

use std::path::PathBuf;

use crate::error::Result;
use crate::layers::{ASTLayer, CallGraphLayer};
use crate::types::SearchResult;

/// Index for code search (text-based for now, semantic embeddings planned)
pub struct SemanticIndex {
    functions: Vec<(String, PathBuf, usize, String)>,
}

impl SemanticIndex {
    pub fn new() -> Result<Self> {
        Ok(Self {
            functions: Vec::new(),
        })
    }

    pub async fn warm(&mut self, ast: &ASTLayer, _call_graph: &CallGraphLayer) -> Result<()> {
        self.functions = ast
            .all_functions()
            .into_iter()
            .map(|f| (f.name.clone(), f.file.clone(), f.line, f.signature.clone()))
            .collect();
        Ok(())
    }

    /// Search functions by name pattern (case-insensitive substring match)
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query_lower = query.to_lowercase();

        let mut results: Vec<SearchResult> = self
            .functions
            .iter()
            .filter(|(name, _, _, _)| name.to_lowercase().contains(&query_lower))
            .map(|(name, file, line, signature)| SearchResult {
                function: name.clone(),
                file: file.clone(),
                line: *line,
                score: 1.0, // Text match = perfect score for now
                signature: signature.clone(),
            })
            .take(limit)
            .collect();

        // Sort by relevance: exact match first, then prefix, then substring
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

impl Default for SemanticIndex {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn search_includes_file_path_matches() {
        let mut index = SemanticIndex::new().unwrap();
        index.functions = vec![
            ("run".to_string(), PathBuf::from("src/daemon/mod.rs"), 1, "fn run()".to_string()),
            ("handle".to_string(), PathBuf::from("src/daemon/handler.rs"), 1, "fn handle()".to_string()),
            ("other".to_string(), PathBuf::from("src/other.rs"), 1, "fn other()".to_string()),
        ];

        let results = index.search("daemon", 10).await.unwrap();

        assert!(!results.is_empty(), "Should find results from daemon files");
        assert!(results.iter().any(|r| r.file.to_string_lossy().contains("daemon")));
    }
}
