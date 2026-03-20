//! Layer 4: DFG (Data Flow Graph) - "Where does this value come from?"
//!
//! Tracks variable definitions and uses per-function. Data flows are built
//! during AST traversal (not via line-proximity heuristics) for accuracy.

use crate::error::{Error, Result};
use crate::parsers::tree_sitter::TreeSitterParser;
use crate::types::{DFGInfo, DataFlow, FunctionInfo, Language, VariableFlow};
use std::collections::{HashMap, HashSet};

pub struct DFGLayer {}

impl DFGLayer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn analyze(&self, func: &FunctionInfo) -> Result<DFGInfo> {
        let source = std::fs::read_to_string(&func.file)
            .map_err(|e| Error::PathNotFound(func.file.display().to_string(), e))?;

        let lang =
            Language::from_extension(func.file.extension().and_then(|e| e.to_str()).unwrap_or(""))
                .unwrap_or(Language::Auto);

        let parser = TreeSitterParser::new();
        let ts_lang = parser
            .get_ts_language(lang)
            .ok_or_else(|| Error::LanguageNotSupported(lang.to_string()))?;

        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser
            .set_language(&ts_lang)
            .map_err(|e| Error::TreeSitter(e.to_string()))?;
        let tree = ts_parser
            .parse(&source, None)
            .ok_or_else(|| Error::ParseError {
                file: func.file.display().to_string(),
                message: "Failed to parse".to_string(),
            })?;

        let func_node = TreeSitterParser::find_function_node(tree.root_node(), &func.name, &source)
            .ok_or_else(|| Error::FunctionNotFound(func.name.clone()))?;

        let mut var_map: HashMap<String, VariableFlow> = HashMap::new();
        let mut flows: Vec<DataFlow> = Vec::new();

        // Seed with parameters
        for p in &func.params {
            var_map
                .entry(p.name.clone())
                .or_insert_with(|| VariableFlow {
                    name: p.name.clone(),
                    defined_at: vec![func.line],
                    used_at: vec![],
                });
        }

        // Walk function body
        walk_for_assignments(func_node, &source, lang, &mut var_map, &mut flows);

        let variables: Vec<VariableFlow> = var_map.into_values().collect();

        Ok(DFGInfo {
            function: func.name.clone(),
            variables,
            flows,
        })
    }
}

/// Language-specific assignment node kinds
fn assignment_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Python => &["assignment", "augmented_assignment", "named_expression"],
        Language::Rust => &["let_declaration", "compound_assignment_expr"],
        _ => &["assignment", "expression_statement", "let_declaration"],
    }
}

/// LHS extraction by language
fn extract_lhs(node: tree_sitter::Node, source: &str, lang: Language) -> Option<(String, bool)> {
    match lang {
        Language::Rust => {
            let pattern = node
                .child_by_field_name("pattern")
                .or_else(|| node.child(0));
            pattern.map(|n| {
                (
                    text_for(n, source).trim().to_string(),
                    n.kind() == "identifier",
                )
            })
        }
        _ => {
            let lhs = node.child_by_field_name("left").or_else(|| node.child(0));
            lhs.map(|n| {
                (
                    text_for(n, source).trim().to_string(),
                    n.kind() == "identifier",
                )
            })
        }
    }
}

/// RHS extraction by language
fn extract_rhs<'a>(node: tree_sitter::Node<'a>, lang: Language) -> Option<tree_sitter::Node<'a>> {
    match lang {
        Language::Rust => node.child_by_field_name("value"),
        _ => node.child_by_field_name("right"),
    }
}

fn walk_for_assignments(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    var_map: &mut HashMap<String, VariableFlow>,
    flows: &mut Vec<DataFlow>,
) {
    let kinds = assignment_kinds(lang);
    let cursor = &mut node.walk();

    for child in node.children(cursor) {
        let kind = child.kind();

        if kinds.contains(&kind) {
            process_assignment(child, source, lang, var_map, flows);
        } else if kind == "return_statement" {
            track_uses_in_subtree(child, source, var_map);
        } else {
            walk_for_assignments(child, source, lang, var_map, flows);
        }
    }
}

