//! Tree-sitter based parsers for robust AST extraction
//!
//! Provides parsers for all supported languages using tree-sitter.

use std::path::Path;

use crate::error::Result;
use crate::types::{
    CallExpression, ClassInfo, ConstantInfo, FileAnalysis, FunctionInfo, ImportInfo, Language,
    Parameter,
};

/// Parser using tree-sitter
pub struct TreeSitterParser;

impl TreeSitterParser {
    pub fn new() -> Self {
        Self
    }

    pub fn get_ts_language(&self, lang: Language) -> Option<tree_sitter::Language> {
        match lang {
            Language::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Language::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Language::JavaScript => Some(tree_sitter_javascript::LANGUAGE.into()),
            Language::TypeScript => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            Language::Go => Some(tree_sitter_go::LANGUAGE.into()),
            Language::Java => Some(tree_sitter_java::LANGUAGE.into()),
            Language::C => Some(tree_sitter_c::LANGUAGE.into()),
            Language::Cpp => Some(tree_sitter_cpp::LANGUAGE.into()),
            #[cfg(feature = "tree-sitter-extra")]
            Language::Ruby => Some(tree_sitter_ruby::LANGUAGE.into()),
            #[cfg(feature = "tree-sitter-extra")]
            Language::PHP => Some(tree_sitter_php::LANGUAGE_PHP.into()),
            #[cfg(feature = "tree-sitter-extra")]
            Language::CSharp => Some(tree_sitter_c_sharp::LANGUAGE.into()),
            _ => None,
        }
    }

    /// Parse source code and extract AST information
    pub fn parse(&self, source: &str, file: &Path, language: Language) -> Result<FileAnalysis> {
        let mut analysis = FileAnalysis {
            file: file.to_path_buf(),
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            call_expressions: Vec::new(),
            constants: Vec::new(),
            language,
        };

        // Try tree-sitter first, fall back to regex if not available
        if let Some(ts_lang) = self.get_ts_language(language) {
            self.parse_with_tree_sitter(source, &mut analysis, ts_lang)?;
        } else {
            // Fall back to regex-based parsing for unsupported languages
            self.parse_with_regex(source, &mut analysis, file, language)?;
        }

        Ok(analysis)
    }

