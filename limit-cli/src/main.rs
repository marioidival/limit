mod agent_bridge;
mod system_prompt;

mod error;
mod logging;
mod render;
mod repl;
mod session;
mod tools;
mod tui_bridge;

fn main() {
    limit_cli::init_logging();
    if let Err(e) = repl::Repl::new().and_then(|mut repl| repl.run()) {
        tracing::error!("Application error: {}", e);
        std::process::exit(1);
    }
}
