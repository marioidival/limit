//! CLI for TLDR

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::types::Language;

mod runner;

pub use runner::run;

#[derive(Parser)]
#[command(name = "tldr")]
#[command(about = "Code analysis that actually fits in context")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Project path (defaults to current directory)
    #[arg(short, long, global = true)]
    pub project: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build/update all indexes
    Warm {
        /// Project path
        path: Option<PathBuf>,
    },
    
    /// Start daemon
    Daemon {
        #[command(subcommand)]
        command: DaemonCommands,
    },
    
    /// Get LLM-ready context for a function
    Context {
        /// Entry point function
        entry: String,
        
        /// Depth of call graph traversal
        #[arg(short, long, default_value = "2")]
        depth: usize,
    },
    
    /// Find who calls a function
    Impact {
        /// Function name
        function: String,
        
        /// Project path
        path: Option<PathBuf>,
    },
    
    /// Get control flow graph
    Cfg {
        /// File path
        file: PathBuf,
        
        /// Function name
        function: String,
    },
    
    /// Get data flow graph
    Dfg {
        /// File path
        file: PathBuf,
        
        /// Function name
        function: String,
    },
    
    /// Get program slice for a line
    Slice {
        /// File path
        file: PathBuf,
        
        /// Function name
        function: String,
        
        /// Target line number
        line: usize,
    },
    
    /// Extract file structure
    Extract {
        /// File path
        file: PathBuf,
    },
    
    /// Find dead code
    Dead {
        /// Entry points (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        entry: Vec<String>,
        
        /// Project path
        path: Option<PathBuf>,
    },
    
    /// Detect architecture layers
    Arch {
        /// Project path
        path: Option<PathBuf>,
    },
    
    /// Semantic search for code
    Semantic {
        /// Search query
        query: String,
        
        /// Project path
        path: Option<PathBuf>,
        
        /// Number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    
    /// Text search in code
    Search {
        /// Search pattern
        pattern: String,
        
        /// Project path
        path: Option<PathBuf>,
    },
    
    /// Get file tree
    Tree {
        /// Project path
        path: Option<PathBuf>,
    },
    
    /// Get code structure overview
    Structure {
        /// Project path
        path: Option<PathBuf>,
        
        /// Language
        #[arg(short, long)]
        lang: Option<String>,
    },
    
    /// Parse imports from a file
    Imports {
        /// File path
        file: PathBuf,
    },
    
    /// Find all files that import a module
    Importers {
        /// Module name
        module: String,
        
        /// Project path
        path: Option<PathBuf>,
    },
    
    /// Type check + lint
    Diagnostics {
        /// File or directory path
        path: PathBuf,
        
        /// Output format
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start daemon
    Start,
    
    /// Stop daemon
    Stop,
    
    /// Check daemon status
    Status,
}
