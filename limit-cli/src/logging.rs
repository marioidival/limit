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
            let default_level = if cfg!(debug_assertions) {
                "debug,limit_llm=debug,limit_agent=debug,limit_cli=debug,reqwest=warn,hyper=warn,h2=warn,ignore=warn,globset=warn,rustls=warn"
            } else {
                "warn,limit_llm=warn,limit_agent=warn,limit_cli=warn,reqwest=warn,hyper=warn,h2=warn,ignore=warn,globset=warn,rustls=warn"
            };
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

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
            let default_level = if cfg!(debug_assertions) {
                "debug"
            } else {
                "warn"
            };
            let filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

            fmt().with_env_filter(filter).init();
        }
    }
}
