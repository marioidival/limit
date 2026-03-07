// Diff View Demo

use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use limit_tui::components::DiffView;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    prelude::Widget,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::io;

fn main() -> io::Result<()> {
    // Sample diff to display
    let diff_text = r#"--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@
 fn main() {
     println!("Hello, World!");
+    println!("This is a new line!");
     let x = 42;
     println!("x = {}", x);
+    let y = x * 2;
+    println!("y = {}", y);
 }

@@ -10,8 +11,8 @@
 fn helper() {
-    let old_var = 100;
-    println!("Old value: {}", old_var);
+    let new_var = 200;
+    println!("New value: {}", new_var);
     return;
 }

--- a/Cargo.toml
+++ b/Cargo.toml
@@ -5,6 +5,7 @@
 edition = "2021"
 
 [dependencies]
+ratatui = "0.29"
 serde = "1.0"
 tokio = "1.0"
"#;

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, DisableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create diff view
    let mut diff_view = DiffView::from_diff(diff_text);

    // Help text
    let help_text = ["Diff View Demo - Keyboard Controls:",
        "↑/k - Scroll Up",
        "↓/j - Scroll Down",
        "Page Up - Previous Page",
        "Page Down - Next Page",
        "Home - Go to Top",
        "End - Go to Bottom",
        "q/ESC - Quit"];

    // Event loop
    loop {
        terminal.draw(|f| {
            let size = f.area();

            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Length(7), Constraint::Min(0)].as_ref())
                .split(size);

            // Render help text
            let help_paragraph = Paragraph::new(help_text.join("\n"))
                .wrap(Wrap { trim: true })
                .block(Block::default().title("Help").borders(Borders::ALL));
            help_paragraph.render(chunks[0], f.buffer_mut());

            // Render diff view
            let diff_block = Block::default().title("Unified Diff").borders(Borders::ALL);
            let inner_area = diff_block.inner(chunks[1]);
            diff_block.render(chunks[1], f.buffer_mut());
            (&diff_view).render(inner_area, f.buffer_mut());
        })?;

        // Poll for events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up | KeyCode::Char('k') => diff_view.scroll_up(),
                    KeyCode::Down | KeyCode::Char('j') => diff_view.scroll_down(),
                    KeyCode::PageUp => {
                        let page_size = terminal.size()?.height.saturating_sub(10) as usize;
                        diff_view.page_up(page_size);
                    }
                    KeyCode::PageDown => {
                        let page_size = terminal.size()?.height.saturating_sub(10) as usize;
                        diff_view.page_down(page_size);
                    }
                    KeyCode::Home => diff_view.scroll_to_top(),
                    KeyCode::End => diff_view.scroll_to_bottom(),
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    println!("\nDiff view demo completed!");
    Ok(())
}
