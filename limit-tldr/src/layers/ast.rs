//! Layer 1: AST (Abstract Syntax Tree) - "What exists?"
//!
//! Extracts structure from source files: functions, classes, imports.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cache::CacheManager;
use crate::error::{Error, Result};
use crate::parsers::tree_sitter::TreeSitterParser;
use crate::types::{ClassInfo, FileAnalysis, FunctionInfo, Language};

/// AST analysis layer
pub struct ASTLayer {
    language: Language,
    file_cache: HashMap<PathBuf, FileAnalysis>,
}

impl ASTLayer {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            file_cache: HashMap::new(),
        }
    }

    /// Warm up the cache by parsing all files
    pub async fn warm(&mut self, project_path: &Path, cache: &mut CacheManager) -> Result<()> {
        let files = self.discover_files(project_path)?;

        for file in files {
            if !cache.is_cached(&file)? {
                let analysis = self.analyze_file(&file).await?;
                cache.store_file(&file, &analysis)?;
                self.file_cache.insert(file, analysis);
            } else if let Some(analysis) = cache.get_file(&file)? {
                self.file_cache.insert(file, analysis);
            } else {
                // Hash says cached but cache file missing — re-parse
                let analysis = self.analyze_file(&file).await?;
                cache.store_file(&file, &analysis)?;
                self.file_cache.insert(file, analysis);
            }
        }

        Ok(())
    }

    /// Discover source files in a project
    fn discover_files(&self, project_path: &Path) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;

        let mut files = Vec::new();

        for entry in WalkDir::new(project_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip hidden directories and common non-source directories
            // Only check components relative to the project root
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

            // Check extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let lang = Language::from_extension(ext);

                if let Some(lang) = lang {
                    if self.language == Language::Auto || self.language == lang {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(files)
    }

    /// Analyze a single file
    pub async fn analyze_file(&self, path: &Path) -> Result<FileAnalysis> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| Error::ParseError {
                file: path.display().to_string(),
                message: "No file extension".to_string(),
            })?;

        let language = Language::from_extension(ext)
            .ok_or_else(|| Error::LanguageNotSupported(ext.to_string()))?;

        let source = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Error::PathNotFound(path.display().to_string(), e))?;

        let parser = TreeSitterParser::new();
        let mut analysis = parser.parse(&source, path, language)?;
        analysis.file = path.to_path_buf();

        Ok(analysis)
    }

    /// Find a function by name across all files
    pub fn find_function(&self, name: &str) -> Result<Option<FunctionInfo>> {
        for analysis in self.file_cache.values() {
            for func in &analysis.functions {
                if func.name == name {
                    return Ok(Some(func.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Find all functions matching name across all files (for disambiguation)
    pub fn find_all_functions(&self, name: &str) -> Vec<&FunctionInfo> {
        self.file_cache
            .values()
            .flat_map(|a| a.functions.iter())
            .filter(|f| f.name == name)
            .collect()
    }

    /// Find a function, preferring one in the given file path
    pub fn find_function_preferring_file(
        &self,
        name: &str,
        preferred_file: &Path,
    ) -> Result<Option<FunctionInfo>> {
        // First try exact file match
        if let Some(analysis) = self.file_cache.get(preferred_file) {
            for func in &analysis.functions {
                if func.name == name {
                    return Ok(Some(func.clone()));
                }
            }
        }

        // Fallback: try file path contains the preferred path
        let preferred_str = preferred_file.to_string_lossy().to_lowercase();
        for analysis in self.file_cache.values() {
            if analysis
                .file
                .to_string_lossy()
                .to_lowercase()
                .contains(&preferred_str)
            {
                for func in &analysis.functions {
                    if func.name == name {
                        return Ok(Some(func.clone()));
                    }
                }
            }
        }

        // Last resort: return first match
        self.find_function(name)
    }

    /// Find a function in a specific file
    pub fn find_function_in_file(&self, file: &Path, name: &str) -> Result<Option<FunctionInfo>> {
        if let Some(analysis) = self.file_cache.get(file) {
            for func in &analysis.functions {
                if func.name == name {
                    return Ok(Some(func.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Get all functions
    pub fn all_functions(&self) -> Vec<&FunctionInfo> {
        self.file_cache
            .values()
            .flat_map(|a| a.functions.iter())
            .collect()
    }

    /// Get all classes/structs
    pub fn all_classes(&self) -> Vec<&ClassInfo> {
        self.file_cache
            .values()
            .flat_map(|a| a.classes.iter())
            .collect()
    }

    /// Find a class/struct by name across all files
    pub fn find_class(&self, name: &str) -> Result<Option<ClassInfo>> {
        for analysis in self.file_cache.values() {
            for cls in &analysis.classes {
                if cls.name == name {
                    return Ok(Some(cls.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Find a class/struct by name, preferring one in the given file path
    pub fn find_class_preferring_file(
        &self,
        name: &str,
        preferred_file: &Path,
    ) -> Result<Option<ClassInfo>> {
        if let Some(analysis) = self.file_cache.get(preferred_file) {
            for cls in &analysis.classes {
                if cls.name == name {
                    return Ok(Some(cls.clone()));
                }
            }
        }

        // Fallback: search all files
        self.find_class(name)
    }

    /// Get all indexed file paths
    pub fn files(&self) -> Vec<PathBuf> {
        self.file_cache.keys().cloned().collect()
    }

    /// Get all file analyses
    pub fn file_analyses(&self) -> Vec<&FileAnalysis> {
        self.file_cache.values().collect()
    }

    /// Get file analysis by filename
    pub fn get_by_name(&self, file_name: &str) -> Option<&FileAnalysis> {
        self.file_cache
            .values()
            .find(|a| a.file.file_name().map(|n| n == file_name).unwrap_or(false))
    }

    /// Iterate over all file analyses
    pub fn iter_analyses(&self) -> impl Iterator<Item = &FileAnalysis> {
        self.file_cache.values()
    }
}
