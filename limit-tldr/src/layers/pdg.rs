//! Layer 5: PDG (Program Dependence Graph) - "What affects this line?"
//!
//! Backward slice: find all lines that define or affect variables used on the target line.
//! Uses DFG data flows for accurate tracing (not line-proximity heuristics).

use std::collections::{HashMap, HashSet};

use crate::error::Result;
use crate::layers::dfg::DFGLayer;
use crate::types::{FunctionInfo, SliceInfo};

pub struct PDGLayer {
    dfg: DFGLayer,
}

impl PDGLayer {
    pub fn new() -> Self {
        Self {
            dfg: DFGLayer::new(),
        }
    }

    pub fn slice(&self, func: &FunctionInfo, target_line: usize) -> Result<SliceInfo> {
        let source = std::fs::read_to_string(&func.file)
            .map_err(|e| crate::error::Error::PathNotFound(func.file.display().to_string(), e))?;

        let dfg_info = self.dfg.analyze(func)?;
        let lines: Vec<&str> = source.lines().collect();

        // Build reverse map: variable -> lines where it's defined
        let mut def_map: HashMap<&str, Vec<usize>> = HashMap::new();
        for var in &dfg_info.variables {
            def_map
                .entry(&var.name)
                .or_default()
                .extend(var.defined_at.iter().copied());
        }

        // Build adjacency from DFG flows: var -> set of vars it depends on
        let mut depends_on: HashMap<&str, HashSet<&str>> = HashMap::new();
        for flow in &dfg_info.flows {
            depends_on.entry(&flow.to).or_default().insert(&flow.from);
        }

        // Collect all variables used on target_line
        let mut vars_to_trace: HashSet<String> = HashSet::new();
        for var in &dfg_info.variables {
            if var.used_at.contains(&target_line) {
                vars_to_trace.insert(var.name.clone());
            }
        }

        // BFS backward through data flows
        let mut slice_lines: HashSet<usize> = HashSet::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: Vec<String> = vars_to_trace.into_iter().collect();

        while let Some(var_name) = queue.pop() {
            if !visited.insert(var_name.clone()) {
                continue;
            }

            // Add all definition lines for this variable
            if let Some(defs) = def_map.get(var_name.as_str()) {
                for &def_line in defs {
                    if def_line >= func.line && def_line <= func.end_line {
                        slice_lines.insert(def_line);
                    }
                }
            }

            // Follow data flow edges backward
            if let Some(deps) = depends_on.get(var_name.as_str()) {
                for dep in deps {
                    if !visited.contains(*dep) {
                        queue.push(dep.to_string());
                    }
                }
            }
        }

        slice_lines.insert(target_line);
        let mut sorted: Vec<usize> = slice_lines.into_iter().collect();
        sorted.sort();

        let slice_code: Vec<String> = sorted
            .iter()
            .filter_map(|&line| lines.get(line.saturating_sub(1)).map(|s| s.to_string()))
            .collect();

        Ok(SliceInfo {
            target_line,
            slice: sorted,
            slice_code,
        })
    }
}

impl Default for PDGLayer {
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
    async fn pdg_slice_finds_relevant_lines() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def login(user):
    db_user = get_user(user)
    if db_user is None:
        raise NotFound
    token = create_token(db_user)
    session.token = token
    return session
"#,
        )
        .await
        .unwrap();
        let func = &ASTLayer::new(Language::Python)
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        let slice = PDGLayer::new().slice(func, 6).unwrap(); // line 6: return session
        assert!(
            slice.slice.len() >= 3,
            "Slice should contain multiple lines, got {}",
            slice.slice.len()
        );
        assert!(!slice.slice_code.is_empty());
    }

    #[tokio::test]
    async fn pdg_slice_includes_target_line() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("t.py"),
            r#"
def foo(x):
    y = x + 1
    return y
"#,
        )
        .await
        .unwrap();
        let func = &ASTLayer::new(Language::Python)
            .analyze_file(&dir.path().join("t.py"))
            .await
            .unwrap()
            .functions[0];

        let slice = PDGLayer::new().slice(func, 3).unwrap(); // line 3: return y
        assert!(
            slice.slice.contains(&3),
            "Slice should include target line 3"
        );
    }
}
