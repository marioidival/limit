mod agent_bridge;
mod error;
mod repl;
mod render;
mod session;
mod tools;
mod tui_bridge;

fn main() {
    if let Err(e) = repl::Repl::new().and_then(|mut repl| repl.run()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
