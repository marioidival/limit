//! Layer 1: AST (Abstract Syntax Tree) - "What exists?"
//!
//! Extracts structure from source files: functions, classes, imports.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cache::CacheManager;
use crate::error::{Error, Result};
use crate::types::{ClassInfo, FileAnalysis, FunctionInfo, ImportInfo, Language, Parameter};

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
            if path.components().any(|c| {
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

        // Simple regex-based parsing for now
        // In production, would use tree-sitter
        let mut analysis = FileAnalysis {
            file: path.to_path_buf(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            language,
        };

        match language {
            Language::Python => self.parse_python(&source, &mut analysis, path),
            Language::Rust => self.parse_rust(&source, &mut analysis, path),
            Language::TypeScript | Language::JavaScript => {
                self.parse_typescript(&source, &mut analysis, path)
            }
            _ => {}
        }

        Ok(analysis)
    }

    /// Simple Python parser
    fn parse_python(&self, source: &str, analysis: &mut FileAnalysis, file: &Path) {
        use regex::Regex;

        // Simple function pattern
        let func_re =
            Regex::new(r"(?m)^(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*(.+))?").unwrap();

        for cap in func_re.captures_iter(source) {
            let name = cap[1].to_string();
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().trim().to_string());

            let params: Vec<Parameter> = params_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let parts: Vec<&str> = s.trim().splitn(2, ':').collect();
                    Parameter {
                        name: parts[0].trim().to_string(),
                        type_annotation: parts.get(1).map(|t| t.trim().to_string()),
                        default_value: None,
                    }
                })
                .collect();

            analysis.functions.push(FunctionInfo {
                name,
                signature: cap[0].to_string(),
                params,
                return_type,
                is_async: cap[0].starts_with("async"),
                line: 0, // Would calculate from position
                end_line: 0,
                file: file.to_path_buf(),
                docstring: None,
                complexity: None,
            });
        }

        // Simple class pattern
        let class_re = Regex::new(r"(?m)^class\s+(\w+)(?:\s*\([^)]*\))?").unwrap();

        for cap in class_re.captures_iter(source) {
            analysis.classes.push(ClassInfo {
                name: cap[1].to_string(),
                methods: Vec::new(),
                fields: Vec::new(),
                line: 0,
                file: file.to_path_buf(),
                docstring: None,
            });
        }

        // Simple import pattern
        let import_re = Regex::new(r"(?m)^import\s+(\S+)|^from\s+(\S+)\s+import\s+(.+)").unwrap();

        for cap in import_re.captures_iter(source) {
            if let Some(module) = cap.get(1) {
                analysis.imports.push(ImportInfo {
                    module: module.as_str().to_string(),
                    names: Vec::new(),
                    alias: None,
                    line: 0,
                });
            } else if let (Some(module), Some(names)) = (cap.get(2), cap.get(3)) {
                let names: Vec<String> = names
                    .as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                analysis.imports.push(ImportInfo {
                    module: module.as_str().to_string(),
                    names,
                    alias: None,
                    line: 0,
                });
            }
        }
    }

    /// Simple Rust parser
    fn parse_rust(&self, source: &str, analysis: &mut FileAnalysis, file: &Path) {
        use regex::Regex;

        let func_re = Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*->\s*(.+?))?\s*\{")
            .unwrap();

        for cap in func_re.captures_iter(source) {
            let name = cap[1].to_string();
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().trim().to_string());

            let params: Vec<Parameter> = params_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let parts: Vec<&str> = s.trim().splitn(2, ':').collect();
                    Parameter {
                        name: parts[0].trim().replace("mut ", "").to_string(),
                        type_annotation: parts.get(1).map(|t| t.trim().to_string()),
                        default_value: None,
                    }
                })
                .collect();

            analysis.functions.push(FunctionInfo {
                name,
                signature: cap[0].lines().next().unwrap_or("").trim().to_string(),
                params,
                return_type,
                is_async: cap[0].contains("async"),
                line: 0,
                end_line: 0,
                file: file.to_path_buf(),
                docstring: None,
                complexity: None,
            });
        }

        let struct_re = Regex::new(r"(?m)^(?:pub\s+)?struct\s+(\w+)").unwrap();

        for cap in struct_re.captures_iter(source) {
            analysis.classes.push(ClassInfo {
                name: cap[1].to_string(),
                methods: Vec::new(),
                fields: Vec::new(),
                line: 0,
                file: file.to_path_buf(),
                docstring: None,
            });
        }

        let use_re = Regex::new(r"(?m)^use\s+(.+);").unwrap();

        for cap in use_re.captures_iter(source) {
            analysis.imports.push(ImportInfo {
                module: cap[1].to_string(),
                names: Vec::new(),
                alias: None,
                line: 0,
            });
        }
    }

    /// Simple TypeScript/JavaScript parser
    fn parse_typescript(&self, source: &str, analysis: &mut FileAnalysis, file: &Path) {
        use regex::Regex;

        let func_re = Regex::new(r"(?m)(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*:\s*(.+?))?\s*\{")
            .unwrap();

        for cap in func_re.captures_iter(source) {
            let name = cap[1].to_string();
            let params_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let return_type = cap.get(3).map(|m| m.as_str().trim().to_string());

            let params: Vec<Parameter> = params_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let parts: Vec<&str> = s.trim().splitn(2, ':').collect();
                    Parameter {
                        name: parts[0].trim().to_string(),
                        type_annotation: parts.get(1).map(|t| t.trim().to_string()),
                        default_value: None,
                    }
                })
                .collect();

            analysis.functions.push(FunctionInfo {
                name,
                signature: cap[0].lines().next().unwrap_or("").trim().to_string(),
                params,
                return_type,
                is_async: cap[0].contains("async"),
                line: 0,
                end_line: 0,
                file: file.to_path_buf(),
                docstring: None,
                complexity: None,
            });
        }

        let import_re =
            Regex::new(r#"import\s+(?:\{([^}]*)\}|(\w+))\s+from\s+['"]([^'"]+)['"]"#).unwrap();

        for cap in import_re.captures_iter(source) {
            let module = cap[3].to_string();

            let names = if let Some(named) = cap.get(1) {
                named
                    .as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            } else if let Some(default) = cap.get(2) {
                vec![default.as_str().to_string()]
            } else {
                Vec::new()
            };

            analysis.imports.push(ImportInfo {
                module,
                names,
                alias: None,
                line: 0,
            });
        }
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
}
