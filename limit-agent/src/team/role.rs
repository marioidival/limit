//! Role definitions for team agents.
//!
//! Each role has a specific system prompt, and optionally a dedicated
//! model and tool whitelist — both configurable via [`RoleConfig`].

use serde::{Deserialize, Serialize};
/// Specialized role for a team agent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    /// Product Manager — understands requirements, product vision.
    PM,
    /// Tech Lead — architecture, task breakdown, validation.
    TL,
    /// Junior Developer — executes tasks with tools.
    Jr,
}

/// Per-role configuration loaded from `[team.roles]` in `config.toml`.
///
/// Any field set to `None` falls back to the active provider's default
/// model (the same one used by the single-agent mode).
///
/// When `provider` is `Some`, a **separate** LLM provider instance is
/// created for this role (cross-provider mixing). When `None`, the main
/// provider is cloned with an optional `max_tokens` override.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoleConfig {
    /// Model override for this role (e.g. `"gpt-4o-mini"` for Jr to save cost).
    ///
    /// **Required** when `provider` is set.
    pub model: Option<String>,
    /// Tool names this role is allowed to use.
    ///
    /// `None` means **all registered tools** are available.
    /// An empty `Vec` means **no tools** (PM by default).
    pub tools: Option<Vec<String>>,
    /// Max tokens override for this role.
    pub max_tokens: Option<u32>,
    /// Override the provider type for this role (e.g. `"openai"`, `"anthropic"`).
    ///
    /// When set, `model` must also be set. The API key is resolved from
    /// environment variables (`{PROVIDER}_API_KEY`).
    pub provider: Option<String>,
    /// Custom base URL for the role's provider endpoint.
    pub base_url: Option<String>,
}

/// Full team section from `config.toml`.
///
/// ```toml
/// [team]
/// default_juniors = 2
/// max_parallel_tasks = 4
/// enable_streaming = true
///
/// [team.roles]
/// pm = { tools = [] }
/// tl = { tools = ["bash"] }
/// jr = { provider = "openai", model = "gpt-4o-mini", max_tokens = 8192 }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSection {
    /// Number of junior agents to spawn (default: 2).
    #[serde(default = "default_juniors")]
    pub default_juniors: usize,
    /// Maximum tasks executed concurrently (default: 4).
    #[serde(default = "default_max_parallel")]
    pub max_parallel_tasks: usize,
    /// Enable streaming output (default: true).
    #[serde(default = "default_true")]
    pub enable_streaming: bool,
    /// Per-role overrides.
    #[serde(default)]
    pub roles: TeamRolesSection,
}

impl Default for TeamSection {
    fn default() -> Self {
        Self {
            default_juniors: default_juniors(),
            max_parallel_tasks: default_max_parallel(),
            enable_streaming: default_true(),
            roles: TeamRolesSection::default(),
        }
    }
}

/// Container for per-role config entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRolesSection {
    #[serde(default)]
    pub pm: RoleConfig,
    #[serde(default)]
    pub tl: RoleConfig,
    #[serde(default)]
    pub jr: RoleConfig,
}

impl Default for TeamRolesSection {
    fn default() -> Self {
        Self {
            pm: RoleConfig {
                model: None,
                tools: Some(vec![]), // PM: no tools by default
                max_tokens: None,
                provider: None,
                base_url: None,
            },
            tl: RoleConfig {
                model: None,
                tools: Some(vec!["bash".to_string()]), // TL: can run validation commands
                max_tokens: None,
                provider: None,
                base_url: None,
            },
            jr: RoleConfig {
                model: None,
                max_tokens: Some(8192),
                tools: Some(
                    // Jr: restricted safe tools only (explicit opt-in for dangerous tools)
                    Role::Jr
                        .default_tools()
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect(),
                ),
                provider: None,
                base_url: None,
            },
        }
    }
}

