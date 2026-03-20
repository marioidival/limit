//! TLDR - Code Analysis That Actually Fits In Context
//!
//! The Core Insight: LLMs can't read your entire codebase. So we extract the
//! structure, trace the dependencies, and give them exactly what they need—
//! at **95% fewer tokens** than raw code.
//!
//! # Architecture
//!
//! TLDR uses a 5-layer architecture for code analysis:
//!
//! - **Layer 1 (AST)**: Structure - "What functions exist?"
//! - **Layer 2 (Call Graph)**: Dependencies - "Who calls what?"
//! - **Layer 3 (CFG)**: Control Flow - "How complex is this?"
//! - **Layer 4 (DFG)**: Data Flow - "Where does this value come from?"
//! - **Layer 5 (PDG)**: Program Dependence - "What affects this line?"
//!
//! # Example
//!
//! ```no_run
//! use limit_tldr::{TLDR, Config};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut tldr = TLDR::new("./my-project", Config::default()).await?;
//!     
//!     // Build indexes
//!     tldr.warm().await?;
//!     
//!     // Get function context for LLM
//!     let context = tldr.get_context("process_data", 2).await?;
//!     println!("{}", context);
//!     
//!     // Impact analysis - who calls this function?
//!     let callers = tldr.get_impact("hash_password")?;
//!     for caller in callers {
//!         println!("{}:{}", caller.file.display(), caller.line);
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod coordinator;
pub mod error;
pub mod layers;
pub mod parsers;
pub mod semantic;
pub mod types;
pub mod utils;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use cache::CacheManager;
use coordinator::ParseCoordinator;
use layers::{
    ast::ASTLayer, call_graph::CallGraphLayer, cfg::CFGLayer, dfg::DFGLayer, pdg::PDGLayer,
};
use semantic::SemanticIndex;

pub use error::{Error, Result};
pub use types::*;

/// Main TLDR instance for code analysis
pub struct TLDR {
    project_path: PathBuf,
    #[allow(dead_code)]
    config: Config,
    cache: CacheManager,
    ast: ASTLayer,
    call_graph: CallGraphLayer,
    cfg: CFGLayer,
    dfg: DFGLayer,
    pdg: PDGLayer,
    semantic: SemanticIndex,
}

/// Configuration for TLDR
#[derive(Debug, Clone)]
pub struct Config {
    /// Language to analyze
    pub language: Language,
    /// Maximum depth for call graph traversal
    pub max_depth: usize,
    /// Cache directory (defaults to .tldr/cache)
    pub cache_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: Language::Auto,
            max_depth: 3,
            cache_dir: None,
        }
    }
}

impl TLDR {
    /// Create a new TLDR instance for a project
    pub async fn new<P: AsRef<Path>>(project_path: P, config: Config) -> Result<Self> {
        let project_path = project_path
            .as_ref()
            .canonicalize()
            .map_err(|e| Error::PathNotFound(project_path.as_ref().display().to_string(), e))?;

        let cache_dir = config
            .cache_dir
            .clone()
            .unwrap_or_else(|| project_path.join(".tldr").join("cache"));

        let cache = CacheManager::new(cache_dir)?;

        // Initialize all layers
        let ast = ASTLayer::new(config.language);
        let call_graph = CallGraphLayer::new();
        let cfg = CFGLayer::new();
        let dfg = DFGLayer::new();
        let pdg = PDGLayer::new();
        let semantic = SemanticIndex::new()?;

        Ok(Self {
            project_path,
            config,
            cache,
            ast,
            call_graph,
            cfg,
            dfg,
            pdg,
            semantic,
        })
    }

    /// Build/warm all indexes for the project
    pub async fn warm(&mut self) -> Result<()> {
        // Clean up old cache format
        self.cache.cleanup();

        // Try loading semantic index from cache (skip rebuild if nothing changed)
        if self.semantic.load(self.cache.cache_dir()) {
            tracing::info!("semantic: loaded from cache");
        }

        // ParseCoordinator: discover → hash check → parallel parse → cache
        let mut coordinator = ParseCoordinator::new(
            self.project_path.clone(),
            self.config.language,
            // Create a fresh cache ref for the coordinator
            CacheManager::new(self.cache.cache_dir().to_path_buf())?,
        );
        let analyses = coordinator.warm().await?;

        // Populate AST layer from parsed analyses (for public API)
        self.ast.populate(&analyses);

        // Build call graph from pre-computed analyses
        self.call_graph.build(&analyses);

        // Build semantic index
        self.semantic.build(&analyses, &self.call_graph).await?;

        // Persist semantic index if embeddings were generated
        if self.semantic.should_save() {
            self.semantic.save(self.cache.cache_dir())?;
        }

        Ok(())
    }

    /// Get LLM-ready context for a function
    pub async fn get_context(&self, function: &str, depth: usize) -> Result<String> {
        // Find the function
        let func_info = self
            .ast
            .find_function(function)?
            .ok_or_else(|| Error::FunctionNotFound(function.to_string()))?;

        // Get call graph
        let calls = self.call_graph.get_forward_calls(function)?;
        let called_by = self.call_graph.get_backward_calls(function)?;

        // Get CFG complexity
        let cfg_info = self.cfg.analyze(&func_info)?;

        // Build context string
        let mut context = String::new();

        // Function signature
        context.push_str(&format!("## Function: {}\n\n", function));
        context.push_str(&format!("**Signature:** `{}`\n", func_info.signature));

        if let Some(doc) = &func_info.docstring {
            context.push_str(&format!("**Description:** {}\n", doc));
        }

        // Complexity
        context.push_str(&format!(
            "**Complexity:** {} (cyclomatic)\n",
            cfg_info.complexity
        ));

        // Calls
        if !calls.is_empty() {
            context.push_str("\n**Calls:**\n");
            for call in &calls {
                context.push_str(&format!("- `{}`\n", call));
            }
        }

        // Called by
        if !called_by.is_empty() {
            context.push_str("\n**Called by:**\n");
            for caller in &called_by {
                context.push_str(&format!(
                    "- `{}` ({}:{})\n",
                    caller.function,
                    caller.file.display(),
                    caller.line
                ));
            }
        }

        // Traverse deeper if requested
        if depth > 0 {
            for call in &calls {
                if let Ok(Some(callee_info)) = self.ast.find_function(call) {
                    context.push_str(&format!("\n### Callee: `{}`\n", call));
                    context.push_str(&format!(
                        "- **File:** {}:{}\n",
                        callee_info.file.display(),
                        callee_info.line
                    ));
                    context.push_str(&format!("- **Signature:** `{}`\n", callee_info.signature));
                }
            }
        }

        Ok(context)
    }

