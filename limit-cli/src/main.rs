mod agent_bridge;
mod clipboard;
mod system_prompt;

mod error;
mod logging;
mod render;
mod repl;
mod session;
mod syntax;
mod tools;
mod tui_bridge;

use clap::Parser;
use error::CliError;

#[derive(Parser)]
#[command(name = "limit", about = "AI-powered code agent with TUI")]
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
    repl::Repl::new().and_then(|mut r| r.run())
}

fn run_tui() -> Result<(), CliError> {
    use agent_bridge::AgentBridge;
    use tokio::sync::mpsc;
    use tui_bridge::{TuiApp, TuiBridge};

    let config = limit_llm::Config::load().map_err(|e| CliError::ConfigError(e.to_string()))?;

    let (tx, rx) = mpsc::unbounded_channel();
    let mut bridge = AgentBridge::new(config)?;
    bridge.set_event_tx(tx);

    let tui_bridge = TuiBridge::new(bridge, rx)?;
    let mut app = TuiApp::new(tui_bridge)?;
    app.run()
}
