#![allow(clippy::all)]
//
// Run with: cargo run --package limit-tui --example prompt_demo
//
// This demo shows both InputPrompt and SelectPrompt components

use crossterm::{
    event::{self, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use limit_tui::components::{InputPrompt, InputResult, SelectPrompt, SelectResult};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Paragraph, Wrap},
    Terminal,
};
use std::io::{self};

/// Demo state
struct DemoState {
    mode: DemoMode,
    input_prompt: InputPrompt,
    select_prompt: SelectPrompt,
    result: Option<String>,
}

/// Demo mode
enum DemoMode {
    /// Displaying instructions
    Instructions,
    /// Input prompt active
    Input,
    /// Select prompt active
    Select,
    /// Displaying result
    Result,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create demo state
    let mut state = DemoState {
        mode: DemoMode::Instructions,
        input_prompt: InputPrompt::new("Enter your name:"),
        select_prompt: SelectPrompt::new(
            "Choose a color:",
            vec![
                "Red".to_string(),
                "Green".to_string(),
                "Blue".to_string(),
                "Yellow".to_string(),
                "Purple".to_string(),
            ],
        ),
        result: None,
    };

    // Main loop
    loop {
        terminal.draw(|f| {
            let size = f.area();

            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(10),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(size);

            // Title
            let title = Paragraph::new(Text::styled(
                "Prompt Demo - limit-tui",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ))
            .alignment(Alignment::Center);
            f.render_widget(title, chunks[0]);

            // Main content based on mode
            match state.mode {
                DemoMode::Instructions => {
                    render_instructions(f, chunks[1]);
                }
                DemoMode::Input => {
                    render_input(f, chunks[1], &state.input_prompt);
                }
                DemoMode::Select => {
                    render_select(f, chunks[1], &state.select_prompt);
                }
                DemoMode::Result => {
                    render_result(f, chunks[1], &state.result);
                }
            }

            // Footer
            let footer = match state.mode {
                DemoMode::Instructions => Paragraph::new(Text::styled(
                    "Press '1' for Input Demo | Press '2' for Select Demo | Press 'q' to quit",
                    Style::default().fg(Color::Gray),
                )),
                DemoMode::Input => Paragraph::new(Text::styled(
                    "Type text | Enter: Submit | Esc: Cancel | 'r': Return to menu",
                    Style::default().fg(Color::Gray),
                )),
                DemoMode::Select => Paragraph::new(Text::styled(
                    "↑/↓: Navigate | Enter: Select | Esc: Cancel | 'r': Return to menu",
                    Style::default().fg(Color::Gray),
                )),
                DemoMode::Result => Paragraph::new(Text::styled(
                    "Press 'r' to return to menu | 'q' to quit",
                    Style::default().fg(Color::Gray),
                )),
            };
            f.render_widget(footer, chunks[2]);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let event::Event::Key(key) = event::read()? {
                if handle_key(&mut state, key) {
                    break;
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Render instructions screen
fn render_instructions(f: &mut ratatui::Frame, area: Rect) {
    let instructions = vec![
        "This demo showcases the prompt components in limit-tui.",
        "",
        "Press '1' to try the Input Prompt:",
        "  - Text input with cursor",
        "  - Placeholder text",
        "  - Backspace and navigation",
        "",
        "Press '2' to try the Select Prompt:",
        "  - List of options",
        "  - Arrow key navigation",
        "  - Selection indicator",
        "",
        "Press 'q' to quit the demo.",
    ];

    let text = Text::from(
        instructions
            .iter()
            .map(|s| ratatui::text::Line::from(s.to_string()))
            .collect::<Vec<_>>(),
    );

    let paragraph = Paragraph::new(text)
        .block(Block::bordered().title("Instructions"))
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}

/// Render input prompt
fn render_input(f: &mut ratatui::Frame, area: Rect, prompt: &InputPrompt) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(1)].as_ref())
        .split(area);

    // Input prompt
    prompt.render(chunks[0], f.buffer_mut());

    // Status
    let status = Paragraph::new(Text::styled(
        format!("Text: {} | Cursor: {}", prompt.text(), prompt.cursor_pos()),
        Style::default().fg(Color::Gray),
    ));
    f.render_widget(status, chunks[1]);
}

/// Render select prompt
fn render_select(f: &mut ratatui::Frame, area: Rect, prompt: &SelectPrompt) {
    prompt.render(area, f.buffer_mut());
}

/// Render result screen
fn render_result(f: &mut ratatui::Frame, area: Rect, result: &Option<String>) {
    let text = match result {
        Some(msg) => Text::styled(
            msg,
            Style::default()
                .fg(Color::Green)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        None => Text::styled("No result", Style::default().fg(Color::Red)),
    };

    let paragraph = Paragraph::new(text)
        .block(Block::bordered().title("Result"))
        .alignment(Alignment::Center);

    f.render_widget(paragraph, area);
}

/// Handle keyboard input
/// Returns true to exit the program
fn handle_key(state: &mut DemoState, key: KeyEvent) -> bool {
    match state.mode {
        DemoMode::Instructions => {
            match key.code {
                KeyCode::Char('1') => {
                    // Switch to input mode
                    state.mode = DemoMode::Input;
                    state.input_prompt = InputPrompt::new("Enter your name:");
                }
                KeyCode::Char('2') => {
                    // Switch to select mode
                    state.mode = DemoMode::Select;
                    state.select_prompt = SelectPrompt::new(
                        "Choose a color:",
                        vec![
                            "Red".to_string(),
                            "Green".to_string(),
                            "Blue".to_string(),
                            "Yellow".to_string(),
                            "Purple".to_string(),
                        ],
                    );
                }
                KeyCode::Char('q') => return true,
                _ => {}
            }
        }
        DemoMode::Input => {
            match key.code {
                KeyCode::Char('r') => {
                    // Return to menu
                    state.mode = DemoMode::Instructions;
                }
                KeyCode::Char('q') => return true,
                _ => {
                    // Handle input prompt
                    match state.input_prompt.handle_key(key) {
                        InputResult::Submitted(text) => {
                            state.result = Some(format!("Input submitted: {}", text));
                            state.mode = DemoMode::Result;
                        }
                        InputResult::Cancelled => {
                            state.result = Some("Input cancelled".to_string());
                            state.mode = DemoMode::Result;
                        }
                        InputResult::None => {}
                    }
                }
            }
        }
        DemoMode::Select => {
            match key.code {
                KeyCode::Char('r') => {
                    // Return to menu
                    state.mode = DemoMode::Instructions;
                }
                KeyCode::Char('q') => return true,
                _ => {
                    // Handle select prompt
                    match state.select_prompt.handle_key(key) {
                        SelectResult::Selected(index) => {
                            if let Some(text) = state.select_prompt.selected_text() {
                                state.result =
                                    Some(format!("Selected option {} ({})", index, text));
                            } else {
                                state.result = Some(format!("Selected option {}", index));
                            }
                            state.mode = DemoMode::Result;
                        }
                        SelectResult::Cancelled => {
                            state.result = Some("Selection cancelled".to_string());
                            state.mode = DemoMode::Result;
                        }
                        SelectResult::None => {}
                    }
                }
            }
        }
        DemoMode::Result => {
            match key.code {
                KeyCode::Char('r') => {
                    // Return to menu
                    state.mode = DemoMode::Instructions;
                }
                KeyCode::Char('q') => return true,
                _ => {}
            }
        }
    }

    false
}
