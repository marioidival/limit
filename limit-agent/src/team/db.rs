//! Team persistence via SQLite.
//!
//! Stores team metadata, execution runs, and event history in
//! `~/.limit/team.db`. Config stays in `config.toml` as the
//! single source of truth for role settings.

use crate::error::AgentError;
use crate::team::history::{EventLevel, TeamEvent};
use crate::team::workflow::TeamResult;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

/// Maximum runs to keep per team.
const MAX_RUNS_PER_TEAM: usize = 5;

/// SQLite-backed persistence for team data.
pub struct TeamDb {
    conn: Mutex<Connection>,
}

impl TeamDb {
    /// Open (or create) the database at the default path `~/.limit/team.db`.
    pub fn open_default() -> Result<Self, AgentError> {
        let home = dirs::home_dir()
            .ok_or_else(|| AgentError::TeamError("home directory not found".into()))?;
        let path = home.join(".limit").join("team.db");
        Self::open(&path)
    }

    /// Open (or create) the database at the given path.
    ///
    /// Parent directories are created automatically.
    pub fn open(path: &Path) -> Result<Self, AgentError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AgentError::TeamError(format!("cannot create db dir: {e}")))?;
        }

        let conn = Connection::open(path)
            .map_err(|e| AgentError::TeamError(format!("cannot open db: {e}")))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| AgentError::TeamError(format!("cannot set pragmas: {e}")))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<(), AgentError> {
        self.conn
            .lock()
            .unwrap()
            .execute_batch(
                "
CREATE TABLE IF NOT EXISTS teams (
    name        TEXT PRIMARY KEY,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS team_runs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    team_name     TEXT NOT NULL REFERENCES teams(name) ON DELETE CASCADE,
    task          TEXT NOT NULL,
    status        TEXT NOT NULL,
    started_at    TEXT NOT NULL,
    finished_at   TEXT,
    duration_ms   INTEGER,
    tokens_input  INTEGER NOT NULL DEFAULT 0,
    tokens_output INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS team_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     INTEGER NOT NULL REFERENCES team_runs(id) ON DELETE CASCADE,
    timestamp  TEXT NOT NULL,
    role       TEXT NOT NULL,
    action     TEXT NOT NULL,
    content    TEXT NOT NULL,
    level      TEXT NOT NULL DEFAULT 'info'
);

CREATE INDEX IF NOT EXISTS idx_runs_team ON team_runs(team_name, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_events_run ON team_events(run_id, timestamp);
",
            )
            .map_err(|e| AgentError::TeamError(format!("schema init failed: {e}")))?;
        Ok(())
    }

    // ── Teams ──────────────────────────────────────────────────────────

    /// Insert a new team. Returns `false` if it already exists.
    pub fn insert_team(&self, name: &str) -> Result<bool, AgentError> {
        let affected = self
            .conn
            .lock()
            .unwrap()
            .execute(
                "INSERT OR IGNORE INTO teams (name, created_at) VALUES (?1, ?2)",
                rusqlite::params![name, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| AgentError::TeamError(format!("insert team: {e}")))?;
        Ok(affected > 0)
    }

    /// Check whether a team exists.
    pub fn team_exists(&self, name: &str) -> Result<bool, AgentError> {
        let exists: bool = self
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM teams WHERE name = ?1)",
                rusqlite::params![name],
                |row| row.get(0),
            )
            .map_err(|e| AgentError::TeamError(format!("team exists: {e}")))?;
        Ok(exists)
    }

    /// List all team names.
    pub fn list_teams(&self) -> Result<Vec<String>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM teams ORDER BY name")
            .map_err(|e| AgentError::TeamError(format!("list teams: {e}")))?;

        let names = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| AgentError::TeamError(format!("list teams query: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(names)
    }

    /// Delete a team and all its runs/events (cascade).
    pub fn delete_team(&self, name: &str) -> Result<bool, AgentError> {
        let affected = self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM teams WHERE name = ?1", rusqlite::params![name])
            .map_err(|e| AgentError::TeamError(format!("delete team: {e}")))?;
        Ok(affected > 0)
    }

    // ── Runs ───────────────────────────────────────────────────────────

    /// Start a new run, returning the run id.
    pub fn insert_run(&self, team_name: &str, task: &str) -> Result<i64, AgentError> {
        self.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO team_runs (team_name, task, status, started_at)
                 VALUES (?1, ?2, 'running', ?3)",
                rusqlite::params![team_name, task, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| AgentError::TeamError(format!("insert run: {e}")))?;
        Ok(self.conn.lock().unwrap().last_insert_rowid())
    }

    /// Finalize a run after execution completes.
    pub fn finish_run(
        &self,
        run_id: i64,
        status: &str,
        duration: Duration,
        tokens_input: u64,
        tokens_output: u64,
    ) -> Result<(), AgentError> {
        self.conn
            .lock()
            .unwrap()
            .execute(
                "UPDATE team_runs SET status = ?1, finished_at = ?2,
                 duration_ms = ?3, tokens_input = ?4, tokens_output = ?5
                 WHERE id = ?6",
                rusqlite::params![
                    status,
                    chrono::Utc::now().to_rfc3339(),
                    duration.as_millis() as i64,
                    tokens_input as i64,
                    tokens_output as i64,
                    run_id,
                ],
            )
            .map_err(|e| AgentError::TeamError(format!("finish run: {e}")))?;
        Ok(())
    }

    /// Persist a full [`TeamResult`] (run + events) for a team atomically.
    pub fn persist_run_result(
        &self,
        team_name: &str,
        task: &str,
        result: &TeamResult,
    ) -> Result<i64, AgentError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| AgentError::TeamError(format!("begin transaction: {e}")))?;

        tx.execute(
            "INSERT INTO team_runs (team_name, task, status, started_at)
             VALUES (?1, ?2, 'running', ?3)",
            rusqlite::params![team_name, task, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| AgentError::TeamError(format!("insert run: {e}")))?;
        let run_id = tx.last_insert_rowid();

        let status = if result.failed_tasks > 0 {
            "failed"
        } else {
            "success"
        };

        tx.execute(
            "UPDATE team_runs SET status = ?1, finished_at = ?2,
             duration_ms = ?3, tokens_input = ?4, tokens_output = ?5
             WHERE id = ?6",
            rusqlite::params![
                status,
                chrono::Utc::now().to_rfc3339(),
                result.duration.as_millis() as i64,
                result.tokens_input as i64,
                result.tokens_output as i64,
                run_id,
            ],
        )
        .map_err(|e| AgentError::TeamError(format!("finish run: {e}")))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO team_events (run_id, timestamp, role, action, content, level)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| AgentError::TeamError(format!("insert events: {e}")))?;

            for ev in &result.events {
                stmt.execute(rusqlite::params![
                    run_id,
                    ev.timestamp.to_rfc3339(),
                    ev.role,
                    ev.action,
                    ev.content,
                    level_to_str(ev.level),
                ])
                .map_err(|e| AgentError::TeamError(format!("insert event: {e}")))?;
            }
        }

        tx.execute(
            "DELETE FROM team_runs WHERE team_name = ?1 AND id NOT IN (
                SELECT id FROM team_runs
                WHERE team_name = ?1
                ORDER BY started_at DESC
                LIMIT ?2
            )",
            rusqlite::params![team_name, MAX_RUNS_PER_TEAM],
        )
        .map_err(|e| AgentError::TeamError(format!("prune runs: {e}")))?;

        tx.commit()
            .map_err(|e| AgentError::TeamError(format!("commit: {e}")))?;

        Ok(run_id)
    }

    /// Persist a failed run (no result, just an error message) atomically.
    pub fn persist_run_error(
        &self,
        team_name: &str,
        task: &str,
        duration: Duration,
        error_msg: &str,
    ) -> Result<i64, AgentError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| AgentError::TeamError(format!("begin transaction: {e}")))?;

        tx.execute(
            "INSERT INTO team_runs (team_name, task, status, started_at)
             VALUES (?1, ?2, 'failed', ?3)",
            rusqlite::params![team_name, task, chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|e| AgentError::TeamError(format!("insert run: {e}")))?;
        let run_id = tx.last_insert_rowid();

        tx.execute(
            "UPDATE team_runs SET finished_at = ?1, duration_ms = ?2
             WHERE id = ?3",
            rusqlite::params![
                chrono::Utc::now().to_rfc3339(),
                duration.as_millis() as i64,
                run_id
            ],
        )
        .map_err(|e| AgentError::TeamError(format!("finish run: {e}")))?;

        tx.execute(
            "INSERT INTO team_events (run_id, timestamp, role, action, content, level)
             VALUES (?1, ?2, 'system', 'error', ?3, 'error')",
            rusqlite::params![run_id, chrono::Utc::now().to_rfc3339(), error_msg],
        )
        .map_err(|e| AgentError::TeamError(format!("insert error event: {e}")))?;

        tx.execute(
            "DELETE FROM team_runs WHERE team_name = ?1 AND id NOT IN (
                SELECT id FROM team_runs WHERE team_name = ?1
                ORDER BY started_at DESC LIMIT ?2
            )",
            rusqlite::params![team_name, MAX_RUNS_PER_TEAM],
        )
        .map_err(|e| AgentError::TeamError(format!("prune runs: {e}")))?;

        tx.commit()
            .map_err(|e| AgentError::TeamError(format!("commit: {e}")))?;
        Ok(run_id)
    }

    // ── Events ─────────────────────────────────────────────────────────

    /// Get events from the most recent run for a team, reconstructed as `TeamEvent`s.
    pub fn get_latest_run_events(
        &self,
        team_name: &str,
    ) -> Result<Option<(Vec<TeamEvent>, RunSummary)>, AgentError> {
        let conn = self.conn.lock().unwrap();

        let run = conn.query_row(
            "SELECT id, task, status, started_at, finished_at, duration_ms,
                    tokens_input, tokens_output
             FROM team_runs
             WHERE team_name = ?1
             ORDER BY started_at DESC
             LIMIT 1",
            rusqlite::params![team_name],
            |row| {
                Ok(RunSummary {
                    run_id: row.get(0)?,
                    task: row.get(1)?,
                    status: row.get(2)?,
                    started_at: row.get(3)?,
                    finished_at: row.get(4)?,
                    duration_ms: row.get(5)?,
                    tokens_input: row.get(6)?,
                    tokens_output: row.get(7)?,
                })
            },
        );

        let summary = match run {
            Ok(s) => s,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(AgentError::TeamError(format!("get latest run: {e}"))),
        };

        let mut stmt = conn
            .prepare(
                "SELECT timestamp, role, action, content, level
                 FROM team_events
                 WHERE run_id = ?1
                 ORDER BY timestamp",
            )
            .map_err(|e| AgentError::TeamError(format!("get events: {e}")))?;

        let events = stmt
            .query_map(rusqlite::params![summary.run_id], |row| {
                let level_str: String = row.get(4)?;
                Ok(TeamEvent {
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(0)?)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    role: row.get(1)?,
                    action: row.get(2)?,
                    content: row.get(3)?,
                    level: str_to_level(&level_str),
                })
            })
            .map_err(|e| AgentError::TeamError(format!("query events: {e}")))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(Some((events, summary)))
    }
}

