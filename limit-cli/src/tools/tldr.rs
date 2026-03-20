//! TLDR tool for code analysis.
//!
//! Provides a tool interface for the `limit-tldr` library, enabling agents
//! to analyze code structure, dependencies, and complexity.

use limit_agent::AgentError;
use limit_agent::Tool;
use crate::tools::warm_guard::WarmGuard;
use async_trait::async_trait;
use limit_tldr::{Config as TldrConfig, Language, TLDR};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Notify, OnceCell};
use tracing::{debug, info, warn};

/// Analysis type to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisType {
    /// Get compressed context for a function (token-efficient)
    Context,
    /// Get source code of a function (use instead of file_read for implementation details)
    Source,
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
    /// Cached TLDR instance (initialized once per session)
    cache: Arc<OnceCell<(PathBuf, Arc<TLDR>)>>,
    /// Default project path
    default_project: PathBuf,
    /// Notify waiters when background warm completes
    warm_notify: Arc<Notify>,
    /// Whether pre_warm has been spawned (lazy, once inside tokio runtime)
    warm_started: Arc<AtomicBool>,
}

impl TldrTool {
    /// Create a new TLDR tool with default project path
    pub fn new() -> Self {
        let default_project = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            cache: Arc::new(OnceCell::new()),
            default_project,
            warm_notify: Arc::new(Notify::new()),
            warm_started: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create TLDR tool with a specific project path
    pub fn with_project<P: Into<PathBuf>>(project: P) -> Self {
        Self {
            cache: Arc::new(OnceCell::new()),
            default_project: project.into(),
            warm_notify: Arc::new(Notify::new()),
            warm_started: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn pre_warm if not already started. Safe to call inside tokio runtime.
    fn ensure_pre_warm_started(&self) {
        if self
            .warm_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let project = self.default_project.clone();
            let cache = Arc::clone(&self.cache);
            let notify = Arc::clone(&self.warm_notify);

            tokio::spawn(async move {
                Self::pre_warm(project, cache, notify).await;
            });
        }
    }

    /// Background warm: check freshness, warm if stale, notify waiters.
    async fn pre_warm(
        project_path: PathBuf,
        cache: Arc<OnceCell<(PathBuf, Arc<TLDR>)>>,
        notify: Arc<Notify>,
    ) {
        let cache_dir = match Self::get_cache_dir(&project_path) {
            Ok(dir) => dir,
            Err(e) => {
                warn!("pre_warm: failed to get cache dir: {}", e);
                notify.notify_waiters();
                return;
            }
        };

        let guard = WarmGuard::new(&cache_dir);
        if guard.is_fresh(&project_path) {
            info!("pre_warm: skipping, warm is fresh");
            notify.notify_waiters();
            return;
        }

        info!("pre_warm: warming TLDR for {:?}", project_path);
        let config = TldrConfig {
            language: Language::Auto,
            max_depth: 3,
            cache_dir: Some(cache_dir),
        };

        match TLDR::new(&project_path, config).await {
            Ok(mut tldr) => match tldr.warm().await {
                Ok(()) => {
                    guard.save(&project_path);
                    info!("pre_warm: warm complete");
                    let _ = cache.set((project_path, Arc::new(tldr))).map_err(|_| {
                        debug!("pre_warm: OnceCell already set (race with get_tldr)");
                    });
                    notify.notify_waiters();
                }
                Err(e) => warn!("pre_warm: warm failed: {}", e),
            },
            Err(e) => warn!("pre_warm: TLDR::new failed: {}", e),
        }
        notify.notify_waiters();
    }

    /// Get or create TLDR instance for a project (thread-safe, initializes once).
    ///
    /// If pre_warm is still running, waits for it. Falls back to lazy creation
    /// if pre_warm fails or hasn't started.
    async fn get_tldr(&self, project_path: &Path) -> Result<Arc<TLDR>, AgentError> {
        let project_path = project_path.to_path_buf();
        let project_path_for_check = project_path.clone();

        // Kick off pre_warm on first call inside the tokio runtime
        self.ensure_pre_warm_started();

        // If cache already populated, return immediately
        if let Some((cached_path, tldr)) = self.cache.get() {
            if *cached_path == project_path_for_check {
                debug!("TLDR cache hit for project: {:?}", project_path_for_check);
                return Ok(Arc::clone(tldr));
            }
            return Err(AgentError::ToolError(format!(
                "Project path mismatch: cached {:?} != requested {:?}",
                cached_path, project_path_for_check
            )));
        }

        // Wait for background warm to finish (with timeout fallback)
        info!("get_tldr: waiting for pre_warm...");
        tokio::select! {
            _ = self.warm_notify.notified() => {
                // pre_warm finished — check if it succeeded
                if let Some((cached_path, tldr)) = self.cache.get() {
                    if *cached_path == project_path_for_check {
                        debug!("TLDR cache hit after pre_warm for: {:?}", project_path_for_check);
                        return Ok(Arc::clone(tldr));
                    }
                }
                // pre_warm failed or was skipped (fresh) — fall through to lazy
                warn!("get_tldr: pre_warm did not populate cache, falling back to lazy");
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                warn!("get_tldr: pre_warm timed out after 30s, falling back to lazy");
            }
        }

        // Lazy fallback
        let cache = Arc::clone(&self.cache);
        let result: Result<&(PathBuf, Arc<TLDR>), AgentError> = cache
            .get_or_try_init(|| async {
                info!(
                    "Lazy creating TLDR instance for project: {:?}",
                    project_path
                );
                let config = TldrConfig {
                    language: Language::Auto,
                    max_depth: 3,
                    cache_dir: Some(Self::get_cache_dir(&project_path)?),
                };

                let mut tldr = TLDR::new(&project_path, config)
                    .await
                    .map_err(|e| AgentError::ToolError(format!("Failed to create TLDR: {}", e)))?;

                info!("Warming TLDR indexes...");
                tldr.warm()
                    .await
                    .map_err(|e| AgentError::ToolError(format!("Failed to warm TLDR: {}", e)))?;

                Ok((project_path, Arc::new(tldr)))
            })
            .await;

        let (_cached_path, tldr) = result?;
        debug!(
            "TLDR cache hit (lazy) for project: {:?}",
            project_path_for_check
        );
        Ok(Arc::clone(tldr))
    }

    /// Get cache directory for a project (~/.limit/projects/<project-hash>/tldr)
    fn get_cache_dir(project_path: &Path) -> Result<PathBuf, AgentError> {
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

        Ok(home
            .join(".limit")
            .join("projects")
            .join(&hash)
            .join("tldr"))
    }

    /// Build a source result JSON, reading the file and extracting lines
    async fn build_source_result(
        &self,
        function: &str,
        source_file: PathBuf,
        start_line: usize,
        end_line: usize,
        project_path: &Path,
    ) -> Result<Value, AgentError> {
        let relative_file = source_file
            .strip_prefix(project_path)
            .unwrap_or(&source_file)
            .to_path_buf();

        let file_path = project_path.join(&source_file);
        let source = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| AgentError::ToolError(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = source.lines().collect();
        let start = start_line.saturating_sub(1);
        let end = end_line.min(lines.len());
        let max_lines = 80;
        let truncated = (end - start) > max_lines;
        let actual_end = if truncated { start + max_lines } else { end };

        let function_source = lines[start..actual_end].join("\n");

        let mut result = json!({
            "type": "source",
            "function": function,
            "file": relative_file.display().to_string(),
            "line": start_line,
            "end_line": actual_end,
            "source": function_source
        });
        if truncated {
            result["truncated"] = json!(true);
            result["total_lines"] = json!(end - start);
        }

        Ok(result)
    }

    /// Perform analysis based on parameters
    async fn analyze(&self, params: TldrParams) -> Result<Value, AgentError> {
        let project_path = params
            .project_path
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_project.clone());

        let tldr = self.get_tldr(&project_path).await?;

        let result = match params.analysis_type {
            AnalysisType::Context => {
                let function = params.function.ok_or_else(|| {
                    AgentError::ToolError("function parameter required for context analysis".into())
                })?;

                let context = tldr
                    .get_context(&function, params.depth)
                    .await
                    .map_err(|e| {
                        AgentError::ToolError(format!("Context analysis failed: {}", e))
                    })?;

                Ok(json!({
                    "type": "context",
                    "function": function,
                    "depth": params.depth,
                    "context": context
                }))
            }

            AnalysisType::Source => {
                let function = params.function.ok_or_else(|| {
                    AgentError::ToolError("function parameter required for source analysis".into())
                })?;

                // Handle qualified method names: "StructName::method" (Rust/JS)
                // Resolve the struct to its file, then search for the method name alone
                let (function, file_override) = if !function.starts_with("struct ") {
                    if let Some(pos) = function.find("::") {
                        let class_name = &function[..pos];
                        let method_name = &function[pos + 2..];
                        if !method_name.is_empty() {
                            let class_info = if let Some(ref file) = params.file {
                                let file_path = project_path.join(file);
                                tldr.find_class_in(class_name, &file_path).unwrap_or(None)
                            } else {
                                tldr.find_class(class_name).unwrap_or(None)
                            };
                            if let Some(info) = class_info {
                                let resolved_file = info
                                    .file
                                    .strip_prefix(&project_path)
                                    .unwrap_or(&info.file)
                                    .to_string_lossy()
                                    .to_string();
                                (method_name.to_string(), Some(resolved_file))
                            } else {
                                (function, None)
                            }
                        } else {
                            (function, None)
                        }
                    } else {
                        (function, None)
                    }
                } else {
                    (function, None)
                };
                // Use resolved file if available, otherwise keep original
                let effective_file = file_override.or(params.file.clone());

                let is_struct = function.starts_with("struct ");
                let lookup_name = if is_struct {
                    function.strip_prefix("struct ").unwrap()
                } else {
                    &function
                };

                let (source_file, start_line, end_line) = if is_struct {
                    // Struct/class lookup
                    let class_info = if let Some(ref file) = effective_file {
                        let file_path = project_path.join(file);
                        tldr.find_class_in(lookup_name, &file_path)
                            .map_err(|e| {
                                AgentError::ToolError(format!("Source analysis failed: {}", e))
                            })?
                            .ok_or_else(|| {
                                AgentError::ToolError(format!(
                                    "Struct '{}' not found in '{}'",
                                    lookup_name, file
                                ))
                            })?
                    } else {
                        tldr.find_class(lookup_name)
                            .map_err(|e| {
                                AgentError::ToolError(format!("Source analysis failed: {}", e))
                            })?
                            .ok_or_else(|| {
                                AgentError::ToolError(format!("Struct not found: {}", lookup_name))
                            })?
                    };
                    (class_info.file, class_info.line, class_info.end_line)
                } else {
                    // Function lookup — also try struct/class fallback
                    let func_info = if let Some(ref file) = effective_file {
                        let file_path = project_path.join(file);
                        // Try function first, then struct fallback
                        if let Some(func) =
                            tldr.find_function_in(&function, &file_path).map_err(|e| {
                                AgentError::ToolError(format!("Source analysis failed: {}", e))
                            })?
                        {
                            func
                        } else if let Some(cls) =
                            tldr.find_class_in(&function, &file_path).map_err(|e| {
                                AgentError::ToolError(format!("Source analysis failed: {}", e))
                            })?
                        {
                            // Found as struct — treat as struct lookup
                            return self
                                .build_source_result(
                                    &function,
                                    cls.file,
                                    cls.line,
                                    cls.end_line,
                                    &project_path,
                                )
                                .await;
                        } else {
                            return Err(AgentError::ToolError(format!(
                                "Function or struct '{}' not found in '{}'",
                                function, file
                            )));
                        }
                    } else {
                        // Try find_all to detect ambiguity
                        let all_matches = tldr.find_all_functions(&function);
                        if all_matches.len() > 1 {
                            let match_list: Vec<String> = all_matches
                                .iter()
                                .take(5)
                                .map(|f| {
                                    let relative =
                                        f.file.strip_prefix(&project_path).unwrap_or(&f.file);
                                    format!("{} ({}:{})", f.name, relative.display(), f.line)
                                })
                                .collect();
                            return Ok(json!({
                                "type": "disambiguation_needed",
                                "function": function,
                                "match_count": all_matches.len(),
                                "matches": match_list,
                                "hint": format!(
                                    "Use file parameter to disambiguate, e.g.: {{\"analysis_type\": \"source\", \"function\": \"{}\", \"file\": \"path/to/file.rs\"}}",
                                    function
                                )
                            }));
                        }
                        tldr.find_function(&function)
                            .await
                            .map_err(|e| {
                                AgentError::ToolError(format!("Source analysis failed: {}", e))
                            })?
                            .ok_or_else(|| {
                                AgentError::ToolError(format!("Function not found: {}", function))
                            })?
                    };
                    (func_info.file, func_info.line, func_info.end_line)
                };

                self.build_source_result(
                    &function,
                    source_file,
                    start_line,
                    end_line,
                    &project_path,
                )
                .await
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
                let dead = tldr.find_dead_code(&entries).map_err(|e| {
                    AgentError::ToolError(format!("Dead code analysis failed: {}", e))
                })?;

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
                let arch = tldr.detect_architecture().map_err(|e| {
                    AgentError::ToolError(format!("Architecture detection failed: {}", e))
                })?;

                // Return counts + samples to keep output small
                // Full lists can be huge (20k+ chars) defeating token efficiency goal
                let entry_sample: Vec<_> = arch.entry.iter().take(10).collect();
                let middle_sample: Vec<_> = arch.middle.iter().take(10).collect();
                let leaf_sample: Vec<_> = arch.leaf.iter().take(10).collect();

                Ok(json!({
                    "type": "architecture",
                    "summary": {
                        "entry_points_count": arch.entry.len(),
                        "middle_layer_count": arch.middle.len(),
                        "leaf_functions_count": arch.leaf.len()
                    },
                    "sample_entry_points": entry_sample,
                    "sample_middle_layer": middle_sample,
                    "sample_leaf_functions": leaf_sample,
                    "note": "Showing top 10 of each category. Use Search analysis for specific functions."
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
                    "results": results.iter().map(|r| {
                        let relative = r
                            .file
                            .strip_prefix(&project_path)
                            .unwrap_or(&r.file);
                        json!({
                            "function": r.function,
                            "file": relative.display().to_string(),
                            "line": r.line,
                            "score": r.score,
                            "signature": r.signature
                        })
                    }).collect::<Vec<_>>()
                }))
            }
        };

        debug!("Analysis complete for: {:?}", params.analysis_type);
        result
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

        info!("tldr_analyze invoked: type={:?}", params.analysis_type);
        if let Some(ref f) = &params.function {
            debug!("  function: {}", f);
        }
        if let Some(ref q) = &params.query {
            debug!("  query: {}", q);
        }

        let result = match self.analyze(params).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("tldr_analyze failed: {}", e);
                return Err(e);
            }
        };
        let result_str =
            serde_json::to_string(&result).unwrap_or_else(|_| "serialize error".to_string());
        info!(
            "tldr_analyze result: {} chars, {} bytes",
            result_str.chars().count(),
            result_str.len()
        );
        Ok(result)
    }
}

