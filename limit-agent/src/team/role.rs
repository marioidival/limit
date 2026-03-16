//! Role definitions for team agents.
//!
//! Each role has a specific system prompt, default model preference,
//! and set of available tools.

use serde::{Deserialize, Serialize};

/// Specialized role for a team agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Role {
    /// Product Manager — understands requirements, product vision.
    PM,
    /// Tech Lead — architecture, task breakdown, validation.
    TL,
    /// Junior Developer — executes tasks with tools.
    Jr,
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
}