/// Summary of a persisted run.
pub struct RunSummary {
    pub run_id: i64,
    pub task: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub tokens_input: i64,
    pub tokens_output: i64,
}

fn level_to_str(level: EventLevel) -> &'static str {
    match level {
        EventLevel::Info => "info",
        EventLevel::Warn => "warn",
        EventLevel::Error => "error",
    }
}

fn str_to_level(s: &str) -> EventLevel {
    match s {
        "warn" => EventLevel::Warn,
        "error" => EventLevel::Error,
        _ => EventLevel::Info,
    }
}

/// Get the default database path (`~/.limit/team.db`).
pub fn default_db_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".limit").join("team.db"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_temp_db() -> (TeamDb, TempDir) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        let db = TeamDb::open(&path).unwrap();
        (db, tmp)
    }

    #[test]
    fn test_schema_creation() {
        let (db, _tmp) = open_temp_db();
        let teams: Vec<String> = db.list_teams().unwrap();
        assert!(teams.is_empty());
    }

    #[test]
    fn test_insert_and_list_teams() {
        let (db, _tmp) = open_temp_db();

        assert!(db.insert_team("alpha").unwrap());
        assert!(db.insert_team("beta").unwrap());
        assert!(!db.insert_team("alpha").unwrap());

        let teams = db.list_teams().unwrap();
        assert_eq!(teams, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_team_exists() {
        let (db, _tmp) = open_temp_db();

        assert!(!db.team_exists("missing").unwrap());
        db.insert_team("exists").unwrap();
        assert!(db.team_exists("exists").unwrap());
    }

    #[test]
    fn test_delete_team() {
        let (db, _tmp) = open_temp_db();

        db.insert_team("to-delete").unwrap();
        assert!(db.delete_team("to-delete").unwrap());
        assert!(!db.team_exists("to-delete").unwrap());
        assert!(!db.delete_team("ghost").unwrap());
    }

    #[test]
    fn test_insert_and_finish_run() {
        let (db, _tmp) = open_temp_db();

        db.insert_team("myteam").unwrap();
        let run_id = db.insert_run("myteam", "fix bug").unwrap();
        assert!(run_id > 0);

        db.finish_run(run_id, "success", Duration::from_secs(10), 100, 50)
            .unwrap();

        let (events, summary) = db.get_latest_run_events("myteam").unwrap().unwrap();
        assert_eq!(summary.status, "success");
        assert_eq!(summary.task, "fix bug");
        assert_eq!(summary.duration_ms, Some(10_000));
        assert_eq!(summary.tokens_input, 100);
        assert_eq!(summary.tokens_output, 50);
        assert!(events.is_empty());
    }

    #[test]
    fn test_persist_run_result() {
        let (db, _tmp) = open_temp_db();

        db.insert_team("myteam").unwrap();

        let result = TeamResult {
            solution: "done".into(),
            duration: Duration::from_secs(5),
            events: vec![
                TeamEvent {
                    timestamp: chrono::Utc::now(),
                    role: "PM".into(),
                    action: "analysis".into(),
                    content: "analyzed".into(),
                    level: EventLevel::Info,
                },
                TeamEvent {
                    timestamp: chrono::Utc::now(),
                    role: "Jr".into(),
                    action: "execution".into(),
                    content: "failed".into(),
                    level: EventLevel::Error,
                },
            ],
            total_retries: 0,
            failed_tasks: 1,
            total_tasks: 2,
            files_modified: vec!["src/main.rs".into()],
            tokens_input: 200,
            tokens_output: 100,
        };

        let run_id = db.persist_run_result("myteam", "fix bug", &result).unwrap();
        assert!(run_id > 0);

        let (events, summary) = db.get_latest_run_events("myteam").unwrap().unwrap();
        assert_eq!(summary.status, "failed");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].role, "PM");
        assert_eq!(events[1].level, EventLevel::Error);
    }

    #[test]
    fn test_prune_old_runs() {
        let (db, _tmp) = open_temp_db();

        db.insert_team("myteam").unwrap();

        for i in 0..7 {
            let run_id = db.insert_run("myteam", &format!("task {i}")).unwrap();
            db.finish_run(run_id, "success", Duration::from_secs(1), 0, 0)
                .unwrap();
        }

        let (_, summary) = db.get_latest_run_events("myteam").unwrap().unwrap();
        assert_eq!(summary.task, "task 6");
    }

    #[test]
    fn test_cascade_delete() {
        let (db, _tmp) = open_temp_db();

        db.insert_team("myteam").unwrap();

        let result = TeamResult {
            solution: "done".into(),
            duration: Duration::from_secs(1),
            events: vec![TeamEvent {
                timestamp: chrono::Utc::now(),
                role: "PM".into(),
                action: "test".into(),
                content: "data".into(),
                level: EventLevel::Info,
            }],
            total_retries: 0,
            failed_tasks: 0,
            total_tasks: 0,
            files_modified: vec![],
            tokens_input: 0,
            tokens_output: 0,
        };
        db.persist_run_result("myteam", "task", &result).unwrap();

        db.delete_team("myteam").unwrap();
        assert!(db.get_latest_run_events("myteam").unwrap().is_none());
    }

    #[test]
    fn test_persist_run_error() {
        let (db, _tmp) = open_temp_db();

        db.insert_team("myteam").unwrap();
        db.persist_run_error("myteam", "task", Duration::from_secs(3), "oom")
            .unwrap();

        let (events, summary) = db.get_latest_run_events("myteam").unwrap().unwrap();
        assert_eq!(summary.status, "failed");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, EventLevel::Error);
        assert_eq!(events[0].role, "system");
        assert!(events[0].content.contains("oom"));
    }

    #[test]
    fn test_default_db_path() {
        let path = default_db_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains(".limit"));
        assert!(p.to_string_lossy().contains("team.db"));
    }
}
