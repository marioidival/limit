//! Layer 2: Call Graph - "Who calls what?"
//!
//! Tracks forward and backward dependencies between functions.

use std::collections::{HashMap, HashSet};

use crate::cache::CacheManager;
use crate::error::Result;
use crate::layers::ast::ASTLayer;
use crate::layers::trait_def::AnalysisLayer;
use crate::types::{ArchitectureInfo, CallerInfo, FileAnalysis, FunctionInfo};

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

    /// Build call graph from pre-computed file analyses
    pub fn build(&mut self, analyses: &[FileAnalysis]) {
        // Build function location index
        for analysis in analyses {
            for func in &analysis.functions {
                self.function_locations
                    .insert(func.name.clone(), (func.file.clone(), func.line));
            }
        }

        // Build call graph from call expressions
        for analysis in analyses {
            for call in &analysis.call_expressions {
                if self.function_locations.contains_key(&call.callee) {
                    self.forward_calls
                        .entry(call.caller.clone())
                        .or_default()
                        .push(call.callee.clone());

                    let caller_info = CallerInfo {
                        function: call.caller.clone(),
                        file: call.file.clone(),
                        line: call.line,
                    };
                    self.backward_calls
                        .entry(call.callee.clone())
                        .or_default()
                        .push(caller_info);
                }
            }
        }
    }

    pub async fn warm(&mut self, ast: &ASTLayer, _cache: &mut CacheManager) -> Result<()> {
        let analyses: Vec<FileAnalysis> = ast.file_analyses().into_iter().cloned().collect();
        self.build(&analyses);
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

impl AnalysisLayer for CallGraphLayer {
    fn build(&mut self, analyses: &[FileAnalysis]) -> Result<()> {
        CallGraphLayer::build(self, analyses);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CallExpression, FunctionInfo, Language};
    use std::path::PathBuf;

    fn make_analysis(
        file: &str,
        funcs: Vec<(&str, usize)>,
        calls: Vec<(&str, &str, usize)>,
    ) -> FileAnalysis {
        FileAnalysis {
            file: PathBuf::from(file),
            functions: funcs
                .into_iter()
                .map(|(name, line)| FunctionInfo {
                    name: name.to_string(),
                    signature: format!("fn {}()", name),
                    params: Vec::new(),
                    return_type: None,
                    is_async: false,
                    line,
                    end_line: line + 10,
                    file: PathBuf::from(file),
                    docstring: None,
                    complexity: None,
                })
                .collect(),
            classes: Vec::new(),
            imports: Vec::new(),
            call_expressions: calls
                .into_iter()
                .map(|(caller, callee, line)| CallExpression {
                    caller: caller.to_string(),
                    callee: callee.to_string(),
                    line,
                    file: PathBuf::from(file),
                })
                .collect(),
            language: Language::Rust,
        }
    }

    #[test]
    fn build_from_analyses() {
        let analyses = vec![
            make_analysis(
                "src/main.rs",
                vec![("main", 1), ("helper", 10)],
                vec![("main", "helper", 3)],
            ),
            make_analysis(
                "src/lib.rs",
                vec![("helper", 1), ("core", 5)],
                vec![("helper", "core", 2)],
            ),
        ];

        let mut cg = CallGraphLayer::new();
        cg.build(&analyses);

        let fwd = cg.get_forward_calls("main").unwrap();
        assert_eq!(fwd, vec!["helper"]);

        let bwd = cg.get_backward_calls("helper").unwrap();
        assert_eq!(bwd.len(), 1);
        assert_eq!(bwd[0].function, "main");

        let fwd2 = cg.get_forward_calls("helper").unwrap();
        assert_eq!(fwd2, vec!["core"]);
    }
}
