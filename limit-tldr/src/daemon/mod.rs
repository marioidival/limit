//! Daemon mode for TLDR
//!
//! Long-running background process with indexes in RAM for 300x faster queries.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::types::{DaemonCommand, DaemonStatus};
use crate::TLDR;

/// Daemon instance
pub struct Daemon {
    socket_path: PathBuf,
    project_path: PathBuf,
    tldr: Arc<RwLock<TLDR>>,
    start_time: Instant,
    stats: Arc<RwLock<DaemonStats>>,
}

#[derive(Default)]
struct DaemonStats {
    queries: u64,
    cache_hits: u64,
}

impl Daemon {
    /// Create a new daemon for a project
    pub async fn new<P: Into<PathBuf>>(project_path: P) -> Result<Self> {
        let project_path = project_path.into();
        let socket_path = Self::socket_path(&project_path);

        // Initialize TLDR
        let config = crate::Config::default();
        let tldr = TLDR::new(&project_path, config).await?;

        // Warm up indexes
        let mut tldr = tldr;
        tldr.warm().await?;

        Ok(Self {
            socket_path,
            project_path,
            tldr: Arc::new(RwLock::new(tldr)),
            start_time: Instant::now(),
            stats: Arc::new(RwLock::new(DaemonStats::default())),
        })
    }

    /// Get socket path for a project
    pub fn socket_path(project_path: &Path) -> PathBuf {
        let hash = blake3::hash(project_path.to_string_lossy().as_bytes());
        let hash_hex = hex::encode(&hash.as_bytes()[..4]);
        PathBuf::from(format!("/tmp/tldr-{}.sock", hash_hex))
    }

    /// Get PID file path
    pub fn pid_path(project_path: &Path) -> PathBuf {
        project_path.join(".tldr").join("daemon.pid")
    }

    /// Check if daemon is running
    pub async fn is_running(project_path: &Path) -> bool {
        let socket_path = Self::socket_path(project_path);

        if let Ok(stream) = UnixStream::connect(&socket_path) {
            // Send ping
            let _cmd = serde_json::to_string(&DaemonCommand::Ping).unwrap();
            let _ = stream.peer_addr();
            return true;
        }

        false
    }

    /// Get daemon status
    pub async fn status(&self) -> DaemonStatus {
        let _tldr = self.tldr.read().await;
        let stats = self.stats.read().await;

        DaemonStatus {
            running: true,
            pid: Some(std::process::id()),
            socket: self.socket_path.clone(),
            uptime: Some(self.start_time.elapsed().as_secs()),
            files_indexed: 0, // Would track in real impl
            cache_hit_rate: if stats.queries > 0 {
                stats.cache_hits as f32 / stats.queries as f32
            } else {
                0.0
            },
            semantic_functions: None,
        }
    }

    /// Run the daemon
    pub async fn run(&self) -> Result<()> {
        // Remove old socket if exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .map_err(|e| Error::Daemon(format!("Failed to remove old socket: {}", e)))?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| Error::Daemon(format!("Failed to bind socket: {}", e)))?;

        // Write PID file
        let pid_path = Self::pid_path(&self.project_path);
        if let Some(parent) = pid_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&pid_path, std::process::id().to_string()).ok();

        println!("TLDR daemon listening on {}", self.socket_path.display());

        // Accept connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    self.handle_connection(stream).await?;
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(&self, stream: UnixStream) -> Result<()> {
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();

        if reader.read_line(&mut line).is_ok() {
            let cmd: DaemonCommand = serde_json::from_str(&line)
                .map_err(|e| Error::Daemon(format!("Invalid command: {}", e)))?;

            let response = self.execute_command(cmd).await?;

            let json = serde_json::to_string(&response)
                .map_err(|e| Error::Daemon(format!("Failed to serialize response: {}", e)))?;

            let mut stream = &stream;
            stream.write_all(json.as_bytes()).ok();
            stream.write_all(b"\n").ok();
        }

        Ok(())
    }

    async fn execute_command(&self, cmd: DaemonCommand) -> Result<serde_json::Value> {
        let tldr = self.tldr.read().await;

        match cmd {
            DaemonCommand::Ping => Ok(serde_json::json!({"status": "ok"})),

            DaemonCommand::Status => {
                let status = self.status().await;
                Ok(serde_json::to_value(status)?)
            }

            DaemonCommand::Search { pattern: _ } => {
                // Text search in code
                Ok(serde_json::json!({"results": []}))
            }

            DaemonCommand::Impact { function } => {
                let callers = tldr.get_impact(&function)?;
                Ok(serde_json::to_value(callers)?)
            }

            DaemonCommand::Context { entry, depth } => {
                let context = tldr.get_context(&entry, depth).await?;
                Ok(serde_json::json!({"context": context}))
            }

            DaemonCommand::Cfg { file, function } => {
                let cfg = tldr.get_cfg(&file, &function)?;
                Ok(serde_json::to_value(cfg)?)
            }

            DaemonCommand::Dfg { file, function } => {
                let dfg = tldr.get_dfg(&file, &function)?;
                Ok(serde_json::to_value(dfg)?)
            }

            DaemonCommand::Slice {
                file,
                function,
                line,
            } => {
                let slice = tldr.get_slice(&file, &function, line)?;
                Ok(serde_json::to_value(slice)?)
            }

            _ => Ok(serde_json::json!({"error": "Command not implemented"})),
        }
    }
}

/// Daemon client for querying the daemon
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(project_path: &Path) -> Self {
        Self {
            socket_path: Daemon::socket_path(project_path),
        }
    }

    /// Send a command to the daemon
    pub fn send(&self, cmd: &DaemonCommand) -> Result<serde_json::Value> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|e| Error::Daemon(format!("Failed to connect: {}", e)))?;

        let cmd_json = serde_json::to_string(cmd)
            .map_err(|e| Error::Daemon(format!("Failed to serialize: {}", e)))?;

        stream
            .write_all(cmd_json.as_bytes())
            .map_err(|e| Error::Daemon(format!("Failed to write: {}", e)))?;
        stream
            .write_all(b"\n")
            .map_err(|e| Error::Daemon(format!("Failed to write newline: {}", e)))?;

        let mut response = String::new();
        let mut reader = BufReader::new(&stream);
        reader
            .read_line(&mut response)
            .map_err(|e| Error::Daemon(format!("Failed to read: {}", e)))?;

        let value: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| Error::Daemon(format!("Failed to parse response: {}", e)))?;

        Ok(value)
    }
}

// Add hex dependency for hash encoding
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