    /// Get impact analysis for a function (who calls this?)
    pub fn get_impact(&self, function: &str) -> Result<Vec<CallerInfo>> {
        self.call_graph.get_backward_calls(function)
    }

    /// Get control flow graph for a function
    pub fn get_cfg(&self, file: &Path, function: &str) -> Result<CFGInfo> {
        let func_info = self
            .ast
            .find_function_in_file(file, function)?
            .ok_or_else(|| {
                Error::FunctionNotFound(format!("{} in {}", function, file.display()))
            })?;

        self.cfg.analyze(&func_info)
    }

    /// Get data flow graph for a function
    pub fn get_dfg(&self, file: &Path, function: &str) -> Result<DFGInfo> {
        let func_info = self
            .ast
            .find_function_in_file(file, function)?
            .ok_or_else(|| {
                Error::FunctionNotFound(format!("{} in {}", function, file.display()))
            })?;

        self.dfg.analyze(&func_info)
    }

    /// Get program slice for a line
    pub fn get_slice(&self, file: &Path, function: &str, line: usize) -> Result<SliceInfo> {
        let func_info = self
            .ast
            .find_function_in_file(file, function)?
            .ok_or_else(|| {
                Error::FunctionNotFound(format!("{} in {}", function, file.display()))
            })?;

        self.pdg.slice(&func_info, line)
    }

    /// Semantic search for code
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.semantic.search(query, limit).await
    }

    /// Get semantic index reference
    pub fn semantic_index(&self) -> &SemanticIndex {
        &self.semantic
    }

    /// Find dead code
    pub fn find_dead_code(&self, entries: &[&str]) -> Result<Vec<FunctionInfo>> {
        self.call_graph.find_unreachable(entries)
    }

    /// Detect architecture layers
    pub fn detect_architecture(&self) -> Result<ArchitectureInfo> {
        self.call_graph.detect_layers()
    }

    /// Find a function by name
    pub async fn find_function(&self, name: &str) -> Result<Option<FunctionInfo>> {
        self.ast.find_function(name)
    }

    /// Find all functions matching name (for disambiguation when name is ambiguous)
    pub fn find_all_functions(&self, name: &str) -> Vec<FunctionInfo> {
        self.ast
            .find_all_functions(name)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Find a function, preferring one in the given file
    pub fn find_function_in(&self, name: &str, file: &Path) -> Result<Option<FunctionInfo>> {
        self.ast.find_function_preferring_file(name, file)
    }

    /// Find a class/struct by name
    pub fn find_class(&self, name: &str) -> Result<Option<ClassInfo>> {
        self.ast.find_class(name)
    }

    /// Find a class/struct, preferring one in the given file
    pub fn find_class_in(&self, name: &str, file: &Path) -> Result<Option<ClassInfo>> {
        self.ast.find_class_preferring_file(name, file)
    }

    /// Get the project path
    pub fn project_path(&self) -> &Path {
        &self.project_path
    }

    /// Get file tree of indexed files
    pub fn tree(&self) -> Result<Vec<PathBuf>> {
        Ok(self.ast.files())
    }

    /// Get structure (functions, classes) per file
    pub fn structure(&self) -> Result<Vec<&FileAnalysis>> {
        Ok(self.ast.file_analyses())
    }

    /// Text pattern search across function names
    pub fn search(&self, pattern: &str) -> Result<Vec<FunctionInfo>> {
        let pattern_lower = pattern.to_lowercase();
        Ok(self
            .ast
            .all_functions()
            .into_iter()
            .filter(|f| f.name.to_lowercase().contains(&pattern_lower))
            .cloned()
            .collect())
    }

    /// Extract full file analysis by filename
    pub fn extract(&self, file_name: &str) -> Result<FileAnalysis> {
        self.ast
            .get_by_name(file_name)
            .cloned()
            .ok_or_else(|| Error::FileNotFound(PathBuf::from(file_name)))
    }

    /// Get forward calls for a function
    pub fn get_calls(&self, function: &str) -> Result<Vec<String>> {
        self.call_graph.get_forward_calls(function)
    }

    /// Get imports for a file
    pub fn get_imports(&self, file_name: &str) -> Result<Vec<ImportInfo>> {
        Ok(self.extract(file_name)?.imports)
    }

    /// Find files that import a module (returns file paths)
    pub fn get_importers(&self, module: &str) -> Result<Vec<PathBuf>> {
        let module_lower = module.to_lowercase();
        let mut files = Vec::new();

        for analysis in self.ast.iter_analyses() {
            for imp in &analysis.imports {
                if imp.module.to_lowercase().contains(&module_lower)
                    || imp
                        .names
                        .iter()
                        .any(|n: &String| n.to_lowercase() == module_lower)
                {
                    files.push(analysis.file.clone());
                    break;
                }
            }
        }

        Ok(files)
    }
}
