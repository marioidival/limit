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