    /// Parse using tree-sitter
    fn parse_with_tree_sitter(
        &self,
        source: &str,
        analysis: &mut FileAnalysis,
        language: tree_sitter::Language,
    ) -> Result<()> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| crate::error::Error::ParseError {
                file: analysis.file.display().to_string(),
                message: format!("Tree-sitter language error: {}", e),
            })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| crate::error::Error::ParseError {
                file: analysis.file.display().to_string(),
                message: "Failed to parse source".to_string(),
            })?;

        let root = tree.root_node();
        self.walk_node(root, source, analysis, &mut None);

        // Set file paths for all extracted functions and classes
        for func in &mut analysis.functions {
            func.file = analysis.file.clone();
        }
        for cls in &mut analysis.classes {
            cls.file = analysis.file.clone();
        }
        for call in &mut analysis.call_expressions {
            call.file = analysis.file.clone();
        }
        for constant in &mut analysis.constants {
            constant.file = analysis.file.clone();
        }

        Ok(())
    }

    /// Walk the tree-sitter tree and extract information
    fn walk_node(
        &self,
        node: tree_sitter::Node,
        source: &str,
        analysis: &mut FileAnalysis,
        current_function: &mut Option<String>,
    ) {
        let cursor = &mut node.walk();

        for child in node.children(cursor) {
            match child.kind() {
                // Function definitions
                "function_definition"
                | "function_item"
                | "method_definition"
                | "arrow_function" => {
                    if let Some(func) = self.extract_function(child, source) {
                        let name = func.name.clone();
                        analysis.functions.push(func);
                        let prev = current_function.take();
                        *current_function = Some(name);
                        self.walk_node(child, source, analysis, current_function);
                        *current_function = prev;
                        continue;
                    }
                }
                // Class/struct/trait definitions
                "class_definition"
                | "struct_item"
                | "trait_item"
                | "class_declaration"
                | "interface_declaration" => {
                    if let Some(class) = self.extract_class(child, source) {
                        analysis.classes.push(class);
                    }
                }
                // Imports
                "import_statement" | "import_declaration" | "use_declaration"
                | "include_directive" => {
                    if let Some(import) = self.extract_import(child, source) {
                        analysis.imports.push(import);
                    }
                }
                // Call expressions
                "call_expression" | "call" | "function_call" | "member_call" => {
                    if let Some(caller) = current_function.as_deref() {
                        if let Some(call) = self.extract_call(child, source, caller) {
                            analysis.call_expressions.push(call);
                        }
                    }
                }
                // Constants/variables
                "const_item" | "static_item" => {
                    let node_text = self.node_text(child, source);
                    let is_mutable = node_text.contains("mut");
                    if let Some(constant) = self.extract_constant(child, source, is_mutable) {
                        analysis.constants.push(constant);
                    }
                }
                "assignment" => {
                    // Python module-level ALL_CAPS heuristic
                    if let Some(constant) = self.extract_python_constant(child, source) {
                        analysis.constants.push(constant);
                    }
                }
                "variable_declaration" | "lexical_declaration" => {
                    // TypeScript/JavaScript const/let
                    if let Some(constant) = self.extract_ts_js_constant(child, source) {
                        analysis.constants.push(constant);
                    }
                }
                "const_declaration" | "var_declaration" => {
                    // Go const/var
                    if let Some(constants) = self.extract_go_constants(child, source) {
                        analysis.constants.extend(constants);
                    }
                }
                _ => {}
            }

            // Recurse into children
            self.walk_node(child, source, analysis, current_function);
        }
    }

    /// Extract function information from a tree-sitter node
    fn extract_function(&self, node: tree_sitter::Node, source: &str) -> Option<FunctionInfo> {
        let name = self.find_child_by_field(node, "name", source)?;
        let params_text = self
            .find_child_by_field(node, "parameters", source)
            .unwrap_or_default();
        let return_type = self.find_child_by_field(node, "return_type", source);

        let full_text = self.node_text(node, source);
        let is_async = full_text.contains("async");
        // Extract only the signature line (fn/async fn ... {), not the full body
        let signature = full_text.lines().next().unwrap_or(&full_text).to_string();

        let params = self.parse_params(&params_text);

        Some(FunctionInfo {
            name,
            signature,
            params,
            return_type,
            is_async,
            line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            file: std::path::PathBuf::new(), // Set by caller
            docstring: None,
            complexity: None,
        })
    }

    /// Extract class information from a tree-sitter node
    fn extract_class(&self, node: tree_sitter::Node, source: &str) -> Option<ClassInfo> {
        let name = self.find_child_by_field(node, "name", source)?;

        Some(ClassInfo {
            name,
            methods: Vec::new(),
            fields: Vec::new(),
            line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            file: std::path::PathBuf::new(),
            docstring: None,
        })
    }

    /// Extract import information from a tree-sitter node
    fn extract_import(&self, node: tree_sitter::Node, source: &str) -> Option<ImportInfo> {
        let text = self.node_text(node, source);

        Some(ImportInfo {
            module: text.clone(),
            names: Vec::new(),
            alias: None,
            line: node.start_position().row + 1,
        })
    }

    /// Extract call expression from a tree-sitter node
    fn extract_call(
        &self,
        node: tree_sitter::Node,
        source: &str,
        caller: &str,
    ) -> Option<CallExpression> {
        let function_node = node
            .child_by_field_name("function")
            .or_else(|| node.child(0))?;

        let callee = self.node_text(function_node, source);
        // Take last segment: self.method() -> method, foo.bar.baz() -> baz, baz::qux() -> qux
        let callee = callee
            .split(['.', ':'])
            .rfind(|s| !s.is_empty())
            .unwrap_or(&callee)
            .trim()
            .to_string();

        if callee.is_empty() || callee == "self" {
            return None;
        }

        Some(CallExpression {
            caller: caller.to_string(),
            callee,
            line: node.start_position().row + 1,
            file: std::path::PathBuf::new(), // Set by caller
        })
    }

    /// Extract constant information from Rust const/static nodes
    fn extract_constant(
        &self,
        node: tree_sitter::Node,
        source: &str,
        is_mutable: bool,
    ) -> Option<ConstantInfo> {
        let name = self.find_child_by_field(node, "name", source)?;
        let type_annotation = self.find_child_by_field(node, "type", source);
        let value = self
            .find_child_by_field(node, "value", source)
            .map(|v| self.truncate_value(&v));

        Some(ConstantInfo {
            name,
            type_annotation,
            value,
            line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            file: std::path::PathBuf::new(),
            docstring: None,
            is_mutable,
        })
    }

    /// Extract Python constants using ALL_CAPS heuristic
    fn extract_python_constant(
        &self,
        node: tree_sitter::Node,
        source: &str,
    ) -> Option<ConstantInfo> {
        let left = node.child_by_field_name("left")?;
        let right = node.child_by_field_name("right")?;

        let name = self.node_text(left, source).trim().to_string();
        let value = self.node_text(right, source);

        // Only extract if name is ALL_CAPS
        let is_all_caps = name.chars().all(|c| c.is_uppercase() || c == '_') && name.contains('_');

        if is_all_caps {
            Some(ConstantInfo {
                name,
                type_annotation: None,
                value: Some(self.truncate_value(&value)),
                line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                file: std::path::PathBuf::new(),
                docstring: None,
                is_mutable: false,
            })
        } else {
            None
        }
    }

    /// Extract TypeScript/JavaScript const/let declarations
    fn extract_ts_js_constant(
        &self,
        node: tree_sitter::Node,
        source: &str,
    ) -> Option<ConstantInfo> {
        let node_text = self.node_text(node, source);
        let is_mutable = node_text.starts_with("let") || node_text.starts_with("var");

        // Find the variable_declarator child
        let mut cursor = node.walk();
        let declarator = node
            .children(&mut cursor)
            .find(|child| child.kind() == "variable_declarator")?;

        let name = declarator.child_by_field_name("name")?;
        let name_text = self.node_text(name, source);

        let value = declarator
            .child_by_field_name("value")
            .map(|v| self.node_text(v, source));

        Some(ConstantInfo {
            name: name_text,
            type_annotation: None,
            value: value.map(|v| self.truncate_value(&v)),
            line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            file: std::path::PathBuf::new(),
            docstring: None,
            is_mutable,
        })
    }

    /// Extract Go const/var declarations (can have multiple in parenthesized block)
    fn extract_go_constants(
        &self,
        node: tree_sitter::Node,
        source: &str,
    ) -> Option<Vec<ConstantInfo>> {
        let node_text = self.node_text(node, source);
        let is_mutable = node_text.starts_with("var");

        let mut constants = Vec::new();
        let mut cursor = node.walk();

        // Go can have multiple constants in a parenthesized block or single declaration
        for child in node.children(&mut cursor) {
            if child.kind() == "const_spec" || child.kind() == "var_spec" {
                if let Some(name) = child.child_by_field_name("name") {
                    let name_text = self.node_text(name, source);
                    let value = child
                        .child_by_field_name("value")
                        .map(|v| self.node_text(v, source));

                    constants.push(ConstantInfo {
                        name: name_text,
                        type_annotation: None,
                        value: value.map(|v| self.truncate_value(&v)),
                        line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        file: std::path::PathBuf::new(),
                        docstring: None,
                        is_mutable,
                    });
                }
            }
        }

        if constants.is_empty() {
            None
        } else {
            Some(constants)
        }
    }

    /// Truncate value to 200 characters maximum
    fn truncate_value(&self, value: &str) -> String {
        let char_count = value.chars().count();
        if char_count > 200 {
            let truncated: String = value.chars().take(197).collect();
            format!("{truncated}...")
        } else {
            value.to_string()
        }
    }

    /// Find child node by field name
    fn find_child_by_field(
        &self,
        node: tree_sitter::Node,
        field: &str,
        source: &str,
    ) -> Option<String> {
        let child = node.child_by_field_name(field)?;
        Some(self.node_text(child, source))
    }

    /// Get text for a node
    fn node_text(&self, node: tree_sitter::Node, source: &str) -> String {
        let start = node.start_byte();
        let end = node.end_byte();
        source[start..end].to_string()
    }

    /// Parse parameter string into parameters
    fn parse_params(&self, params_text: &str) -> Vec<Parameter> {
        let trimmed = params_text.trim_start_matches('(').trim_end_matches(')');

        if trimmed.is_empty() {
            return Vec::new();
        }

        trimmed
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let parts: Vec<&str> = s.trim().splitn(2, ':').collect();
                Parameter {
                    name: parts[0].trim().to_string(),
                    type_annotation: parts.get(1).map(|t| t.trim().to_string()),
                    default_value: None,
                }
            })
            .collect()
    }

    /// Fallback regex-based parsing
    fn parse_with_regex(
        &self,
        source: &str,
        analysis: &mut FileAnalysis,
        file: &Path,
        language: Language,
    ) -> Result<()> {
        use regex::Regex;

        match language {
            Language::Python => {
                let func_re = Regex::new(r"(?m)^(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)").unwrap();

                for cap in func_re.captures_iter(source) {
                    analysis.functions.push(FunctionInfo {
                        name: cap[1].to_string(),
                        signature: cap[0].to_string(),
                        params: Vec::new(),
                        return_type: None,
                        is_async: cap[0].starts_with("async"),
                        line: 0,
                        end_line: 0,
                        file: file.to_path_buf(),
                        docstring: None,
                        complexity: None,
                    });
                }
            }
            Language::Rust => {
                let func_re =
                    Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)").unwrap();

                for cap in func_re.captures_iter(source) {
                    analysis.functions.push(FunctionInfo {
                        name: cap[1].to_string(),
                        signature: cap[0].to_string(),
                        params: Vec::new(),
                        return_type: None,
                        is_async: cap[0].contains("async"),
                        line: 0,
                        end_line: 0,
                        file: file.to_path_buf(),
                        docstring: None,
                        complexity: None,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Find a function's tree-sitter node by name (shared utility for CFG/DFG/PDG)
    pub fn find_function_node<'a>(
        root: tree_sitter::Node<'a>,
        name: &str,
        source: &str,
    ) -> Option<tree_sitter::Node<'a>> {
        let cursor = &mut root.walk();
        for child in root.children(cursor) {
            if matches!(
                child.kind(),
                "function_definition" | "function_item" | "method_definition" | "arrow_function"
            ) {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let func_name = &source[name_node.start_byte()..name_node.end_byte()];
                    if func_name == name {
                        return Some(child);
                    }
                }
            }
            if let Some(found) = Self::find_function_node(child, name, source) {
                return Some(found);
            }
        }
        None
    }
}

impl Default for TreeSitterParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod struct_extraction_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rust_struct_extraction() {
        let source = r#"pub struct AppConfig {
    name: String,
    port: u16,
}"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(
            analysis.classes.len(),
            1,
            "Should find 1 struct, found {}: {:?}",
            analysis.classes.len(),
            analysis.classes.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        assert_eq!(analysis.classes[0].name, "AppConfig");
        assert_eq!(analysis.classes[0].line, 1);
        assert!(analysis.classes[0].end_line >= 3, "end_line should be >= 3");
        assert_eq!(
            analysis.classes[0].file,
            PathBuf::from("test.rs"),
            "ClassInfo.file should be set to the parsed file path"
        );
    }

    #[test]
    fn rust_struct_with_cfg_attrs() {
        let source = r#"pub struct SemanticIndex {
    entries: Vec<String>,
    #[cfg(feature = "semantic")]
    embeddings: Option<Vec<Vec<f32>>>,
}"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(
            analysis.classes.len(),
            1,
            "Should find 1 struct with cfg attrs, found {}",
            analysis.classes.len()
        );
        assert_eq!(analysis.classes[0].name, "SemanticIndex");
    }

    #[test]
    fn rust_struct_and_function_together() {
        let source = r#"pub struct MyStruct {
    value: i32,
}

