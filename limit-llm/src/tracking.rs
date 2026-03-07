use crate::error::LlmError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Usage statistics for LLM requests
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub avg_duration_ms: f64,
}

/// SQLite database for tracking LLM usage
pub struct TrackingDb {
    conn: Arc<Mutex<Connection>>,
}

impl TrackingDb {
    /// Initialize tracking database at ~/.limit/tracking.db
    pub fn new() -> Result<Self, LlmError> {
        let mut db_path = dirs::home_dir()
            .ok_or_else(|| LlmError::PersistenceError("Home directory not found".to_string()))?;
        db_path.push(".limit");

        // Create .limit directory if it doesn't exist
        std::fs::create_dir_all(&db_path).map_err(|e| {
            LlmError::PersistenceError(format!("Failed to create .limit directory: {}", e))
        })?;

        db_path.push("tracking.db");

        let conn = Connection::open(&db_path)
            .map_err(|e| LlmError::PersistenceError(format!("Failed to open database: {}", e)))?;

        // Set busy timeout for concurrent access (5 seconds)
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| {
                LlmError::PersistenceError(format!("Failed to set busy timeout: {}", e))
            })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.init_tables()?;

        Ok(db)
    }

    /// Create tables if they don't exist
    fn init_tables(&self) -> Result<(), LlmError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LlmError::PersistenceError(format!("Failed to acquire lock: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cost REAL NOT NULL,
                duration_ms INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| LlmError::PersistenceError(format!("Failed to create table: {}", e)))?;

        // Create index on timestamp for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON requests (timestamp)",
            [],
        )
        .map_err(|e| LlmError::PersistenceError(format!("Failed to create index: {}", e)))?;

        Ok(())
    }

    /// Track a new LLM request
    pub fn track_request(
        &self,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost: f64,
        duration_ms: u64,
    ) -> Result<(), LlmError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LlmError::PersistenceError(format!("Failed to get timestamp: {}", e)))?
            .as_secs() as i64;

        let conn = self
            .conn
            .lock()
            .map_err(|e| LlmError::PersistenceError(format!("Failed to acquire lock: {}", e)))?;

        conn.execute(
            "INSERT INTO requests (timestamp, model, input_tokens, output_tokens, cost, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![timestamp, model, input_tokens as i64, output_tokens as i64, cost, duration_ms as i64],
        )
        .map_err(|e| LlmError::PersistenceError(format!("Failed to insert request: {}", e)))?;

        Ok(())
    }

    /// Get usage statistics for the last N days
    pub fn get_usage_stats(&self, days: u32) -> Result<UsageStats, LlmError> {
        let cutoff_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LlmError::PersistenceError(format!("Failed to get timestamp: {}", e)))?
            .as_secs() as i64
            - (days as i64 * 86400);

        let conn = self
            .conn
            .lock()
            .map_err(|e| LlmError::PersistenceError(format!("Failed to acquire lock: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT 
                COUNT(*) as count,
                SUM(input_tokens + output_tokens) as total_tokens,
                SUM(cost) as total_cost,
                AVG(duration_ms) as avg_duration
             FROM requests
             WHERE timestamp >= ?1",
            )
            .map_err(|e| LlmError::PersistenceError(format!("Failed to prepare query: {}", e)))?;

        let mut rows = stmt
            .query([cutoff_timestamp])
            .map_err(|e| LlmError::PersistenceError(format!("Failed to execute query: {}", e)))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| LlmError::PersistenceError(format!("Failed to read row: {}", e)))?
        {
            let total_requests: i64 = row.get(0).unwrap_or(0);
            let total_tokens: i64 = row.get(1).unwrap_or(0);
            let total_cost: f64 = row.get(2).unwrap_or(0.0);
            let avg_duration_ms: f64 = row.get(3).unwrap_or(0.0);

            Ok(UsageStats {
                total_requests: total_requests.max(0) as u64,
                total_tokens: total_tokens.max(0) as u64,
                total_cost,
                avg_duration_ms,
            })
        } else {
            Ok(UsageStats::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_db() -> Result<TrackingDb, LlmError> {
        let temp_file = NamedTempFile::new().map_err(|e| {
            LlmError::PersistenceError(format!("Failed to create temp file: {}", e))
        })?;

        let path = temp_file.path().to_path_buf();
        // Drop temp file to free the path for SQLite
        drop(temp_file);

        let conn = Connection::open(&path).map_err(|e| {
            LlmError::PersistenceError(format!("Failed to open test database: {}", e))
        })?;

        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| {
                LlmError::PersistenceError(format!("Failed to set busy timeout: {}", e))
            })?;

        let db = TrackingDb {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.init_tables()?;

        Ok(db)
    }

    #[test]
    fn test_create_tables() {
        let db = create_test_db().unwrap();

        // Verify table exists by querying it
        let result: rusqlite::Result<i64> =
            db.conn
                .lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM requests", [], |row| row.get(0));
        assert!(result.is_ok());
    }

    #[test]
    fn test_track_request() {
        let db = create_test_db().unwrap();

        db.track_request("claude-3-5-sonnet", 100, 50, 0.001, 1500)
            .unwrap();
        db.track_request("claude-3-opus", 200, 100, 0.005, 3000)
            .unwrap();

        let count: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM requests", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_usage_stats() {
        let db = create_test_db().unwrap();

        db.track_request("model-1", 100, 50, 0.001, 1500).unwrap();
        db.track_request("model-2", 200, 100, 0.005, 3000).unwrap();
        db.track_request("model-3", 150, 75, 0.002, 2000).unwrap();

        let stats = db.get_usage_stats(7).unwrap();

        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.total_tokens, 675); // (100+50) + (200+100) + (150+75)
        assert!((stats.total_cost - 0.008).abs() < 0.0001);
        assert!((stats.avg_duration_ms - 2166.6666666666665).abs() < 0.0001);
    }

    #[test]
    fn test_empty_usage_stats() {
        let db = create_test_db().unwrap();

        let stats = db.get_usage_stats(7).unwrap();

        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_tokens, 0);
        assert_eq!(stats.total_cost, 0.0);
        assert_eq!(stats.avg_duration_ms, 0.0);
    }

    #[test]
    fn test_concurrent_access() {
        let db = create_test_db().unwrap();

        let db_clone1 = TrackingDb {
            conn: Arc::clone(&db.conn),
        };
        let db_clone2 = TrackingDb {
            conn: Arc::clone(&db.conn),
        };

        let handle1 = std::thread::spawn(move || {
            for i in 0..10 {
                db_clone1
                    .track_request(
                        &format!("model-{}", i),
                        100 + i,
                        50 + i,
                        0.001,
                        1000 + (i as u64) * 100,
                    )
                    .unwrap();
            }
        });

        let handle2 = std::thread::spawn(move || {
            for i in 0..10 {
                db_clone2
                    .track_request(
                        &format!("model-{}", i + 10),
                        100 + i,
                        50 + i,
                        0.001,
                        1000 + (i as u64) * 100,
                    )
                    .unwrap();
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        let stats = db.get_usage_stats(7).unwrap();
        assert_eq!(stats.total_requests, 20);
    }
}
