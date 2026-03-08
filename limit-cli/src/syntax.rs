use std::io;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;
use thiserror::Error;

/// Syntax highlighting errors
#[derive(Debug, Error)]
pub enum HighlightError {
    #[error("Syntax not found: {0}")]
    SyntaxNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Parsing error: {0}")]
    Parse(String),
}

/// Syntax highlighter supporting 100+ languages
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl SyntaxHighlighter {
    /// Load default syntax set and theme
    pub fn new() -> Result<Self, HighlightError> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();

        Ok(Self { syntax_set, theme })
    }

    /// Load with a custom theme from the built-in theme set
    pub fn with_theme(theme_name: &str) -> Result<Self, HighlightError> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        let theme = theme_set
            .themes
            .get(theme_name)
            .ok_or_else(|| HighlightError::SyntaxNotFound(theme_name.to_string()))?
            .clone();

        Ok(Self { syntax_set, theme })
    }

    /// List available built-in themes
    pub fn list_builtin_themes() -> Vec<&'static str> {
        vec![
            "base16-ocean.dark",
            "base16-ocean.light",
            "Solarized (dark)",
            "Solarized (light)",
            "InspiredGitHub",
            "Monokai Extended",
            "Nord",
        ]
    }

    /// Detect and return syntax reference for a language identifier
    pub fn detect_language(&self, lang: &str) -> SyntaxReference {
        let lang_lower = lang.to_lowercase();

        let token = match lang_lower.as_str() {
            "rust" | "rs" => "Rust",
            "python" | "py" => "Python",
            "typescript" | "ts" => "TypeScript",
            "tsx" => "TypeScript JSX",
            "javascript" | "js" => "JavaScript",
            "javascript react" | "jsx" => "JavaScript (Babel)",
            "go" | "golang" => "Go",
            "java" => "Java",
            "c" => "C",
            "cpp" | "c++" | "cxx" => "C++",
            "csharp" | "c#" | "cs" => "C#",
            "ruby" | "rb" => "Ruby",
            "php" => "PHP",
            "html" | "htm" => "HTML",
            "xml" => "XML",
            "css" => "CSS",
            "scss" | "sass" => "SCSS",
            "sql" => "SQL",
            "bash" | "sh" | "shell" => "Bash",
            "zsh" => "Shell Script (zsh)",
            "fish" => "Fish",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "toml" => "TOML",
            "ini" => "INI",
            "markdown" | "md" => "Markdown",

            "lua" => "Lua",
            "r" => "R",
            "scala" => "Scala",
            "kotlin" | "kt" => "Kotlin",
            "swift" => "Swift",
            "dart" => "Dart",
            "elixir" | "ex" => "Elixir",
            "erlang" | "erl" => "Erlang",
            "haskell" | "hs" => "Haskell",
            "clojure" | "clj" => "Clojure",
            "fsharp" | "fs" => "F#",
            "ocaml" | "ml" => "OCaml",
            "elm" => "Elm",
            "purescript" | "purs" => "PureScript",
            "reason" | "re" => "Reason",
            "nix" => "Nix",
            "dockerfile" => "Dockerfile",
            "makefile" => "Makefile",
            "cmake" => "CMake",
            "gradle" => "Gradle",
            "groovy" => "Groovy",
            "powershell" | "ps1" => "PowerShell",
            "vue" => "Vue",
            "svelte" => "Svelte",
            "solidity" | "sol" => "Solidity",
            "asm" | "assembly" | "nasm" => "Assembly",
            "verilog" => "Verilog",
            "vhdl" => "VHDL",
            "matlab" => "MATLAB",
            "julia" => "Julia",
            "nim" => "Nim",
            "racket" => "Racket",
            "scheme" => "Scheme",
            "lisp" | "cl" => "Lisp",
            "commonlisp" => "Common Lisp",
            "cobol" => "COBOL",
            "fortran" => "Fortran",
            "pascal" => "Pascal",
            "ada" => "Ada",
            "crystal" => "Crystal",
            "wren" => "Wren",
            "zig" => "Zig",
            "v" => "V",
            "odin" => "Odin",
            "gleam" => "Gleam",
            _ => {
                // Try to find by token directly
                if let Some(syntax) = self.syntax_set.find_syntax_by_token(lang) {
                    return syntax.clone();
                }
                // Fallback to plain text
                return self.syntax_set.find_syntax_plain_text().clone();
            }
        };

        self.syntax_set
            .find_syntax_by_token(token)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
            .clone()
    }
    /// Highlight code for terminal ANSI output (used in CLI/REPL)
    pub fn highlight_to_ansi(&self, code: &str, lang: &str) -> Result<String, HighlightError> {
        use syntect::util::as_24_bit_terminal_escaped;

        let syntax = self.detect_language(lang);
        let mut highlighter = HighlightLines::new(&syntax, &self.theme);

        let mut output = String::new();
        for line in LinesWithEndings::from(code) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .map_err(|e| HighlightError::Parse(e.to_string()))?;
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            output.push_str(&escaped);
        }

        Ok(output)
    }

    /// Get theme name (for debugging/display)
    pub fn theme_name(&self) -> &str {
        // Theme doesn't store the name, so we return a default
        "base16-ocean.dark"
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new().expect("Failed to initialize syntax highlighter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_new() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_highlighter_default() {
        let highlighter = SyntaxHighlighter::default();
        assert_eq!(highlighter.theme_name(), "base16-ocean.dark");
    }

    #[test]
    fn test_highlight_rust_code() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let code = r#"fn main() {
    println!("Hello, world!");
}"#;

        let result = highlighter.highlight_to_ansi(code, "rust");
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        assert!(highlighted.contains("\x1b[")); // ANSI codes present
    }

    #[test]
    fn test_highlight_python_code() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let code = "def hello():\n    print('world')";

        let result = highlighter.highlight_to_ansi(code, "python");
        assert!(result.is_ok());
        assert!(result.unwrap().contains("\x1b[")); // ANSI codes present
    }

    #[test]
    fn test_highlight_javascript_code() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let code = "const x = 42;\nconsole.log(x);";

        let result = highlighter.highlight_to_ansi(code, "javascript");
        assert!(result.is_ok());
    }

    #[test]
    fn test_highlight_json() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let code = r#"{"key": "value"}"#;

        let result = highlighter.highlight_to_ansi(code, "json");
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_languages() {
        let highlighter = SyntaxHighlighter::new().unwrap();

        let rust_syntax = highlighter.detect_language("rust");
        assert_eq!(rust_syntax.name, "Rust");

        let py_syntax = highlighter.detect_language("python");
        assert_eq!(py_syntax.name, "Python");

        let ts_syntax = highlighter.detect_language("typescript");
        assert_eq!(ts_syntax.name, "TypeScript");

        let js_syntax = highlighter.detect_language("javascript");
        assert_eq!(js_syntax.name, "JavaScript");

        let go_syntax = highlighter.detect_language("go");
        assert_eq!(go_syntax.name, "Go");

        let yaml_syntax = highlighter.detect_language("yaml");
        assert_eq!(yaml_syntax.name, "YAML");

        let json_syntax = highlighter.detect_language("json");
        assert_eq!(json_syntax.name, "JSON");

        let plain = highlighter.detect_language("unknown");
        assert_eq!(plain.name, "Plain Text");
    }

    #[test]
    fn test_fallback_on_empty_code() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let empty_code = "";

        let result = highlighter.highlight_to_ansi(empty_code, "rust");
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_themes() {
        let themes = SyntaxHighlighter::list_builtin_themes();
        assert!(!themes.is_empty());
        assert!(themes.contains(&"base16-ocean.dark"));
        assert!(themes.contains(&"Solarized (dark)"));
        assert!(themes.contains(&"Nord"));
    }

    #[test]
    fn test_with_theme_valid() {
        let highlighter = SyntaxHighlighter::with_theme("Solarized (dark)");
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_with_theme_invalid() {
        let highlighter = SyntaxHighlighter::with_theme("invalid-theme-name");
        assert!(highlighter.is_err());
    }

    #[test]
    fn test_language_aliases() {
        let highlighter = SyntaxHighlighter::new().unwrap();

        // Test various aliases
        assert_eq!(highlighter.detect_language("rs").name, "Rust");
        assert_eq!(highlighter.detect_language("py").name, "Python");
        assert_eq!(highlighter.detect_language("js").name, "JavaScript");
        assert_eq!(highlighter.detect_language("ts").name, "TypeScript");
        assert_eq!(highlighter.detect_language("yml").name, "YAML");
        assert_eq!(highlighter.detect_language("sh").name, "Bash");
    }

    #[test]
    fn test_multiline_code() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let code = r#"fn main() {
    // This is a comment
    let x = 42;
    println!("{}", x);
}"#;

        let result = highlighter.highlight_to_ansi(code, "rust");
        assert!(result.is_ok());
        let highlighted = result.unwrap();
        assert!(highlighted.contains("\x1b[")); // ANSI codes present
    }

    #[test]
    fn test_special_characters_in_code() {
        let highlighter = SyntaxHighlighter::new().unwrap();
        let code = r#"let s = "特殊字符 \n\t\r"; println!("{}", s);"#;

        let result = highlighter.highlight_to_ansi(code, "rust");
        assert!(result.is_ok());
    }
}