pub fn my_function() -> MyStruct {
    MyStruct { value: 42 }
}"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(analysis.classes.len(), 1, "Should find 1 struct");
        assert_eq!(analysis.functions.len(), 1, "Should find 1 function");
        assert_eq!(analysis.classes[0].name, "MyStruct");
        assert_eq!(analysis.functions[0].name, "my_function");
    }

    #[test]
    fn rust_trait_extraction() {
        let source = r#"pub trait LlmProvider {
    async fn complete(&self, prompt: &str) -> Result<String>;
}"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(analysis.classes.len(), 1, "Should find 1 trait");
        assert_eq!(analysis.classes[0].name, "LlmProvider");
        assert_eq!(analysis.classes[0].line, 1);
    }
}

#[cfg(test)]
mod node_finder_tests {
    use super::*;

    #[test]
    fn finds_python_function() {
        let source = "def hello():\n    pass\ndef world():\n    pass";
        let parser = TreeSitterParser::new();
        let ts_lang = parser.get_ts_language(Language::Python).unwrap();
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&ts_lang).unwrap();
        let tree = ts_parser.parse(source, None).unwrap();

        let found = TreeSitterParser::find_function_node(tree.root_node(), "hello", source);
        assert!(found.is_some(), "Should find hello function");
        assert_eq!(found.unwrap().start_position().row, 0);
    }

    #[test]
    fn returns_none_for_missing_function() {
        let source = "def hello():\n    pass";
        let parser = TreeSitterParser::new();
        let ts_lang = parser.get_ts_language(Language::Python).unwrap();
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&ts_lang).unwrap();
        let tree = ts_parser.parse(source, None).unwrap();

        assert!(
            TreeSitterParser::find_function_node(tree.root_node(), "missing", source).is_none()
        );
    }

    #[test]
    fn finds_rust_function() {
        let source = "fn main() { bar(); }\nfn bar() {}";
        let parser = TreeSitterParser::new();
        let ts_lang = parser.get_ts_language(Language::Rust).unwrap();
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&ts_lang).unwrap();
        let tree = ts_parser.parse(source, None).unwrap();

        let found = TreeSitterParser::find_function_node(tree.root_node(), "bar", source);
        assert!(found.is_some());
        assert_eq!(found.unwrap().start_position().row, 1);
    }
}

