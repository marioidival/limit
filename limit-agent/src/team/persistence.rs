//! Team persistence — save and load teams to/from `~/.limit/teams/`.
//!
//! Each team is stored as a JSON file containing its config, history,
//! and metadata. This allows teams to survive CLI restarts.

use crate::team::history::TeamHistory;
use crate::team::TeamConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Serialized state of a team, stored as JSON on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSnapshot {
    /// Team name.
    pub name: String,
    /// Configuration used when the team was created.
    pub config: TeamConfig,
    /// Event history up to the last save point.
    pub history: TeamHistory,
    /// When this snapshot was created.
    pub created_at: DateTime<Utc>,
    /// When this snapshot was last updated.
    pub updated_at: DateTime<Utc>,
    /// Number of times the team has been executed.
    pub run_count: usize,
}

impl TeamSnapshot {
    /// Create a new snapshot for a team.
    pub fn new(name: &str, config: TeamConfig) -> Self {
        let now = Utc::now();
        Self {
            name: name.to_string(),
            config,
            history: TeamHistory::new(),
            created_at: now,
            updated_at: now,
            run_count: 0,
        }
    }
}

/// Manages persistent storage of team snapshots on disk.
pub struct TeamStore {
    /// Directory where team JSON files live (e.g. `~/.limit/teams/`).
    dir: PathBuf,
}

impl TeamStore {
    /// Create a store backed by the given directory.
    ///
    /// The directory is created if it doesn't exist.
    pub fn new(dir: PathBuf) -> Result<Self, std::io::Error> {
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Default store at `~/.limit/teams/`.
    pub fn default_dir() -> Result<Self, std::io::Error> {
        let home = dirs::home_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
        })?;
        Self::new(home.join(".limit").join("teams"))
    }

    /// Path to the JSON file for a named team.
    fn team_path(&self, name: &str) -> PathBuf {
        // Sanitize: replace / and other problematic chars
        let safe_name = name.replace(['/', '\\'], "_");
        self.dir.join(format!("{}.json", safe_name))
    }

    /// Save a team snapshot to disk.
    pub fn save(&self, snapshot: &TeamSnapshot) -> Result<(), std::io::Error> {
        let path = self.team_path(&snapshot.name);
        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, json)
    }

    /// Load a team snapshot from disk.
    pub fn load(&self, name: &str) -> Result<Option<TeamSnapshot>, std::io::Error> {
        let path = self.team_path(name);
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path)?;
        let snapshot: TeamSnapshot = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(snapshot))
    }

    /// Delete a team snapshot from disk.
    pub fn delete(&self, name: &str) -> Result<bool, std::io::Error> {
        let path = self.team_path(name);
        if path.exists() {
            fs::remove_file(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all team names that have saved snapshots.
    pub fn list(&self) -> Result<Vec<String>, std::io::Error> {
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// Check if a team snapshot exists.
    pub fn exists(&self, name: &str) -> bool {
        self.team_path(name).exists()
    }

    /// Path to the store directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team::role::TeamRolesSection;
    use tempfile::TempDir;

    fn test_config() -> TeamConfig {
        TeamConfig {
            num_juniors: 2,
            max_parallel_tasks: 4,
            enable_streaming: true,
            roles: TeamRolesSection::default(),
        }
    }

    #[test]
    fn test_snapshot_new() {
        let snap = TeamSnapshot::new("test-team", test_config());
        assert_eq!(snap.name, "test-team");
        assert_eq!(snap.config.num_juniors, 2);
        assert!(snap.history.is_empty());
        assert_eq!(snap.run_count, 0);
    }

    #[test]
    fn test_store_save_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let store = TeamStore::new(tmp.path().to_path_buf()).unwrap();

        let mut snap = TeamSnapshot::new("my-team", test_config());
        snap.run_count = 3;

        store.save(&snap).unwrap();
        let loaded = store.load("my-team").unwrap().unwrap();

        assert_eq!(loaded.name, "my-team");
        assert_eq!(loaded.run_count, 3);
        assert_eq!(loaded.config.num_juniors, 2);
    }

    #[test]
    fn test_store_load_missing() {
        let tmp = TempDir::new().unwrap();
        let store = TeamStore::new(tmp.path().to_path_buf()).unwrap();

        let loaded = store.load("nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_store_delete() {
        let tmp = TempDir::new().unwrap();
        let store = TeamStore::new(tmp.path().to_path_buf()).unwrap();

        let snap = TeamSnapshot::new("to-delete", test_config());
        store.save(&snap).unwrap();
        assert!(store.exists("to-delete"));

        let deleted = store.delete("to-delete").unwrap();
        assert!(deleted);
        assert!(!store.exists("to-delete"));
    }

    #[test]
    fn test_store_delete_missing() {
        let tmp = TempDir::new().unwrap();
        let store = TeamStore::new(tmp.path().to_path_buf()).unwrap();

        let deleted = store.delete("nonexistent").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_store_list() {
        let tmp = TempDir::new().unwrap();
        let store = TeamStore::new(tmp.path().to_path_buf()).unwrap();

        assert!(store.list().unwrap().is_empty());

        store.save(&TeamSnapshot::new("alpha", test_config())).unwrap();
        store.save(&TeamSnapshot::new("beta", test_config())).unwrap();

        let names = store.list().unwrap();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_store_name_sanitization() {
        let tmp = TempDir::new().unwrap();
        let store = TeamStore::new(tmp.path().to_path_buf()).unwrap();

        store
            .save(&TeamSnapshot::new("team/with/slashes", test_config()))
            .unwrap();
        assert!(store.exists("team/with/slashes"));

        let loaded = store.load("team/with/slashes").unwrap();
        assert!(loaded.is_some());
    }

    #[test]
    fn test_snapshot_serialization() {
        let snap = TeamSnapshot::new("ser-test", test_config());
        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: TeamSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap.name, deserialized.name);
    }
}
