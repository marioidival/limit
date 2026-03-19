//! Error types for TLDR

use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Path not found: {0}")]
    PathNotFound(String, #[source] std::io::Error),
    
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    
    #[error("Parse error in {file}: {message}")]
    ParseError { file: String, message: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Language not supported: {0}")]
    LanguageNotSupported(String),
    
    #[error("Daemon error: {0}")]
    Daemon(String),
    
    #[error("Semantic search disabled")]
    SemanticDisabled,
    
    #[error("Semantic search error: {0}")]
    Semantic(String),
    
    #[error("Tree-sitter error: {0}")]
    TreeSitter(String),
    
    #[error("Invalid configuration: {0}")]
    Config(String),
}
