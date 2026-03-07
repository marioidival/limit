#![allow(clippy::all)]
//
// Run with: cargo run --package limit-tui --example progress_demo
//
// This example demonstrates:
// - ProgressBar with animated fill
// - Spinner with rotating characters
// - 10 FPS animation rate
//
// Press 'q' to exit

use crossterm::{
    event::{self, KeyCode},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use limit_tui::components::{ProgressBar, Spinner};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::{
    io,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct DemoState {
    progress_value: f32,
    spinner_frame: usize,
    last_update: Instant,
    direction: f32, // 1.0 for increasing, -1.0 for decreasing
}

impl DemoState {
    fn new() -> Self {
        Self {
            progress_value: 0.0,
            spinner_frame: 0,
            last_update: Instant::now(),
            direction: 1.0,
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);

        // Update at 10 FPS (100ms intervals)
        if elapsed >= Duration::from_millis(100) {
            self.spinner_frame += 1;

            // Update progress bar
            self.progress_value += 0.01 * self.direction;

            // Bounce progress back and forth
            if self.progress_value >= 1.0 {
                self.progress_value = 1.0;
                self.direction = -1.0;
            } else if self.progress_value <= 0.0 {
                self.progress_value = 0.0;
                self.direction = 1.0;
            }

            self.last_update = now;
        }
    }
}

fn main() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create demo state
    let mut state = DemoState::new();

    println!("Starting Progress Indicators Demo...");
    println!("Press 'q' to exit\n");

    // Main loop
    loop {
        // Update state
        state.update();

        // Render UI
        terminal.draw(|f| {
            render_ui(f, &state);
        })?;

        // Check for quit
        if event::poll(Duration::from_millis(50))? {
            if let event::Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // Cleanup
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    println!("\nExiting...");
    Ok(())
}

fn render_ui(f: &mut Frame, state: &DemoState) {
    let size = f.area();

    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(1),  // Spacer
            Constraint::Length(12), // Progress Bar Section
            Constraint::Length(1),  // Spacer
            Constraint::Length(5),  // Spinner Section
            Constraint::Length(1),  // Spacer
            Constraint::Length(3),  // Instructions
        ])
        .split(size);

    // Title
    let title = Paragraph::new(Line::from(vec![Span::styled(
        "Progress Indicators Demo",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(ratatui::style::Modifier::BOLD),
    )]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Progress Bar Section
    let progress_section = Block::default().title("ProgressBar").borders(Borders::ALL);
    f.render_widget(progress_section, chunks[2]);

    let progress_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(chunks[2]);

    // Progress Bar 1
    let mut bar1 = ProgressBar::new("Downloading file...");
    bar1.set_value(state.progress_value);
    bar1.set_width(40);
    bar1.render(progress_chunks[0], f.buffer_mut());

    // Progress Bar 2
    let mut bar2 = ProgressBar::new("Processing data...");
    bar2.set_value(1.0 - state.progress_value);
    bar2.set_width(40);
    bar2.render(progress_chunks[2], f.buffer_mut());

    // Percentage info
    let info = Paragraph::new(Line::from(vec![Span::raw(format!(
        "Progress: {:.0}% - {:.0}%",
        state.progress_value * 100.0,
        (1.0 - state.progress_value) * 100.0
    ))]))
    .alignment(Alignment::Center);
    f.render_widget(info, progress_chunks[1]);

    // Spinner Section
    let spinner_section = Block::default().title("Spinner").borders(Borders::ALL);
    f.render_widget(spinner_section, chunks[4]);

    let spinner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(chunks[4]);

    // Spinner 1
    let mut spinner1 = Spinner::new("Loading configuration...");
    // Tick the spinner to advance to the current frame
    for _ in 0..(state.spinner_frame % 10) {
        spinner1.tick();
    }
    spinner1.render(spinner_chunks[0], f.buffer_mut());

    // Spinner 2
    let mut spinner2 = Spinner::new("Connecting to server...");
    // Tick the spinner to advance to the current frame (offset by 5)
    for _ in 0..((state.spinner_frame + 5) % 10) {
        spinner2.tick();
    }

    // Instructions
    let instructions = Paragraph::new(Line::from(vec![
        Span::styled("Press ", Style::default()),
        Span::styled(
            "'q'",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
        Span::styled(" to exit", Style::default()),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(instructions, chunks[6]);
}
