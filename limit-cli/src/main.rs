mod error;
mod repl;

fn main() {
    if let Err(e) = repl::Repl::new().and_then(|mut repl| repl.run()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
