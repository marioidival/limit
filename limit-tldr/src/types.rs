//! Core types and data structures for TLDR

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Auto,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Rust,
    Java,
    C,
    Cpp,
    Ruby,
    PHP,
    CSharp,
    Kotlin,
    Scala,
    Swift,
    Lua,
    Elixir,
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Language::Auto),
            "python" | "py" => Ok(Language::Python),
            "typescript" | "ts" => Ok(Language::TypeScript),
            "javascript" | "js" => Ok(Language::JavaScript),
            "go" | "golang" => Ok(Language::Go),
            "rust" | "rs" => Ok(Language::Rust),
            "java" => Ok(Language::Java),
            "c" => Ok(Language::C),
            "cpp" | "c++" => Ok(Language::Cpp),
            "ruby" | "rb" => Ok(Language::Ruby),
            "php" => Ok(Language::PHP),
            "csharp" | "cs" | "c#" => Ok(Language::CSharp),
            "kotlin" | "kt" => Ok(Language::Kotlin),
            "scala" => Ok(Language::Scala),
            "swift" => Ok(Language::Swift),
            "lua" => Ok(Language::Lua),
            "elixir" | "ex" => Ok(Language::Elixir),
            _ => Err(format!("Unknown language: {}", s)),
        }
    }
}

impl Language {
    /// Get file extensions for this language
    pub fn extensions(&self) -> &[&str] {
        match self {
            Language::Python => &["py"],
            Language::TypeScript => &["ts", "tsx"],
            Language::JavaScript => &["js", "jsx", "mjs"],
            Language::Go => &["go"],
            Language::Rust => &["rs"],
            Language::Java => &["java"],
            Language::C => &["c", "h"],
            Language::Cpp => &["cpp", "cc", "cxx", "hpp", "hxx"],
            Language::Ruby => &["rb"],
            Language::PHP => &["php"],
            Language::CSharp => &["cs"],
            Language::Kotlin => &["kt", "kts"],
            Language::Scala => &["scala"],
            Language::Swift => &["swift"],
            Language::Lua => &["lua"],
            Language::Elixir => &["ex", "exs"],
            Language::Auto => &[],
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "py" => Some(Language::Python),
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" | "mjs" => Some(Language::JavaScript),
            "go" => Some(Language::Go),
            "rs" => Some(Language::Rust),
            "java" => Some(Language::Java),
            "c" | "h" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
            "rb" => Some(Language::Ruby),
            "php" => Some(Language::PHP),
            "cs" => Some(Language::CSharp),
            "kt" | "kts" => Some(Language::Kotlin),
            "scala" => Some(Language::Scala),
            "swift" => Some(Language::Swift),
            "lua" => Some(Language::Lua),
            "ex" | "exs" => Some(Language::Elixir),
            _ => None,
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Auto => write!(f, "auto"),
            Language::Python => write!(f, "python"),
            Language::TypeScript => write!(f, "typescript"),
            Language::JavaScript => write!(f, "javascript"),
            Language::Go => write!(f, "go"),
            Language::Rust => write!(f, "rust"),
            Language::Java => write!(f, "java"),
            Language::C => write!(f, "c"),
            Language::Cpp => write!(f, "cpp"),
            Language::Ruby => write!(f, "ruby"),
            Language::PHP => write!(f, "php"),
            Language::CSharp => write!(f, "csharp"),
            Language::Kotlin => write!(f, "kotlin"),
            Language::Scala => write!(f, "scala"),
            Language::Swift => write!(f, "swift"),
            Language::Lua => write!(f, "lua"),
            Language::Elixir => write!(f, "elixir"),
        }
    }
}

/// Information about a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,
    /// Full signature
    pub signature: String,
    /// Parameters with types
    pub params: Vec<Parameter>,
    /// Return type (if known)
    pub return_type: Option<String>,
    /// Whether the function is async
    pub is_async: bool,
    /// Line number where function starts
    pub line: usize,
    /// Line number where function ends
    pub end_line: usize,
    /// File path
    pub file: PathBuf,
    /// Docstring/comment
    pub docstring: Option<String>,
    /// Complexity score
    pub complexity: Option<u32>,
}

