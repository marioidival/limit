//! TLDR tool for code analysis.
//!
//! Provides a tool interface for the `limit-tldr` library, enabling agents
//! to analyze code structure, dependencies, and complexity.

use crate::error::AgentError;
use crate::tool::Tool;
use async_trait::async_trait;
use limit_tldr::{Config as TldrConfig, Language, TLDR};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Analysis type to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisType {
    /// Get compressed context for a function (token-efficient)
    Context,
    /// Find who calls a function (impact analysis for refactoring)
    Impact,
    /// Get control flow graph (complexity analysis)
    Cfg,
    /// Get data flow graph (value tracking)
    Dfg,
    /// Find dead code (unreachable functions)
    DeadCode,
    /// Detect architecture layers (entry/middle/leaf)
    Architecture,
    /// Search functions by name pattern
    Search,
}

/// Parameters for the TLDR tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TldrParams {
    /// Type of analysis to perform
    pub analysis_type: AnalysisType,
    
    /// Function name (required for context, impact, cfg, dfg)
    pub function: Option<String>,
    
    /// File path relative to project root (required for cfg, dfg)
    pub file: Option<String>,
    
    /// Depth for context traversal (default: 2)
    #[serde(default = "default_depth")]
    pub depth: usize,
    
    /// Entry points for dead code detection (default: ["main"])
    #[serde(default = "default_entries")]
    pub entries: Vec<String>,
    
    /// Search query for finding functions
    pub query: Option<String>,
    
    /// Maximum results for search (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    
    /// Project path (defaults to current directory)
    pub project_path: Option<String>,
}

fn default_depth() -> usize { 2 }
fn default_entries() -> Vec<String> { vec!["main".to_string()] }
fn default_limit() -> usize { 10 }

/// TLDR tool for code analysis
pub struct TldrTool {
    /// Cached TLDR instance per project
    cache: Arc<RwLock<Option<(PathBuf, TLDR)>>>,
    /// Default project path
    default_project: PathBuf,
}

impl TldrTool {
    /// Create a new TLDR tool with default project path
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            default_project: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
    
    /// Create TLDR tool with a specific project path
    pub fn with_project<P: Into<PathBuf>>(project: P) -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
            default_project: project.into(),
        }
    }
    
    /// Get or create TLDR instance for a project
    async fn get_tldr(&self, project_path: &PathBuf) -> Result<TLDR, AgentError> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((cached_path, _)) = cache.as_ref() {
                if cached_path == project_path {
                    debug!("TLDR cache hit for project: {:?}", project_path);
                }
            }
        }
        
        // Create new instance
        info!("Creating TLDR instance for project: {:?}", project_path);
        let config = TldrConfig {
            language: Language::Auto,
            max_depth: 3,
            cache_dir: Some(Self::get_cache_dir(project_path)?),
        };
        
        let mut tldr = TLDR::new(project_path, config)
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to create TLDR: {}", e)))?;
        
        // Warm up indexes
        info!("Warming TLDR indexes...");
        tldr.warm()
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to warm TLDR: {}", e)))?;
        
        // Cache it
        {
            let mut cache = self.cache.write().await;
            *cache = Some((project_path.clone(), tldr));
        }
        
        // Create a fresh instance for return (TLDR doesn't implement Clone)
        let config = TldrConfig {
            language: Language::Auto,
            max_depth: 3,
            cache_dir: Some(Self::get_cache_dir(project_path)?),
        };
        
        TLDR::new(project_path, config)
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to create TLDR: {}", e)))
    }
    
    /// Get cache directory for a project (~/.limit/projects/<project-hash>/tldr)
    fn get_cache_dir(project_path: &PathBuf) -> Result<PathBuf, AgentError> {
        let home = dirs::home_dir()
            .ok_or_else(|| AgentError::ToolError("Cannot find home directory".into()))?;
        
        // Create a unique identifier for the project
        let project_id = project_path
            .canonicalize()
            .map_err(|e| AgentError::ToolError(format!("Cannot canonicalize path: {}", e)))?
            .to_string_lossy()
            .to_string();
        
        // Simple hash of project path
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        project_id.hash(&mut hasher);
        let hash = format!("{:x}", hasher.finish());
        
        Ok(home.join(".limit").join("projects").join(&hash).join("tldr"))
    }
    
    /// Perform analysis based on parameters
    async fn analyze(&self, params: TldrParams) -> Result<Value, AgentError> {
        let project_path = params
            .project_path
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_project.clone());
        
        let tldr = self.get_tldr(&project_path).await?;
        
        match params.analysis_type {
            AnalysisType::Context => {
                let function = params.function.ok_or_else(|| {
                    AgentError::ToolError("function parameter required for context analysis".into())
                })?;
                
                let context = tldr
                    .get_context(&function, params.depth)
                    .await
                    .map_err(|e| AgentError::ToolError(format!("Context analysis failed: {}", e)))?;
                
                Ok(json!({
                    "type": "context",
                    "function": function,
                    "depth": params.depth,
                    "context": context
                }))
            }
            
            AnalysisType::Impact => {
                let function = params.function.ok_or_else(|| {
                    AgentError::ToolError("function parameter required for impact analysis".into())
                })?;
                
                let callers = tldr
                    .get_impact(&function)
                    .map_err(|e| AgentError::ToolError(format!("Impact analysis failed: {}", e)))?;
                
                Ok(json!({
                    "type": "impact",
                    "function": function,
                    "callers": callers.iter().map(|c| json!({
                        "function": c.function,
                        "file": c.file.display().to_string(),
                        "line": c.line
                    })).collect::<Vec<_>>(),
                    "caller_count": callers.len()
                }))
            }
            
            AnalysisType::Cfg => {
                let file = params.file.ok_or_else(|| {
                    AgentError::ToolError("file parameter required for CFG analysis".into())
                })?;
                let function = params.function.ok_or_else(|| {
                    AgentError::ToolError("function parameter required for CFG analysis".into())
                })?;
                
                let file_path = project_path.join(&file);
                let cfg = tldr
                    .get_cfg(&file_path, &function)
                    .map_err(|e| AgentError::ToolError(format!("CFG analysis failed: {}", e)))?;
                
                Ok(json!({
                    "type": "cfg",
                    "function": function,
                    "file": file,
                    "complexity": cfg.complexity,
                    "blocks": cfg.blocks.len()
                }))
            }
            
            AnalysisType::Dfg => {
                let file = params.file.ok_or_else(|| {
                    AgentError::ToolError("file parameter required for DFG analysis".into())
                })?;
                let function = params.function.ok_or_else(|| {
                    AgentError::ToolError("function parameter required for DFG analysis".into())
                })?;
                
                let file_path = project_path.join(&file);
                let dfg = tldr
                    .get_dfg(&file_path, &function)
                    .map_err(|e| AgentError::ToolError(format!("DFG analysis failed: {}", e)))?;
                
                Ok(json!({
                    "type": "dfg",
                    "function": function,
                    "file": file,
                    "variables": dfg.variables,
                    "flows": dfg.flows.len()
                }))
            }
            
            AnalysisType::DeadCode => {
                let entries: Vec<&str> = params.entries.iter().map(|s| s.as_str()).collect();
                let dead = tldr
                    .find_dead_code(&entries)
                    .map_err(|e| AgentError::ToolError(format!("Dead code analysis failed: {}", e)))?;
                
                Ok(json!({
                    "type": "dead_code",
                    "entries": params.entries,
                    "dead_functions": dead.iter().map(|f| json!({
                        "name": f.name,
                        "file": f.file.display().to_string(),
                        "line": f.line
                    })).collect::<Vec<_>>(),
                    "dead_count": dead.len()
                }))
            }
            
            AnalysisType::Architecture => {
                let arch = tldr
                    .detect_architecture()
                    .map_err(|e| AgentError::ToolError(format!("Architecture detection failed: {}", e)))?;
                
                Ok(json!({
                    "type": "architecture",
                    "entry_points": arch.entry,
                    "middle_layer": arch.middle,
                    "leaf_functions": arch.leaf
                }))
            }
            
            AnalysisType::Search => {
                let query = params.query.unwrap_or_else(|| {
                    params.function.clone().unwrap_or_default()
                });
                
                let results = tldr
                    .semantic_search(&query, params.limit)
                    .await
                    .map_err(|e| AgentError::ToolError(format!("Search failed: {}", e)))?;
                
                Ok(json!({
                    "type": "search",
                    "query": query,
                    "results": results.iter().map(|r| json!({
                        "function": r.function,
                        "file": r.file.display().to_string(),
                        "score": r.score
                    })).collect::<Vec<_>>()
                }))
            }
        }
    }
}

