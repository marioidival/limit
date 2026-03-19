//! Layer 2: Call Graph - "Who calls what?"
//!
//! Tracks forward and backward dependencies between functions.

use std::collections::{HashMap, HashSet};

use crate::cache::CacheManager;
use crate::error::Result;
use crate::layers::ast::ASTLayer;
use crate::types::{CallerInfo, FunctionInfo, ArchitectureInfo};

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
    
    /// Build call graph from AST
    pub async fn warm(&mut self, ast: &ASTLayer, cache: &mut CacheManager) -> Result<()> {
        // Index all function locations
        for func in ast.all_functions() {
            self.function_locations.insert(
                func.name.clone(),
                (func.file.clone(), func.line),
            );
        }
        
        // Build call graph (simplified - in practice would use tree-sitter queries)
        // For each function, find all calls to other indexed functions
        
        // Store in cache
        cache.store_call_graph(&self.forward_calls, &self.backward_calls)?;
        
        Ok(())
    }
    
    /// Get forward calls (what does this function call?)
    pub fn get_forward_calls(&self, function: &str) -> Result<Vec<String>> {
        Ok(self.forward_calls.get(function).cloned().unwrap_or_default())
    }
    
    /// Get backward calls (who calls this function?)
    pub fn get_backward_calls(&self, function: &str) -> Result<Vec<CallerInfo>> {
        Ok(self.backward_calls.get(function).cloned().unwrap_or_default())
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
        let unreachable: Vec<FunctionInfo> = self.function_locations.iter()
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
            let has_callees = self.forward_calls.get(func).map(|c| !c.is_empty()).unwrap_or(false);
            
            if !has_callers {
                entry.push(func.clone());
            } else if !has_callees {
                leaf.push(func.clone());
            } else {
                middle.push(func.clone());
            }
        }
        
        Ok(ArchitectureInfo { entry, middle, leaf })
    }
}

impl Default for CallGraphLayer {
    fn default() -> Self {
        Self::new()
    }
}
