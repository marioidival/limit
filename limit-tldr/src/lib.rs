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
pub mod daemon;
pub mod error;
pub mod layers;
pub mod semantic;
pub mod types;
pub mod utils;

use std::path::{Path, PathBuf};

use layers::{ast::ASTLayer, call_graph::CallGraphLayer, cfg::CFGLayer, dfg::DFGLayer, pdg::PDGLayer};
use semantic::SemanticIndex;
use cache::CacheManager;

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
        let project_path = project_path.as_ref().canonicalize()
            .map_err(|e| Error::PathNotFound(project_path.as_ref().display().to_string(), e))?;
        
        let cache_dir = config.cache_dir.clone()
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
        // Layer 1: Parse all files and extract ASTs
        self.ast.warm(&self.project_path, &mut self.cache).await?;
        
        // Layer 2: Build call graph
        self.call_graph.warm(&self.ast, &mut self.cache).await?;
        
        // Layer 3-5: Build CFG, DFG, PDG for each function
        // These are computed on-demand
        
        // Semantic index
        self.semantic.warm(&self.ast, &self.call_graph).await?;
        
        Ok(())
    }
    
    /// Get LLM-ready context for a function
    pub async fn get_context(&self, function: &str, depth: usize) -> Result<String> {
        // Find the function
        let func_info = self.ast.find_function(function)?
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
        context.push_str(&format!("**Complexity:** {} (cyclomatic)\n", cfg_info.complexity));
        
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
                context.push_str(&format!("- `{}` ({}:{})\n", caller.function, caller.file.display(), caller.line));
            }
        }
        
        // Traverse deeper if requested
        if depth > 0 {
            for call in &calls {
                if let Ok(Some(callee_info)) = self.ast.find_function(call) {
                    context.push_str(&format!("\n### Callee: `{}`\n", call));
                    context.push_str(&format!("- **File:** {}:{}\n", callee_info.file.display(), callee_info.line));
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
        let func_info = self.ast.find_function_in_file(file, function)?
            .ok_or_else(|| Error::FunctionNotFound(format!("{} in {}", function, file.display())))?;
        
        self.cfg.analyze(&func_info)
    }
    
    /// Get data flow graph for a function
    pub fn get_dfg(&self, file: &Path, function: &str) -> Result<DFGInfo> {
        let func_info = self.ast.find_function_in_file(file, function)?
            .ok_or_else(|| Error::FunctionNotFound(format!("{} in {}", function, file.display())))?;
        
        self.dfg.analyze(&func_info)
    }
    
    /// Get program slice for a line
    pub fn get_slice(&self, file: &Path, function: &str, line: usize) -> Result<SliceInfo> {
        let func_info = self.ast.find_function_in_file(file, function)?
            .ok_or_else(|| Error::FunctionNotFound(format!("{} in {}", function, file.display())))?;
        
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
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.max_depth, 3);
    }
}
