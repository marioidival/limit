use std::env;
use std::fs::OpenOptions;
use std::path::PathBuf;
use tracing_subscriber::{fmt, EnvFilter};

#[allow(dead_code)]
pub fn init_logging() {
    // Create log file in ~/.limit/logs/
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let log_dir = PathBuf::from(home).join(".limit").join("logs");

    let _ = std::fs::create_dir_all(&log_dir);

    let log_path = log_dir.join("tui.log");

    // Open file in append mode
    let file = OpenOptions::new().create(true).append(true).open(&log_path);

    match file {
        Ok(f) => {
            let filter = EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    EnvFilter::new("debug,limit_llm=debug,limit_agent=debug,limit_cli=debug,reqwest=warn,hyper=warn")
                });

            fmt()
                .with_writer(f)
                .with_target(true)
                .with_ansi(false)
                .with_env_filter(filter)
                .init();

            tracing::info!("=== Logging initialized to {:?} ===", log_path);
        }
        Err(e) => {
            eprintln!("Warning: Could not open log file {:?}: {}", log_path, e);
            // Fallback to stderr
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

            fmt().with_env_filter(filter).init();
        }
    }
}