impl TeamSection {
    /// Parse from a raw `toml::Value` (typically from `Config::team`).
    ///
    /// Returns the default section if the value is missing or unparseable.
    pub fn from_raw(value: &toml::Value) -> Self {
        value.clone().try_into().unwrap_or_default()
    }
}

fn default_juniors() -> usize {
    2
}
fn default_max_parallel() -> usize {
    4
}
fn default_true() -> bool {
    true
}

impl Role {
    /// Returns the system prompt for this role.
    pub fn system_prompt(&self) -> &'static str {
        match self {
            Role::PM => include_str!("prompts/pm.md"),
            Role::TL => include_str!("prompts/tl.md"),
            Role::Jr => include_str!("prompts/jr.md"),
        }
    }

    /// Returns the display label for this role.
    pub fn label(&self) -> &'static str {
        match self {
            Role::PM => "PM",
            Role::TL => "TL",
            Role::Jr => "Jr",
        }
    }

    /// TOML key used for this role in `[team.roles.<key>]`.
    pub fn config_key(&self) -> &'static str {
        match self {
            Role::PM => "pm",
            Role::TL => "tl",
            Role::Jr => "jr",
        }
    }

    /// Default model recommendation for this role.
    ///
    /// Falls back to the active provider's default model when the
    /// per-role [`RoleConfig::model`] is `None`.
    pub fn default_model(&self) -> &'static str {
        match self {
            Role::PM => "gpt-4",
            Role::TL => "gpt-4",
            Role::Jr => "gpt-4o-mini",
        }
    }

    /// Default tool whitelist for this role when no config is provided.
    ///
    /// Returns a vector of tool names. Empty vector means no tools.
    pub fn default_tools(&self) -> Vec<&'static str> {
        match self {
            Role::PM => vec![],
            Role::TL => vec!["bash"],
            Role::Jr => vec!["file_read", "file_write", "file_edit", "bash"],
        }
    }

    /// Retrieve the [`RoleConfig`] for this role from the team section.
    pub fn config_from(role: Role, section: &TeamSection) -> &RoleConfig {
        match role {
            Role::PM => &section.roles.pm,
            Role::TL => &section.roles.tl,
            Role::Jr => &section.roles.jr,
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_labels() {
        assert_eq!(Role::PM.label(), "PM");
        assert_eq!(Role::TL.label(), "TL");
        assert_eq!(Role::Jr.label(), "Jr");
    }

    #[test]
    fn test_role_display() {
        assert_eq!(format!("{}", Role::PM), "PM");
        assert_eq!(format!("{}", Role::TL), "TL");
        assert_eq!(format!("{}", Role::Jr), "Jr");
    }

    #[test]
    fn test_role_system_prompt_not_empty() {
        assert!(!Role::PM.system_prompt().is_empty());
        assert!(!Role::TL.system_prompt().is_empty());
        assert!(!Role::Jr.system_prompt().is_empty());
    }

    #[test]
    fn test_role_serialization() {
        let role = Role::PM;
        let json = serde_json::to_string(&role).unwrap();
        let deserialized: Role = serde_json::from_str(&json).unwrap();
        assert_eq!(role, deserialized);
    }

    #[test]
    fn test_role_config_key() {
        assert_eq!(Role::PM.config_key(), "pm");
        assert_eq!(Role::TL.config_key(), "tl");
        assert_eq!(Role::Jr.config_key(), "jr");
    }

    #[test]
    fn test_team_section_default() {
        let section = TeamSection::default();
        assert_eq!(section.default_juniors, 2);
        assert_eq!(section.max_parallel_tasks, 4);
        assert!(section.enable_streaming);
    }

    #[test]
    fn test_team_section_deserialize_minimal() {
        let toml = "";
        let section: TeamSection = toml::from_str(toml).unwrap();
        assert_eq!(section.default_juniors, 2);
        assert_eq!(section.max_parallel_tasks, 4);
    }

    #[test]
    fn test_team_section_deserialize_full() {
        let toml = r#"
default_juniors = 3
max_parallel_tasks = 8
enable_streaming = false

[roles]
[roles.pm]
tools = []

[roles.tl]
tools = ["bash", "grep"]

[roles.jr]
model = "gpt-4o-mini"
tools = ["file_read", "file_write", "bash"]
"#;
        let section: TeamSection = toml::from_str(toml).unwrap();
        assert_eq!(section.default_juniors, 3);
        assert_eq!(section.max_parallel_tasks, 8);
        assert!(!section.enable_streaming);

        assert_eq!(section.roles.pm.tools.as_deref(), Some([].as_slice()));
        assert_eq!(
            section.roles.tl.tools.as_deref(),
            Some(["bash".to_string(), "grep".to_string()].as_slice())
        );
        assert_eq!(section.roles.jr.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(
            section.roles.jr.tools.as_deref(),
            Some(
                [
                    "file_read".to_string(),
                    "file_write".to_string(),
                    "bash".to_string()
                ]
                .as_slice()
            )
        );
    }

    #[test]
    fn test_config_from_section() {
        let section = TeamSection::default();
        let pm_cfg = Role::config_from(Role::PM, &section);
        assert!(pm_cfg.model.is_none());
        assert_eq!(pm_cfg.tools.as_deref(), Some([].as_slice()));

        let jr_cfg = Role::config_from(Role::Jr, &section);
        assert!(jr_cfg.model.is_none());
        // Jr now defaults to restricted safe tools (not None/all tools)
        assert!(jr_cfg.tools.is_some());
        let tools = jr_cfg.tools.as_deref().unwrap();
        assert!(tools.contains(&"file_read".to_string()));
        assert!(tools.contains(&"bash".to_string()));
        // Should NOT contain dangerous tools like git_push or web_fetch
        assert!(!tools.contains(&"git_push".to_string()));
        assert!(!tools.contains(&"web_fetch".to_string()));
    }

    #[test]
    fn test_role_config_default() {
        let cfg = RoleConfig::default();
        assert!(cfg.model.is_none());
        assert!(cfg.tools.is_none());
    }

    #[test]
    fn test_role_default_model() {
        assert_eq!(Role::PM.default_model(), "gpt-4");
        assert_eq!(Role::TL.default_model(), "gpt-4");
        assert_eq!(Role::Jr.default_model(), "gpt-4o-mini");
    }

    #[test]
    fn test_role_config_with_provider_override() {
        let toml = r#"
[roles]
[roles.pm]
tools = []

[roles.tl]
tools = ["bash"]

[roles.jr]
provider = "openai"
model = "gpt-4o-mini"
max_tokens = 8192
tools = ["file_read", "bash"]
"#;
        let section: TeamSection = toml::from_str(toml).unwrap();
        assert_eq!(section.roles.jr.provider.as_deref(), Some("openai"));
        assert_eq!(section.roles.jr.model.as_deref(), Some("gpt-4o-mini"));
        assert_eq!(section.roles.jr.max_tokens, Some(8192));
        assert!(section.roles.pm.provider.is_none());
        assert!(section.roles.tl.provider.is_none());
    }

    #[test]
    fn test_role_config_with_base_url() {
        let toml = r#"
[roles]
[roles.jr]
provider = "openai"
model = "gpt-4o-mini"
base_url = "https://custom-endpoint.example.com/v1"
"#;
        let section: TeamSection = toml::from_str(toml).unwrap();
        assert_eq!(
            section.roles.jr.base_url.as_deref(),
            Some("https://custom-endpoint.example.com/v1")
        );
    }

    #[test]
    fn test_role_default_tools() {
        assert!(Role::PM.default_tools().is_empty());
        assert_eq!(Role::TL.default_tools(), vec!["bash"]);
        assert_eq!(
            Role::Jr.default_tools(),
            vec!["file_read", "file_write", "file_edit", "bash"]
        );
    }
}