/// Generate tool definition for LLM providers
pub fn tldr_tool_definition() -> Value {
    json!({
        "name": "tldr_analyze",
        "description": "Token-efficient code analysis. ALWAYS USE THIS when the user asks: 'what does X do', 'how does X work', 'explain X', 'tell me about X', 'what is X'. Saves 95% tokens vs reading raw code. Do NOT combine with file_read or bash — this tool provides all needed context. STRATEGY: (1) search to find functions, (2) source for 1-3 key functions only, (3) write answer. Do NOT read every function. Analysis types: search=find functions, context=dependencies, source=function code, impact=callers, architecture=layers.",
        "parameters": {
            "type": "object",
            "properties": {
                "analysis_type": {
                    "type": "string",
                    "enum": ["search", "context", "source", "impact", "cfg", "dfg", "dead_code", "architecture"],
                    "description": "Type: search=find by keyword, context=dependencies+callers, source=function code (use instead of file_read), impact=who calls this, cfg=control flow, dfg=data flow, dead_code=unreachable, architecture=module layers"
                },
                "function": {
                    "type": "string",
                    "description": "Function or struct name (required for context, source, impact, cfg, dfg). For structs, prefix with 'struct ' (e.g., 'struct AppConfig')"
                },
                "file": {
                    "type": "string",
                    "description": "File path relative to project root. Required for cfg, dfg. Optional for source (use to disambiguate when function name exists in multiple files)"
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
                    "description": "Search query for finding functions (supports patterns like 'daemon', 'auth', 'handle_*')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results for search (default: 10)",
                    "default": 10
                },
                "project_path": {
                    "type": "string",
                    "description": "Project root directory (defaults to current directory). Do NOT use file paths here — use 'file' parameter for file paths."
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

    #[tokio::test]
    async fn test_cache_returns_cached_instance() {
        let tool = TldrTool::new();
        let test_path = std::env::current_dir().unwrap();

        let tldr1 = tool.get_tldr(&test_path).await.unwrap();
        let tldr2 = tool.get_tldr(&test_path).await.unwrap();

        let addr1 = Arc::as_ptr(&tldr1) as usize;
        let addr2 = Arc::as_ptr(&tldr2) as usize;
        assert_eq!(
            addr1, addr2,
            "Second call should return cached instance (same memory address)"
        );
    }
}
