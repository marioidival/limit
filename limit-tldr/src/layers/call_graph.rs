//! Layer 2: Call Graph - "Who calls what?"
//!
//! Tracks forward and backward dependencies between functions.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::cache::CacheManager;
use crate::error::{Error, Result};
use crate::layers::ast::ASTLayer;
use crate::parsers::tree_sitter::TreeSitterParser;
use crate::types::{ArchitectureInfo, CallerInfo, FunctionInfo, Language};

/// Call graph layer
pub struct CallGraphLayer {
    /// Forward calls: function -> functions it calls
    forward_calls: HashMap<String, Vec<String>>,
    /// Backward calls: function -> functions that call it
    backward_calls: HashMap<String, Vec<CallerInfo>>,
    /// Function locations
    function_locations: HashMap<String, (std::path::PathBuf, usize)>,
}

impl CallGraphLayer {
    pub fn new() -> Self {
        Self {
            forward_calls: HashMap::new(),
            backward_calls: HashMap::new(),
            function_locations: HashMap::new(),
        }
    }

    fn extract_calls_from_file(
        &self,
        file: &Path,
        source: &str,
        language: Language,
    ) -> Result<Vec<(String, String, usize)>> {
        let mut calls = Vec::new();

        let parser = TreeSitterParser::new();

        if let Some(ts_lang) = parser.get_ts_language(language) {
            let mut ts_parser = tree_sitter::Parser::new();
            ts_parser
                .set_language(&ts_lang)
                .map_err(|e| Error::ParseError {
                    file: file.display().to_string(),
                    message: format!("Tree-sitter language error: {}", e),
                })?;

            let tree = ts_parser
                .parse(source, None)
                .ok_or_else(|| Error::ParseError {
                    file: file.display().to_string(),
                    message: "Failed to parse source".to_string(),
                })?;

            let root = tree.root_node();
            self.collect_calls(root, source, &mut calls);
        }

        Ok(calls)
    }

    fn collect_calls(
        &self,
        node: tree_sitter::Node,
        source: &str,
        calls: &mut Vec<(String, String, usize)>,
    ) {
        let cursor = &mut node.walk();

        for child in node.children(cursor) {
            match child.kind() {
                // Call expressions vary by language
                "call" | "call_expression" | "function_call" | "member_call" => {
                    if let Some((caller, callee, line)) = self.extract_call_info(child, source) {
                        calls.push((caller, callee, line));
                    }
                }
                _ => {}
            }

            self.collect_calls(child, source, calls);
        }
    }

    fn extract_call_info(
        &self,
        node: tree_sitter::Node,
        source: &str,
    ) -> Option<(String, String, usize)> {
        let function_node = node
            .child_by_field_name("function")
            .or_else(|| node.child(0))?;

        let callee = self.node_text(function_node, source);

        let callee = callee.split('.').next_back().unwrap_or(&callee).to_string();
        let callee = callee.trim().to_string();

        if callee.is_empty() {
            return None;
        }

        let caller = self.find_enclosing_function(node, source)?;
        let line = node.start_position().row + 1;

        Some((caller, callee, line))
    }

    fn find_enclosing_function(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        let mut current = node.parent();

        while let Some(parent) = current {
            match parent.kind() {
                "function_definition"
                | "function_item"
                | "method_definition"
                | "arrow_function"
                | "function_declaration" => {
                    let name_node = parent
                        .child_by_field_name("name")
                        .or_else(|| parent.child(0));

                    if let Some(name_node) = name_node {
                        return Some(self.node_text(name_node, source));
                    }
                }
                _ => {}
            }
            current = parent.parent();
        }

        None
    }

    fn node_text(&self, node: tree_sitter::Node, source: &str) -> String {
        let start = node.start_byte();
        let end = node.end_byte();
        source[start..end].to_string()
    }

    pub async fn warm(&mut self, ast: &ASTLayer, cache: &mut CacheManager) -> Result<()> {
        for func in ast.all_functions() {
            self.function_locations
                .insert(func.name.clone(), (func.file.clone(), func.line));
        }

        let mut files_to_analyze: std::collections::HashSet<PathBuf> =
            std::collections::HashSet::new();
        for func in ast.all_functions() {
            files_to_analyze.insert(func.file.clone());
        }

        for file in files_to_analyze {
            let source = tokio::fs::read_to_string(&file)
                .await
                .map_err(|e: std::io::Error| Error::PathNotFound(file.display().to_string(), e))?;

            let lang = file
                .extension()
                .and_then(|e: &std::ffi::OsStr| e.to_str())
                .and_then(crate::types::Language::from_extension)
                .unwrap_or(Language::Auto);

            let calls = self.extract_calls_from_file(&file, &source, lang)?;

            for (caller, callee, line) in calls {
                if self.function_locations.contains_key(&callee) {
                    self.forward_calls
                        .entry(caller.clone())
                        .or_default()
                        .push(callee.clone());

                    let caller_info = CallerInfo {
                        function: caller.clone(),
                        file: file.clone(),
                        line,
                    };
                    self.backward_calls
                        .entry(callee)
                        .or_default()
                        .push(caller_info);
                }
            }
        }

        cache.store_call_graph(&self.forward_calls, &self.backward_calls)?;

        Ok(())
    }

    /// Get forward calls (what does this function call?)
    pub fn get_forward_calls(&self, function: &str) -> Result<Vec<String>> {
        Ok(self
            .forward_calls
            .get(function)
            .cloned()
            .unwrap_or_default())
    }

    /// Get backward calls (who calls this function?)
    pub fn get_backward_calls(&self, function: &str) -> Result<Vec<CallerInfo>> {
        Ok(self
            .backward_calls
            .get(function)
            .cloned()
            .unwrap_or_default())
    }

    /// Find unreachable functions (dead code)
    pub fn find_unreachable(&self, entries: &[&str]) -> Result<Vec<FunctionInfo>> {
        let mut reachable = HashSet::new();
        let mut queue: Vec<&str> = entries.to_vec();

        while let Some(func) = queue.pop() {
            if reachable.contains(func) {
                continue;
            }
            reachable.insert(func);

            if let Some(calls) = self.forward_calls.get(func) {
                for call in calls {
                    if !reachable.contains(call.as_str()) {
                        queue.push(call);
                    }
                }
            }
        }

        // Find unreachable
        let unreachable: Vec<FunctionInfo> = self
            .function_locations
            .iter()
            .filter(|(name, _)| !reachable.contains(name.as_str()))
            .map(|(name, (file, line))| FunctionInfo {
                name: name.clone(),
                signature: format!("{}()", name),
                params: Vec::new(),
                return_type: None,
                is_async: false,
                line: *line,
                end_line: *line,
                file: file.clone(),
                docstring: None,
                complexity: None,
            })
            .collect();

        Ok(unreachable)
    }

    /// Detect architecture layers (entry/middle/leaf)
    pub fn detect_layers(&self) -> Result<ArchitectureInfo> {
        let mut entry = Vec::new();
        let mut middle = Vec::new();
        let mut leaf = Vec::new();

        for func in self.function_locations.keys() {
            let has_callers = self.backward_calls.contains_key(func);
            let has_callees = self
                .forward_calls
                .get(func)
                .map(|c| !c.is_empty())
                .unwrap_or(false);

            if !has_callers {
                entry.push(func.clone());
            } else if !has_callees {
                leaf.push(func.clone());
            } else {
                middle.push(func.clone());
            }
        }

        Ok(ArchitectureInfo {
            entry,
            middle,
            leaf,
        })
    }
}

impl Default for CallGraphLayer {
    fn default() -> Self {
        Self::new()
    }
}