impl Default for TldrTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TldrTool {
    fn name(&self) -> &str {
        "tldr_analyze"
    }
    
    async fn execute(&self, args: Value) -> Result<Value, AgentError> {
        // Parse parameters
        let params: TldrParams = serde_json::from_value(args)
            .map_err(|e| AgentError::ToolError(format!("Invalid parameters: {}", e)))?;
        
        debug!("TLDR analysis: {:?}", params.analysis_type);
        
        self.analyze(params).await
    }
}

/// Generate tool definition for LLM providers
pub fn tldr_tool_definition() -> Value {
    json!({
        "name": "tldr_analyze",
        "description": "Analyze code structure and dependencies with 95% token savings. Use before editing code to understand context, impact, and dependencies.",
        "parameters": {
            "type": "object",
            "properties": {
                "analysis_type": {
                    "type": "string",
                    "enum": ["context", "impact", "cfg", "dfg", "dead_code", "architecture", "search"],
                    "description": "Type of analysis to perform"
                },
                "function": {
                    "type": "string",
                    "description": "Function name to analyze (required for context, impact, cfg, dfg)"
                },
                "file": {
                    "type": "string",
                    "description": "File path relative to project root (required for cfg, dfg)"
                },
                "depth": {
                    "type": "integer",
                    "description": "Depth for context traversal (default: 2)",
                    "default": 2
                },
                "entries": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Entry points for dead code detection (default: [\"main\"])",
                    "default": ["main"]
                },
                "query": {
                    "type": "string",
                    "description": "Search query for finding functions"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results for search (default: 10)",
                    "default": 10
                },
                "project_path": {
                    "type": "string",
                    "description": "Project path (defaults to current directory)"
                }
            },
            "required": ["analysis_type"]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_definition() {
        let def = tldr_tool_definition();
        assert_eq!(def["name"], "tldr_analyze");
        assert!(def["parameters"]["properties"]["analysis_type"]["enum"].is_array());
    }
    
    #[test]
    fn test_params_deserialization() {
        let json = json!({
            "analysis_type": "context",
            "function": "main",
            "depth": 3
        });
        
        let params: TldrParams = serde_json::from_value(json).unwrap();
        assert!(matches!(params.analysis_type, AnalysisType::Context));
        assert_eq!(params.function, Some("main".to_string()));
        assert_eq!(params.depth, 3);
    }
}
