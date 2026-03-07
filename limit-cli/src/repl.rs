use crate::error::CliError;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use std::io::Write;

pub struct Repl {
    reader: Editor<(), DefaultHistory>,
}

impl Repl {
    pub fn new() -> Result<Self, CliError> {
        let reader = Editor::<(), DefaultHistory>::new()?;
        Ok(Self { reader })
    }

    pub fn run(&mut self) -> Result<(), CliError> {
        println!("limit-cli - Interactive REPL");
        println!("Type /help for available commands\n");

        loop {
            let line = self.reader.readline("limit> ")?;

            self.process_line(line)?;
        }
    }

    fn process_line(&mut self, line: String) -> Result<(), CliError> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Ok(());
        }

        match trimmed {
            "/exit" => {
                println!("Goodbye!");
                std::process::exit(0);
            }
            "/clear" => self.clear_screen()?,
            "/help" => self.show_help(),
            _ => {
                self.reader.add_history_entry(line.as_str())?;
                println!("You said: {}", line);
            }
        }

        Ok(())
    }

    fn clear_screen(&self) -> Result<(), CliError> {
        print!("\x1B[2J\x1B[1;1H");
        std::io::stdout().flush()?;
        Ok(())
    }

    fn show_help(&self) {
        println!("Available commands:");
        println!("  /help   - Show this help message");
        println!("  /clear  - Clear the screen");
        println!("  /exit   - Exit the REPL");
        println!("\nAny other input will be echoed back.");
    }
}