/// Function parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
}

/// Information about a class/struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Methods in the class
    pub methods: Vec<String>,
    /// Fields/properties
    pub fields: Vec<FieldInfo>,
    /// Line number
    pub line: usize,
    /// End line number
    pub end_line: usize,
    /// File path
    pub file: PathBuf,
    /// Docstring
    pub docstring: Option<String>,
}

/// Field/property information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_annotation: Option<String>,
}

/// Constant/variable information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantInfo {
    /// Constant name
    pub name: String,
    /// Type annotation (if known)
    pub type_annotation: Option<String>,
    /// Constant value (truncated to 200 chars)
    pub value: Option<String>,
    /// Line number where constant starts
    pub line: usize,
    /// Line number where constant ends
    pub end_line: usize,
    /// File path
    pub file: PathBuf,
    /// Docstring/comment
    pub docstring: Option<String>,
    /// Whether the constant is mutable (e.g., static mut in Rust, let in JS)
    pub is_mutable: bool,
}

/// Import statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    /// Module being imported
    pub module: String,
    /// Names imported (empty for `import module`)
    pub names: Vec<String>,
    /// Alias (for `import module as alias`)
    pub alias: Option<String>,
    /// Line number
    pub line: usize,
}

/// Call expression extracted during parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallExpression {
    /// Function containing the call
    pub caller: String,
    /// Function being called
    pub callee: String,
    /// Line number of the call
    pub line: usize,
    /// File path
    pub file: PathBuf,
}

/// Caller information for impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerInfo {
    /// Calling function name
    pub function: String,
    /// File containing the call
    pub file: PathBuf,
    /// Line number of the call
    pub line: usize,
}

/// Control Flow Graph information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CFGInfo {
    /// Function name
    pub function: String,
    /// Basic blocks
    pub blocks: Vec<BasicBlock>,
    /// Control flow edges
    pub edges: Vec<(usize, usize)>,
    /// Cyclomatic complexity
    pub complexity: u32,
}

/// Basic block in CFG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    /// Block ID
    pub id: usize,
    /// Statements in this block
    pub statements: Vec<String>,
    /// Start line
    pub start_line: usize,
    /// End line
    pub end_line: usize,
}

/// Data Flow Graph information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DFGInfo {
    /// Function name
    pub function: String,
    /// Variable definitions and uses
    pub variables: Vec<VariableFlow>,
    /// Data flow edges
    pub flows: Vec<DataFlow>,
}

/// Variable flow information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableFlow {
    /// Variable name
    pub name: String,
    /// Lines where defined
    pub defined_at: Vec<usize>,
    /// Lines where used
    pub used_at: Vec<usize>,
}

/// Data flow edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlow {
    /// Source variable
    pub from: String,
    /// Target variable
    pub to: String,
    /// Function that transforms (if any)
    pub via: Option<String>,
}

/// Program slice information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceInfo {
    /// Target line
    pub target_line: usize,
    /// Lines in the slice
    pub slice: Vec<usize>,
    /// Code at those lines
    pub slice_code: Vec<String>,
}

/// Architecture layer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureInfo {
    /// Entry points (not called by anything)
    pub entry: Vec<String>,
    /// Middle layer (both calls and called)
    pub middle: Vec<String>,
    /// Leaf functions (don't call anything)
    pub leaf: Vec<String>,
}

/// Semantic search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Function name
    pub function: String,
    /// File path
    pub file: PathBuf,
    /// Line number
    pub line: usize,
    /// Similarity score (0-1)
    pub score: f32,
    /// Function signature
    pub signature: String,
}

/// File analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    /// File path
    pub file: PathBuf,
    /// Functions found
    pub functions: Vec<FunctionInfo>,
    /// Classes found
    pub classes: Vec<ClassInfo>,
    /// Imports
    pub imports: Vec<ImportInfo>,
    /// Call expressions found
    #[serde(default)]
    pub call_expressions: Vec<CallExpression>,
    /// Constants found
    #[serde(default)]
    pub constants: Vec<ConstantInfo>,
    /// Language detected
    pub language: Language,
}
