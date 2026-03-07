use std::path::PathBuf;
use std::{fs, io};

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Config {
    pub api_key: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_base_url")]
    pub base_url: Option<String>,
}

fn default_model() -> String {
    "claude-3-5-sonnet-20241022".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout() -> u64 {
    60
}

fn default_base_url() -> Option<String> {
    None
}

impl Config {
    pub fn load() -> Result<Self, io::Error> {
        let config_path = config_path();

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let config_content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_key: None,
            model: default_model(),
            max_tokens: default_max_tokens(),
            timeout: default_timeout(),
            base_url: None,
        }
    }
}

fn config_path() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");
    home_dir.join(".limit").join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_file() {
        // Load config from non-existent path
        let config = Config::load().unwrap();

        // Should use defaults
        assert_eq!(config, Config::default());
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout, 60);
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
api_key = "sk-ant-test123"
model = "claude-3-5-sonnet-20241022"
max_tokens = 8192
timeout = 120
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.api_key, Some("sk-ant-test123".to_string()));
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.timeout, 120);
    }

    #[test]
    fn test_load_partial_config_uses_defaults() {
        let config_content = r#"
api_key = "sk-ant-partial"
model = "custom-model"
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.api_key, Some("sk-ant-partial".to_string()));
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.max_tokens, 4096); // default
        assert_eq!(config.timeout, 60); // default
        assert!(config.base_url.is_none()); // default
    }

    #[test]
    fn test_load_config_with_base_url() {
        let config_content = r#"
api_key = "sk-ant-test123"
model = "claude-3-5-sonnet-20241022"
max_tokens = 8192
timeout = 120
base_url = "https://custom.api/endpoint"
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.api_key, Some("sk-ant-test123".to_string()));
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_tokens, 8192);
        assert_eq!(config.timeout, 120);
        assert_eq!(config.base_url, Some("https://custom.api/endpoint".to_string()));
    }

    #[test]
    fn test_load_config_without_base_url() {
        let config_content = r#"
api_key = "sk-ant-test456"
model = "claude-3-5-sonnet-20241022"
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        assert_eq!(config.api_key, Some("sk-ant-test456".to_string()));
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_tokens, 4096); // default
        assert_eq!(config.timeout, 60); // default
        assert!(config.base_url.is_none()); // default
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout, 60);
        assert_eq!(config.model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_tokens, 4096);
        assert_eq!(config.timeout, 60);
        assert!(config.api_key.is_none());
        assert!(config.base_url.is_none());
    }
}
