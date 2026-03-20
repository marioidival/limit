//! ParseCoordinator - orchestrates file discovery, caching, and parallel parsing.
//!
//! Replaces ASTLayer::warm() as the single entry point for parsing.
//! Each file is parsed once; all layers receive pre-built FileAnalysis.

use rayon::prelude::*;
use std::path::PathBuf;

use crate::cache::CacheManager;
use crate::error::{Error, Result};
use crate::parsers::tree_sitter::TreeSitterParser;
use crate::types::{FileAnalysis, Language};

/// Orchestrates file discovery, cache checking, and parallel parsing.
pub struct ParseCoordinator {
    project_path: PathBuf,
    language: Language,
    cache: CacheManager,
}

impl ParseCoordinator {
    pub fn new(project_path: PathBuf, language: Language, cache: CacheManager) -> Self {
        Self {
            project_path,
            language,
            cache,
        }
    }

    /// Discover, hash-check, parse (parallel), cache, and return all file analyses.
    pub async fn warm(&mut self) -> Result<Vec<FileAnalysis>> {
        let files = self.discover_files()?;

        // Check hashes (sequential I/O — fast)
        let mut to_parse = Vec::new();
        let mut cached = Vec::new();

        for file in &files {
            if self.cache.is_cached(file)? {
                if let Some(analysis) = self.cache.get_file(file)? {
                    cached.push(analysis);
                } else {
                    // Hash says cached but file missing — re-parse
                    to_parse.push(file.clone());
                }
            } else {
                to_parse.push(file.clone());
            }
        }

        let total = files.len();
        let modified_count = to_parse.len();

        // Parse modified files (parallel CPU via rayon, in blocking context)
        let analyses: Vec<FileAnalysis> = if to_parse.is_empty() {
            Vec::new()
        } else {
            let files_to_parse = to_parse.clone();
            tokio::task::spawn_blocking(move || {
                let parser = TreeSitterParser::new();
                files_to_parse
                    .par_iter()
                    .filter_map(|file| {
                        let ext = file.extension()?.to_str()?;
                        let lang = Language::from_extension(ext)?;
                        let source = std::fs::read_to_string(file).ok()?;
                        parser.parse(&source, file, lang).ok()
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .map_err(|e| Error::Cache(format!("Parse task panicked: {}", e)))?
        };

        // Cache newly parsed files
        for analysis in &analyses {
            self.cache.store_file(&analysis.file, analysis)?;
        }

        let mut all = cached;
        all.extend(analyses);

        tracing::info!(
            "warm: parsed {modified_count}/{total} files ({} cached)",
            total - modified_count
        );

        Ok(all)
    }

    /// Discover source files in the project.
    fn discover_files(&self) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;

        let mut files = Vec::new();

        for entry in WalkDir::new(&self.project_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            let relative = path.strip_prefix(&self.project_path).unwrap_or(path);
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

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if let Some(lang) = Language::from_extension(ext) {
                    if self.language == Language::Auto || self.language == lang {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(files)
    }
}
