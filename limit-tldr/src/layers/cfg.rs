//! Layer 3: CFG (Control Flow Graph) - "How complex is this?"
//!
//! Analyzes branching logic and computes cyclomatic complexity using tree-sitter.

use crate::error::{Error, Result};
use crate::layers::cfg_builder::{build_cfg, count_branches};
use crate::parsers::tree_sitter::TreeSitterParser;
use crate::types::{BasicBlock, CFGInfo, FunctionInfo, Language};

/// Control flow graph layer
pub struct CFGLayer {}

impl CFGLayer {
    pub fn new() -> Self {
        Self {}
    }

    /// Analyze a function and compute its CFG with real cyclomatic complexity
    pub fn analyze(&self, func: &FunctionInfo) -> Result<CFGInfo> {
        let file_path = &func.file;
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| Error::ParseError {
                file: file_path.display().to_string(),
                message: "No file extension".to_string(),
            })?;

        let language = Language::from_extension(ext)
            .ok_or_else(|| Error::LanguageNotSupported(ext.to_string()))?;

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| Error::PathNotFound(file_path.display().to_string(), e))?;

        let parser = TreeSitterParser::new();
        let ts_lang = parser
            .get_ts_language(language)
            .ok_or_else(|| Error::TreeSitter(format!("No tree-sitter support for {}", language)))?;

        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser
            .set_language(&ts_lang)
            .map_err(|e| Error::ParseError {
                file: file_path.display().to_string(),
                message: format!("Tree-sitter language error: {}", e),
            })?;

        let tree = ts_parser
            .parse(&source, None)
            .ok_or_else(|| Error::ParseError {
                file: file_path.display().to_string(),
                message: "Failed to parse source".to_string(),
            })?;

        let root = tree.root_node();
        let func_node = TreeSitterParser::find_function_node(root, &func.name, &source)
            .ok_or_else(|| Error::FunctionNotFound(func.name.clone()))?;

        let branches = count_branches(func_node);
        let complexity = 1 + branches;

        let graph = build_cfg(func_node, &source);
        let blocks: Vec<BasicBlock> = graph
            .blocks
            .into_iter()
            .enumerate()
            .map(|(id, b)| BasicBlock {
                id,
                statements: b.statements,
                start_line: b.start_line,
                end_line: b.end_line,
            })
            .collect();
        let edges = graph.edges;

        Ok(CFGInfo {
            function: func.name.clone(),
            blocks,
            edges,
            complexity,
        })
    }
}

impl Default for CFGLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::ast::ASTLayer;
    use crate::types::Language;
    use tempfile::tempdir;

    #[tokio::test]
    async fn cfg_simple_function_is_one() {
        let dir = tempdir().unwrap();
        tokio::fs::write(dir.path().join("t.py"), "def foo():\n    return 42")
            .await
            .unwrap();
        let ast = ASTLayer::new(Language::Python);
        let func = &ast
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        assert_eq!(CFGLayer::new().analyze(func).unwrap().complexity, 1);
    }

    #[tokio::test]
    async fn cfg_if_else_is_two() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(x):
    if x > 0:
        return 1
    else:
        return -1
"#,
        )
        .await
        .unwrap();
        let ast = ASTLayer::new(Language::Python);
        let func = &ast
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        assert_eq!(CFGLayer::new().analyze(func).unwrap().complexity, 2);
    }

    #[tokio::test]
    async fn cfg_if_elif_else_is_three() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(x):
    if x > 0:
        return 1
    elif x == 0:
        return 0
    else:
        return -1
"#,
        )
        .await
        .unwrap();
        let ast = ASTLayer::new(Language::Python);
        let func = &ast
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        assert_eq!(CFGLayer::new().analyze(func).unwrap().complexity, 3);
    }

    #[tokio::test]
    async fn cfg_nested_if_is_three() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(x):
    if x > 0:
        if x > 10:
            return 1
        else:
            return 2
    else:
        return -1
"#,
        )
        .await
        .unwrap();
        let ast = ASTLayer::new(Language::Python);
        let func = &ast
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        assert_eq!(CFGLayer::new().analyze(func).unwrap().complexity, 3);
    }

    #[tokio::test]
    async fn cfg_for_loop_is_two() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(items):
    for item in items:
        print(item)
    return items
"#,
        )
        .await
        .unwrap();
        let ast = ASTLayer::new(Language::Python);
        let func = &ast
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        assert_eq!(CFGLayer::new().analyze(func).unwrap().complexity, 2);
    }

    #[tokio::test]
    async fn cfg_rust_match() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.rs"),
            r#"
fn foo(x: i32) -> i32 {
    match x {
        1 => 10,
        2 => 20,
        _ => 0,
    }
}
"#,
        )
        .await
        .unwrap();
        let ast = ASTLayer::new(Language::Rust);
        let func = &ast
            .analyze_file(&dir.path().join("t.rs"))
            .await
            .unwrap()
            .functions[0];

        let cfg = CFGLayer::new().analyze(func).unwrap();
        assert!(
            cfg.complexity >= 3,
            "Match with 3 arms should have complexity >= 3, got {}",
            cfg.complexity
        );
    }

    #[tokio::test]
    async fn cfg_multiple_blocks_for_if_else() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(x):
    y = 1
    if x > 0:
        y = 2
    else:
        y = 3
    return y
"#,
        )
        .await
        .unwrap();
        let ast = ASTLayer::new(Language::Python);
        let func = &ast
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        let cfg = CFGLayer::new().analyze(func).unwrap();
        assert!(
            cfg.blocks.len() >= 2,
            "Should have at least 2 blocks for if/else"
        );
        assert!(!cfg.edges.is_empty(), "Should have edges between blocks");
    }
}
