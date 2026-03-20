//! AnalysisLayer trait - unified interface for building analysis layers

use crate::error::Result;
use crate::types::FileAnalysis;

/// Unified trait for analysis layers that build from pre-computed file analyses.
pub trait AnalysisLayer {
    /// Build the layer from pre-computed file analyses.
    fn build(&mut self, analyses: &[FileAnalysis]) -> Result<()>;
}
