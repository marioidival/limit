//! CFG builder using tree-sitter AST traversal

use tree_sitter::Node;

/// Decision points that increase cyclomatic complexity (McCabe's formula: M = 1 + P)
///
/// NOTE: `else_clause` is intentionally NOT included — in McCabe's cyclomatic complexity,
/// `else` is the fallthrough path, not a decision point.
/// `try_statement` IS included as a design choice (some tools exclude it).
const BRANCH_KINDS: &[&str] = &[
    // Python
    "if_statement",           // +1 for each if
    "elif_clause",            // +1 for each elif
    "for_statement",          // +1
    "while_statement",        // +1
    "match_statement",        // +1
    "try_statement",          // +1 (design choice)
    "conditional_expression", // ternary: x if cond else y
    // Rust
    "if_expression",    // +1
    "while_expression", // +1
    "loop_expression",  // +1
    "for_expression",   // +1
    "match_expression", // +1
    "match_arm",        // +1 per arm (standard McCabe: N-1 additional paths)
];

/// Count decision points for cyclomatic complexity: M = 1 + count
pub fn count_branches(node: Node) -> u32 {
    let mut count = 0u32;
    count_branches_recursive(node, &mut count);
    count
}

fn count_branches_recursive(node: Node, count: &mut u32) {
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if BRANCH_KINDS.contains(&child.kind()) {
            *count += 1;
        }
        count_branches_recursive(child, count);
    }
}

pub struct CFGGraph {
    pub blocks: Vec<BasicBlockData>,
    pub edges: Vec<(usize, usize)>,
}

pub struct BasicBlockData {
    pub start_line: usize,
    pub end_line: usize,
    pub statements: Vec<String>,
}

/// Build a CFG with basic blocks split at branch points
pub fn build_cfg(func_node: tree_sitter::Node, source: &str) -> CFGGraph {
    let stmts = collect_top_level_stmts(func_node, source);
    let mut graph = CFGGraph {
        blocks: vec![],
        edges: vec![],
    };
    let mut current_block = BasicBlockData {
        start_line: 0,
        end_line: 0,
        statements: vec![],
    };

    for (line, text, kind) in stmts {
        if is_branch_point(kind) && !current_block.statements.is_empty() {
            current_block.end_line = line.saturating_sub(1);
            graph.blocks.push(std::mem::replace(
                &mut current_block,
                BasicBlockData {
                    start_line: line,
                    end_line: 0,
                    statements: vec![],
                },
            ));
        }
        current_block.statements.push(text);
        current_block.end_line = line;
    }
    if !current_block.statements.is_empty() {
        graph.blocks.push(current_block);
    }
    // Sequential edges between adjacent blocks
    for i in 0..graph.blocks.len().saturating_sub(1) {
        graph.edges.push((i, i + 1));
    }
    graph
}

fn collect_top_level_stmts(
    node: tree_sitter::Node,
    source: &str,
) -> Vec<(usize, String, &'static str)> {
    let mut stmts = Vec::new();
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        if matches!(
            child.kind(),
            "function_definition"
                | "function_item"
                | "identifier"
                | "def"
                | "parameters"
                | "type_parameters"
                | "return_type"
                | "arrow"
        ) {
            continue;
        }
        // If the child is a block/compound_statement, descend into it
        if matches!(child.kind(), "block" | "statement_block") {
            collect_stmts_recursive(child, source, &mut stmts);
            continue;
        }
        let text = source[child.start_byte()..child.end_byte()].to_string();
        if !text.trim().is_empty() {
            stmts.push((child.start_position().row + 1, text, child.kind()));
        }
    }
    stmts
}

/// Collect statements recursively, but only at the top level of the block.
/// Branch nodes (if/for/while/match) are included as-is to mark split points.
fn collect_stmts_recursive(
    node: tree_sitter::Node,
    source: &str,
    stmts: &mut Vec<(usize, String, &'static str)>,
) {
    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        let text = source[child.start_byte()..child.end_byte()].to_string();
        if !text.trim().is_empty() {
            stmts.push((child.start_position().row + 1, text, child.kind()));
        }
    }
}

fn is_branch_point(kind: &str) -> bool {
    BRANCH_KINDS.contains(&kind)
}
