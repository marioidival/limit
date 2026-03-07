mod error;
mod repl;
mod tools;

fn main() {
    if let Err(e) = repl::Repl::new().and_then(|mut repl| repl.run()) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
