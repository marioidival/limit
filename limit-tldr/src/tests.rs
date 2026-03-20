//! Tests for TLDR library

use crate::types::Language;
use crate::{Config, Error};
use std::path::PathBuf;

mod config_tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.max_depth, 3);
        assert_eq!(config.language, Language::Auto);
        assert!(config.cache_dir.is_none());
    }

    #[test]
    fn test_config_custom() {
        let config = Config {
            language: Language::Python,
            max_depth: 5,
            cache_dir: Some(PathBuf::from("/tmp/cache")),
        };
        assert_eq!(config.language, Language::Python);
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.cache_dir, Some(PathBuf::from("/tmp/cache")));
    }
}

mod language_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("python").unwrap(), Language::Python);
        assert_eq!(Language::from_str("Python").unwrap(), Language::Python);
        assert_eq!(Language::from_str("rust").unwrap(), Language::Rust);
        assert_eq!(Language::from_str("go").unwrap(), Language::Go);
        assert_eq!(
            Language::from_str("typescript").unwrap(),
            Language::TypeScript
        );
        assert!(Language::from_str("invalid").is_err());
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("xyz"), None);
    }

    #[test]
    fn test_language_extensions() {
        assert!(Language::Python.extensions().contains(&"py"));
        assert!(Language::Rust.extensions().contains(&"rs"));
        assert!(Language::TypeScript.extensions().contains(&"ts"));
        assert!(Language::Auto.extensions().is_empty());
    }

    #[test]
    fn test_language_display() {
        assert_eq!(format!("{}", Language::Python), "python");
        assert_eq!(format!("{}", Language::Rust), "rust");
        assert_eq!(format!("{}", Language::Auto), "auto");
    }
}

mod error_tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::FunctionNotFound("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = Error::LanguageNotSupported("xyz".to_string());
        assert!(err.to_string().contains("xyz"));

        let err = Error::Cache("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }
}

mod token_counter_tests {
    use crate::utils::{count_tokens, estimate_tokens};

    #[test]
    fn test_estimate_tokens() {
        let text = "def hello_world():\n    print('Hello, world!')";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        // Roughly 45 chars / 4 = ~11 tokens
        assert!(tokens > 5 && tokens < 20);
    }

    #[test]
    fn test_count_tokens() {
        let text = "hello world test";
        let tokens = count_tokens(text);
        assert!(tokens >= 3); // At least 3 words
    }

    #[test]
    fn test_count_tokens_with_punctuation() {
        let text = "hello, world! test.";
        let tokens = count_tokens(text);
        assert!(tokens >= 3); // 3 words + 3 punctuation = 6
    }
}

mod api_tests {
    use super::*;
    use crate::TLDR;
    use tempfile::tempdir;

    #[tokio::test]
    async fn tree_returns_file_list() {
        let dir = tempdir().unwrap();
        tokio::fs::create_dir_all(dir.path().join("src"))
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/main.py"), "def foo(): pass")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("src/lib.rs"), "fn bar() {}")
            .await
            .unwrap();

        let mut tldr = TLDR::new(dir.path(), Config::default()).await.unwrap();
        tldr.warm().await.unwrap();

        let tree = tldr.tree().unwrap();
        assert!(tree.len() >= 2);
    }

    #[tokio::test]
    async fn structure_returns_functions() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("test.py"),
            "def foo():\n    pass\ndef bar():\n    return 1",
        )
        .await
        .unwrap();

        let mut tldr = TLDR::new(dir.path(), Config::default()).await.unwrap();
        tldr.warm().await.unwrap();

        let structure = tldr.structure().unwrap();
        assert_eq!(structure.len(), 1);
        assert_eq!(structure[0].functions.len(), 2);
    }

    #[tokio::test]
    async fn search_finds_by_pattern() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("test.py"),
            "def authenticate():\n    pass\ndef authorize():\n    pass",
        )
        .await
        .unwrap();

        let mut tldr = TLDR::new(dir.path(), Config::default()).await.unwrap();
        tldr.warm().await.unwrap();

        let results = tldr.search("auth").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn extract_returns_file_analysis() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("test.py"),
            "import os\ndef foo():\n    pass",
        )
        .await
        .unwrap();

        let mut tldr = TLDR::new(dir.path(), Config::default()).await.unwrap();
        tldr.warm().await.unwrap();

        let analysis = tldr.extract("test.py").unwrap();
        assert_eq!(analysis.imports.len(), 1);
        assert_eq!(analysis.functions.len(), 1);
    }

    #[tokio::test]
    async fn calls_returns_forward_calls() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("test.py"),
            "def foo():\n    return bar()\ndef bar():\n    return 1",
        )
        .await
        .unwrap();

        let mut tldr = TLDR::new(dir.path(), Config::default()).await.unwrap();
        tldr.warm().await.unwrap();

        let calls = tldr.get_calls("foo").unwrap();
        assert!(calls.contains(&"bar".to_string()));
    }

    #[tokio::test]
    async fn imports_returns_file_imports() {
        let dir = tempdir().unwrap();
        tokio::fs::write(
            dir.path().join("test.py"),
            "import os\nimport sys\n\ndef foo():\n    pass",
        )
        .await
        .unwrap();

        let mut tldr = TLDR::new(dir.path(), Config::default()).await.unwrap();
        tldr.warm().await.unwrap();

        let imports = tldr.get_imports("test.py").unwrap();
        assert!(imports.len() >= 2);
        assert!(imports.iter().any(|i| i.module.contains("os")));
        assert!(imports.iter().any(|i| i.module.contains("sys")));
    }

    #[tokio::test]
    async fn importers_returns_file_paths() {
        let dir = tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.py"), "import os\ndef foo():\n    pass")
            .await
            .unwrap();
        tokio::fs::write(
            dir.path().join("b.py"),
            "from sys import argv\ndef bar():\n    pass",
        )
        .await
        .unwrap();

        let mut tldr = TLDR::new(dir.path(), Config::default()).await.unwrap();
        tldr.warm().await.unwrap();

        let importers = tldr.get_importers("os").unwrap();
        assert_eq!(importers.len(), 1);
        assert!(importers[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("a.py"));
    }
}

mod ast_tests {
    use super::*;
    use crate::layers::ast::ASTLayer;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ast_line_numbers() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.py");

        let source = r#"# Line 1
# Line 2
def function_one():
    pass

# Line 6
def function_two(x, y):
    return x + y

# Line 11
def function_three():
    '''A docstring'''
    return 42
"#;
        tokio::fs::write(&file_path, source).await.unwrap();

        let ast_layer = ASTLayer::new(Language::Python);
        let analysis = ast_layer.analyze_file(&file_path).await.unwrap();

        assert!(
            !analysis.functions.is_empty(),
            "Should find at least one function"
        );

        for func in &analysis.functions {
            assert!(
                func.line > 0,
                "Function '{}' should have line > 0, got {}",
                func.name,
                func.line
            );
        }
    }
}
