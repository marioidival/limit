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
        assert_eq!(Language::from_str("typescript").unwrap(), Language::TypeScript);
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
