//! TLDR tool wrapper for CLI
//!
//! Wraps the limit-agent TldrTool for use in limit-cli

use async_trait::async_trait;
use limit_agent::error::AgentError;
use limit_agent::Tool;
use limit_tldr::{Config as TldrConfig, Language, TLDR};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tracing::{debug, info};

/// Analysis type to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisType {
    Context,
    Impact,
    Cfg,
    Dfg,
    DeadCode,
    Architecture,
    Search,
}

/// Parameters for the TLDR tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TldrParams {
    pub analysis_type: AnalysisType,
    pub function: Option<String>,
    pub file: Option<String>,
    #[serde(default = "default_depth")]
    pub depth: usize,
    #[serde(default = "default_entries")]
    pub entries: Vec<String>,
    pub query: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub project_path: Option<String>,
}

fn default_depth() -> usize {
    2
}
fn default_entries() -> Vec<String> {
    vec!["main".to_string()]
}
fn default_limit() -> usize {
    10
}

/// TLDR tool for code analysis
pub struct TldrTool {
    /// Default project path
    default_project: PathBuf,
}

impl TldrTool {
    /// Create a new TLDR tool
    pub fn new() -> Self {
        Self {
            default_project: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create TLDR tool with a specific project path
    pub fn with_project<P: Into<PathBuf>>(project: P) -> Self {
        Self {
            default_project: project.into(),
        }
    }

    /// Get cache directory for a project (~/.limit/projects/<project-hash>/tldr)
    fn get_cache_dir(project_path: &PathBuf) -> Result<PathBuf, AgentError> {
        let home = dirs::home_dir()
            .ok_or_else(|| AgentError::ToolError("Cannot find home directory".into()))?;

        let project_id = project_path
            .canonicalize()
            .map_err(|e| AgentError::ToolError(format!("Cannot canonicalize path: {}", e)))?
            .to_string_lossy()
            .to_string();

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        project_id.hash(&mut hasher);
        let hash = format!("{:x}", hasher.finish());

        Ok(home.join(".limit").join("projects").join(&hash).join("tldr"))
    }

    /// Get or create TLDR instance for a project
    async fn get_tldr(&self, project_path: &PathBuf) -> Result<TLDR, AgentError> {
        info!("Creating TLDR instance for project: {:?}", project_path);
        let config = TldrConfig {
            language: Language::Auto,
            max_depth: 3,
            cache_dir: Some(Self::get_cache_dir(project_path)?),
        };

        let mut tldr = TLDR::new(project_path, config)
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to create TLDR: {}", e)))?;

        info!("Warming TLDR indexes...");
        tldr.warm()
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to warm TLDR: {}", e)))?;

        Ok(tldr)
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
                let query = params
                    .query
                    .unwrap_or_else(|| params.function.clone().unwrap_or_default());

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
        let params: TldrParams = serde_json::from_value(args)
            .map_err(|e| AgentError::ToolError(format!("Invalid parameters: {}", e)))?;

        debug!("TLDR analysis: {:?}", params.analysis_type);

        self.analyze(params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