fn process_assignment(
    node: tree_sitter::Node,
    source: &str,
    lang: Language,
    var_map: &mut HashMap<String, VariableFlow>,
    flows: &mut Vec<DataFlow>,
) {
    let line = node.start_position().row + 1;

    if let Some((lhs_text, is_simple)) = extract_lhs(node, source, lang) {
        if is_simple {
            // Collect RHS identifiers BEFORE recording the definition
            let mut rhs_vars: HashSet<String> = HashSet::new();
            if let Some(rhs_node) = extract_rhs(node, lang) {
                collect_identifiers(rhs_node, source, &mut rhs_vars);
            }

            // Record flows from RHS variables to LHS (statement-scoped)
            for rhs_var in &rhs_vars {
                if rhs_var != &lhs_text {
                    flows.push(DataFlow {
                        from: rhs_var.clone(),
                        to: lhs_text.clone(),
                        via: None,
                    });
                }
            }

            // Record variable definition
            var_map
                .entry(lhs_text.clone())
                .or_insert_with(|| VariableFlow {
                    name: lhs_text.clone(),
                    defined_at: vec![],
                    used_at: vec![],
                })
                .defined_at
                .push(line);

            // Record RHS variable uses
            for rhs_var in rhs_vars {
                var_map
                    .entry(rhs_var.clone())
                    .or_insert_with(|| VariableFlow {
                        name: rhs_var.clone(),
                        defined_at: vec![],
                        used_at: vec![],
                    })
                    .used_at
                    .push(line);
            }
        }
    }
}

fn track_uses_in_subtree(
    node: tree_sitter::Node,
    source: &str,
    var_map: &mut HashMap<String, VariableFlow>,
) {
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "identifier" {
            let name = text_for(child, source).trim().to_string();
            if !name.is_empty() {
                let line = child.start_position().row + 1;
                var_map
                    .entry(name.clone())
                    .or_insert_with(|| VariableFlow {
                        name: name.clone(),
                        defined_at: vec![],
                        used_at: vec![],
                    })
                    .used_at
                    .push(line);
            }
        } else {
            track_uses_in_subtree(child, source, var_map);
        }
    }
}

fn collect_identifiers(node: tree_sitter::Node, source: &str, out: &mut HashSet<String>) {
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if child.kind() == "identifier" {
            let name = text_for(child, source).trim().to_string();
            if !name.is_empty() {
                out.insert(name);
            }
        } else {
            collect_identifiers(child, source, out);
        }
    }
}

fn text_for(node: tree_sitter::Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()].to_string()
}

impl Default for DFGLayer {
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
    async fn dfg_tracks_assignments() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(x):
    y = x + 1
    z = y * 2
    return z
"#,
        )
        .await
        .unwrap();
        let func = &ASTLayer::new(Language::Python)
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        let dfg = DFGLayer::new().analyze(func).unwrap();
        let names: Vec<&str> = dfg.variables.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"x"), "Should track x");
        assert!(names.contains(&"y"), "Should track y");
        assert!(names.contains(&"z"), "Should track z");
    }

    #[tokio::test]
    async fn dfg_tracks_reassignment() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo():
    x = 1
    x = x + 1
    return x
"#,
        )
        .await
        .unwrap();
        let func = &ASTLayer::new(Language::Python)
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        let dfg = DFGLayer::new().analyze(func).unwrap();
        let x_flow = dfg.variables.iter().find(|v| v.name == "x").unwrap();
        assert_eq!(x_flow.defined_at.len(), 2, "x should be defined at 2 lines");
        assert!(!x_flow.used_at.is_empty(), "x should have uses");
    }

    #[tokio::test]
    async fn dfg_builds_accurate_flows() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(x):
    y = x + 1
    z = y * 2
    return z
"#,
        )
        .await
        .unwrap();
        let func = &ASTLayer::new(Language::Python)
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        let dfg = DFGLayer::new().analyze(func).unwrap();
        assert!(
            dfg.flows.iter().any(|f| f.from == "x" && f.to == "y"),
            "Should have flow x -> y"
        );
        assert!(
            dfg.flows.iter().any(|f| f.from == "y" && f.to == "z"),
            "Should have flow y -> z"
        );
        assert!(
            !dfg.flows.iter().any(|f| f.from == "x" && f.to == "z"),
            "Should NOT have flow x -> z"
        );
    }

    #[tokio::test]
    async fn dfg_rust_let_declaration() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.rs"),
            r#"
fn foo(x: i32) -> i32 {
    let y = x + 1;
    let z = y * 2;
    z
}
"#,
        )
        .await
        .unwrap();
        let func = &ASTLayer::new(Language::Rust)
            .analyze_file(&dir.path().join("t.rs"))
            .await
            .unwrap()
            .functions[0];

        let dfg = DFGLayer::new().analyze(func).unwrap();
        let names: Vec<&str> = dfg.variables.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"x"), "Should track x");
        assert!(names.contains(&"y"), "Should track y");
        assert!(names.contains(&"z"), "Should track z");
    }
}