#[cfg(test)]
mod call_extraction_tests {
    use super::*;

    #[test]
    fn rust_call_expressions_extracted() {
        let source = r#"fn main() {
    let x = foo();
    bar(x);
    baz::qux();
}"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(analysis.functions.len(), 1);
        assert_eq!(analysis.call_expressions.len(), 3);

        assert_eq!(analysis.call_expressions[0].caller, "main");
        assert_eq!(analysis.call_expressions[0].callee, "foo");
        assert_eq!(analysis.call_expressions[0].line, 2);

        assert_eq!(analysis.call_expressions[1].caller, "main");
        assert_eq!(analysis.call_expressions[1].callee, "bar");

        assert_eq!(analysis.call_expressions[2].caller, "main");
        assert_eq!(analysis.call_expressions[2].callee, "qux");
    }

    #[test]
    fn python_call_expressions_extracted() {
        let source = r#"def handler():
    validate()
    process(data)
"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.py"), Language::Python)
            .unwrap();

        assert_eq!(analysis.functions.len(), 1);
        assert_eq!(analysis.call_expressions.len(), 2);

        assert_eq!(analysis.call_expressions[0].caller, "handler");
        assert_eq!(analysis.call_expressions[0].callee, "validate");
        assert_eq!(analysis.call_expressions[1].callee, "process");
    }

    #[test]
    fn calls_outside_functions_not_extracted() {
        let source = r#"fn main() {
    inner()
}
fn inner() {}
inner();"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        // Only the call inside main() should be captured
        assert_eq!(analysis.call_expressions.len(), 1);
        assert_eq!(analysis.call_expressions[0].caller, "main");
        assert_eq!(analysis.call_expressions[0].callee, "inner");
    }
}

