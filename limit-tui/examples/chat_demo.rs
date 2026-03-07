// Chat View Demo Example
//
// Run with: cargo run --package limit-tui --example chat_demo
//
// Controls:
// - 'q' to exit
// - 'n' to add a new user message
// - 'a' to add a new assistant message
// - 's' to add a new system message
// - Arrow Up to scroll up
// - Arrow Down to scroll down
// - Home to scroll to top
// - End to scroll to bottom

use std::io::{self, Stdout};

use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use limit_tui::components::{ChatView, Message};
use ratatui::{backend::CrosstermBackend, prelude::Widget, Terminal};

type Term = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<(), io::Error> {
    println!("Starting Chat View Demo...");
    println!("Press 'q' to exit\n");

    // Create a chat view with some initial messages
    let mut chat = ChatView::new();

    chat.add_message(Message::system("Chat session started".to_string()));
    chat.add_message(Message::user(
        "Hello! Can you help me with Rust?".to_string(),
    ));
    chat.add_message(Message::assistant(
        "Of course! I'd be happy to help you with Rust. What would you like to know?".to_string(),
    ));
    chat.add_message(Message::user(
        "How do I create a struct with methods?".to_string(),
    ));
    chat.add_message(Message::assistant(
        "In Rust, you can create a struct with associated functions and methods using impl blocks. Here's an example:\n\n\
        struct Person {\n    name: String,\n    age: u32,\n}\n\n\
        impl Person {\n    fn new(name: String, age: u32) -> Self {\n        Self { name, age }\n    }\n\n    fn greet(&self) -> String {\n        format!(\"Hi, I'm {} and I'm {} years old\", self.name, self.age)\n    }\n}"
            .to_string(),
    ));

    let mut message_counter = 0;

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, DisableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        // Draw
        terminal.draw(|f| {
            let size = f.area();
            (&chat).render(size, f.buffer_mut());
        })?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        // Cleanup
                        disable_raw_mode()?;
                        println!("\nChat demo completed!");
                        return Ok(());
                    }
                    KeyCode::Char('n') => {
                        message_counter += 1;
                        chat.add_message(Message::user(format!(
                            "New user message #{}",
                            message_counter
                        )));
                    }
                    KeyCode::Char('a') => {
                        message_counter += 1;
                        chat.add_message(Message::assistant(format!(
                            "New assistant message #{}",
                            message_counter
                        )));
                    }
                    KeyCode::Char('s') => {
                        message_counter += 1;
                        chat.add_message(Message::system(format!(
                            "System notification #{}",
                            message_counter
                        )));
                    }
                    KeyCode::Up => {
                        chat.scroll_up();
                    }
                    KeyCode::Down => {
                        chat.scroll_down();
                    }
                    KeyCode::Home => {
                        chat.scroll_to_top();
                    }
                    KeyCode::End => {
                        chat.scroll_to_bottom();
                    }
                    _ => {}
                }
            }
        }
    }
}
