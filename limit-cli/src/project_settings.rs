//! Project-specific settings storage
//!
//! Manages per-project settings in SQLite (tracking.db).

use crate::error::CliError;
use rusqlite::{params, Connection};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub struct ProjectSettings {
    db_path: PathBuf,
}

impl ProjectSettings {
    pub fn new() -> Result<Self, CliError> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| CliError::ConfigError("Failed to get home directory".to_string()))?;
        let limit_dir = home_dir.join(".limit");
        std::fs::create_dir_all(&limit_dir).map_err(|e| {
            CliError::ConfigError(format!("Failed to create .limit directory: {}", e))
        })?;

        let db_path = limit_dir.join("tracking.db");
        let settings = Self { db_path };
        settings.init_db()?;
        Ok(settings)
    }

    fn init_db(&self) -> Result<(), CliError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| CliError::ConfigError(format!("Failed to open database: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_settings (
                project_hash TEXT PRIMARY KEY,
                warm_enabled INTEGER NOT NULL DEFAULT 0,
                warmed_at TEXT
            )",
            [],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to create table: {}", e)))?;

        Ok(())
    }

    /// Compute a unique hash for a project path
    pub fn hash_project(project_path: &std::path::Path) -> String {
        let canonical = project_path
            .canonicalize()
            .unwrap_or_else(|_| project_path.to_path_buf());

        let mut hasher = DefaultHasher::new();
        canonical.to_string_lossy().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if warm is enabled for a project
    pub fn is_warm_enabled(&self, project_path: &std::path::Path) -> bool {
        let hash = Self::hash_project(project_path);
        let conn = match Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let enabled: i64 = conn
            .query_row(
                "SELECT warm_enabled FROM project_settings WHERE project_hash = ?",
                params![&hash],
                |row| row.get(0),
            )
            .unwrap_or(0);

        enabled != 0
    }

    /// Enable warm for a project
    pub fn set_warm_enabled(&self, project_path: &std::path::Path) -> Result<(), CliError> {
        let hash = Self::hash_project(project_path);
        let conn = Connection::open(&self.db_path)
            .map_err(|e| CliError::ConfigError(format!("Failed to open database: {}", e)))?;

        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO project_settings (project_hash, warm_enabled, warmed_at) VALUES (?1, 1, ?2)",
            params![&hash, &now],
        )
        .map_err(|e| CliError::ConfigError(format!("Failed to update settings: {}", e)))?;

        Ok(())
    }
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self::new().expect("Failed to create ProjectSettings")
    }
}