#[cfg(test)]
mod constant_extraction_tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn rust_const_extraction() {
        let source = r#"const MAX_RETRIES: u32 = 3;
const SYSTEM_PROMPT: &str = "You are a helpful assistant";"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(analysis.constants.len(), 2);
        assert_eq!(analysis.constants[0].name, "MAX_RETRIES");
        assert_eq!(
            analysis.constants[0].type_annotation,
            Some("u32".to_string())
        );
        assert_eq!(analysis.constants[0].value, Some("3".to_string()));
        assert!(!analysis.constants[0].is_mutable);
    }

    #[test]
    fn rust_static_extraction() {
        let source = r#"static GLOBAL_COUNTER: AtomicUsize = AtomicUsize::new(0);
static mut LEGACY_STATE: bool = false;"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(analysis.constants.len(), 2);
        assert_eq!(analysis.constants[0].name, "GLOBAL_COUNTER");
        assert!(!analysis.constants[0].is_mutable);
        assert_eq!(analysis.constants[1].name, "LEGACY_STATE");
        assert!(analysis.constants[1].is_mutable);
    }

    #[test]
    fn python_module_constant_heuristic() {
        let source = r#"MAX_CONNECTIONS = 100
DEFAULT_TIMEOUT = 30
not_a_constant = "lowercase""#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.py"), Language::Python)
            .unwrap();

        // Should extract ALL_CAPS constants only
        assert_eq!(analysis.constants.len(), 2);
        assert_eq!(analysis.constants[0].name, "MAX_CONNECTIONS");
        assert_eq!(analysis.constants[1].name, "DEFAULT_TIMEOUT");
    }

    #[test]
    fn typescript_const_extraction() {
        let source = r#"const API_KEY = "secret";
const MAX_ITEMS: number = 100;
let mutableVar = "changeable";"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.ts"), Language::TypeScript)
            .unwrap();

        assert_eq!(analysis.constants.len(), 3);
        assert_eq!(analysis.constants[0].name, "API_KEY");
        assert!(!analysis.constants[0].is_mutable);
        assert_eq!(analysis.constants[2].name, "mutableVar");
        assert!(analysis.constants[2].is_mutable);
    }

    #[test]
    fn go_const_extraction() {
        let source = r#"const (
    MaxRetries = 3
    DefaultTimeout = 30
)"#;
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(source, Path::new("test.go"), Language::Go)
            .unwrap();

        assert!(analysis.constants.len() >= 2);
    }

    #[test]
    fn value_truncation() {
        let long_value = "x".repeat(300);
        let source = format!(r#"const LONG_VALUE: &str = "{}";"#, long_value);
        let parser = TreeSitterParser::new();
        let analysis = parser
            .parse(&source, Path::new("test.rs"), Language::Rust)
            .unwrap();

        assert_eq!(analysis.constants.len(), 1);
        assert!(analysis.constants[0].value.as_ref().unwrap().len() <= 200);
    }
}
