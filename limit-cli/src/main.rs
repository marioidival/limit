use clap::Parser;
use limit_cli::CliError;

#[derive(Parser)]
#[command(name = "limit", about = "AI-powered code agent with TUI", version)]
struct Args {
    /// Use text-based REPL instead of TUI
    #[arg(long)]
    no_tui: bool,
}

fn main() {
    limit_cli::init_logging();
    let args = Args::parse();

    let result = if args.no_tui { run_repl() } else { run_tui() };

    if let Err(e) = result {
        tracing::error!("Application error: {}", e);
        std::process::exit(1);
    }
}

fn run_repl() -> Result<(), CliError> {
    // For now, just error - REPL is not the main focus
    Err(CliError::ConfigError(
        "REPL mode not yet implemented in this version".to_string(),
    ))
}

fn run_tui() -> Result<(), CliError> {
    use limit_cli::{AgentBridge, TuiApp, TuiBridge};
    use tokio::sync::mpsc;

    let config = limit_llm::Config::load().map_err(|e| CliError::ConfigError(e.to_string()))?;

    let (tx, rx) = mpsc::unbounded_channel();
    let mut bridge = AgentBridge::new(config)?;
    bridge.set_event_tx(tx);

    let tui_bridge = TuiBridge::new(bridge, rx)?;
    let mut app = TuiApp::new(tui_bridge)?;
    app.run()
}
