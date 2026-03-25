use clap::Parser;
use limit_cli::CliError;

#[derive(Parser)]
#[command(name = "limit", about = "AI-powered code agent with TUI", version)]
struct Args;

fn main() {
    limit_cli::init_logging();

    if let Err(e) = run_tui() {
        tracing::error!("Application error: {}", e);
        std::process::exit(1);
    }
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
