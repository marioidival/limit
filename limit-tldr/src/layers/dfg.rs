//! Layer 4: DFG (Data Flow Graph) - "Where does this value come from?"
//!
//! Tracks variable definitions and uses.

use crate::error::Result;
use crate::types::{DFGInfo, FunctionInfo, VariableFlow};

/// Data flow graph layer
pub struct DFGLayer {}

impl DFGLayer {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze a function and compute its DFG
    pub fn analyze(&self, func: &FunctionInfo) -> Result<DFGInfo> {
        // Simplified implementation
        // In practice, would use tree-sitter to track variable definitions and uses

        let variables = func
            .params
            .iter()
            .map(|p| VariableFlow {
                name: p.name.clone(),
                defined_at: vec![func.line],
                used_at: vec![],
            })
            .collect();

        Ok(DFGInfo {
            function: func.name.clone(),
            variables,
            flows: vec![],
        })
    }
}

impl Default for DFGLayer {
    fn default() -> Self {
        Self::new()
    }
}
