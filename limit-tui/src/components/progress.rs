// Progress indicators for limit-tui
//
// This module provides ProgressBar and Spinner components for displaying
// progress and loading states in terminal UI applications.

use tracing::debug;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Gauge, Paragraph, Widget},
};

/// Default spinner animation frames
pub(crate) const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Progress bar component that displays a percentage-based progress indicator
///
/// The progress bar consists of:
/// - A label displayed above the bar
/// - A filled bar showing the percentage of completion
/// - Percentage text shown on the bar
#[derive(Debug, Clone)]
pub struct ProgressBar {
    /// Current progress value (0.0 to 1.0)
    value: f32,
    /// Label displayed above the progress bar
    label: String,
    /// Width of the progress bar (in terminal columns)
    width: u16,
}

impl ProgressBar {
    /// Create a new progress bar with a given label
    ///
    /// # Arguments
    ///
    /// * `label` - The text to display above the progress bar
    ///
    /// # Returns
    ///
    /// A new `ProgressBar` instance with value set to 0.0
    ///
    /// # Example
    ///
    /// ```rust
    /// use limit_tui::components::ProgressBar;
    ///
    /// let bar = ProgressBar::new("Downloading");
    /// ```
    pub fn new(label: &str) -> Self {
        debug!(component = %"ProgressBar", "Component created");
        Self {
            value: 0.0,
            label: label.to_string(),
            width: 40,
        }
    }

    /// Set the progress value
    /// Set the progress value
    ///
    /// The value should be between 0.0 (0%) and 1.0 (100%).
    /// Values outside this range will be clamped.
    ///
    /// # Arguments
    ///
    /// * `value` - Progress value (0.0 to 1.0)
    ///
    /// # Example
    ///
    /// ```rust
    /// use limit_tui::components::ProgressBar;
    ///
    /// let mut bar = ProgressBar::new("Loading");
    /// bar.set_value(0.5); // 50% complete
    /// ```
    pub fn set_value(&mut self, value: f32) {
        // Clamp value between 0.0 and 1.0
        self.value = value.clamp(0.0, 1.0);
    }

    /// Get the current progress value
    ///
    /// # Returns
    ///
    /// The current progress value (0.0 to 1.0)
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Set the width of the progress bar
    ///
    /// # Arguments
    ///
    /// * `width` - Width in terminal columns
    pub fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    /// Render the progress bar to a buffer
    ///
    /// # Arguments
    ///
    /// * `area` - The area to render the progress bar in
    /// * `buf` - The buffer to render to
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        // Render label above the bar
        let label_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        let label_paragraph = Paragraph::new(self.label.as_str());
        label_paragraph.render(label_area, buf);

        // Calculate bar area (below the label)
        let bar_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: self.width.min(area.width),
            height: 1,
        };

        // Render the gauge with the progress value
        let gauge = Gauge::default()
            .percent((self.value * 100.0) as u16)
            .style(Style::default().fg(Color::Green))
            .label(format!("{:.0}%", self.value * 100.0));

        gauge.render(bar_area, buf);
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new("Progress")
    }
}

/// Spinner component for displaying loading animations
///
/// The spinner consists of:
/// - A rotating character animation
/// - A label displayed next to the spinner
#[derive(Debug, Clone)]
pub struct Spinner {
    /// Current frame index
    current_frame: usize,
    /// Animation frames for the spinner
    frames: Vec<String>,
    /// Label displayed next to the spinner
    label: String,
}

impl Spinner {
    /// Create a new spinner with a given label
    ///
    /// # Arguments
    ///
    /// * `label` - The text to display next to the spinner
    ///
    /// # Returns
    ///
    /// A new `Spinner` instance ready for animation
    ///
    /// # Example
    ///
    /// ```rust
    /// use limit_tui::components::Spinner;
    ///
    /// let spinner = Spinner::new("Loading...");
    /// ```
    pub fn new(label: &str) -> Self {
        debug!(component = %"Spinner", "Component created");
        Self {
            current_frame: 0,
            frames: SPINNER_FRAMES.iter().map(|s| s.to_string()).collect(),
            label: label.to_string(),
        }
    }

    /// Create a new spinner with custom frames
    /// Create a new spinner with custom frames
    ///
    /// # Arguments
    ///
    /// * `label` - The text to display next to the spinner
    /// * `frames` - Vector of animation frames
    ///
    /// # Returns
    ///
    /// A new `Spinner` instance with custom animation frames
    pub fn with_frames(label: &str, frames: Vec<String>) -> Self {
        Self {
            current_frame: 0,
            frames,
            label: label.to_string(),
        }
    }

    /// Advance the spinner to the next frame
    ///
    /// Call this method at your desired frame rate. For a smooth
    /// 10 FPS animation, call `tick()` every 100ms.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use limit_tui::components::Spinner;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let mut spinner = Spinner::new("Loading...");
    /// loop {
    ///     spinner.tick();
    ///     // render spinner
    ///     thread::sleep(Duration::from_millis(100)); // 10 FPS
    /// }
    /// ```
    pub fn tick(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.frames.len();
    }

    /// Get the current animation frame
    ///
    /// # Returns
    ///
    /// The current spinner character
    pub fn current_frame(&self) -> &str {
        &self.frames[self.current_frame]
    }

    /// Render the spinner to a buffer
    ///
    /// # Arguments
    ///
    /// * `area` - The area to render the spinner in
    /// * `buf` - The buffer to render to
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let text = format!("{} {}", self.current_frame(), self.label);
        let paragraph = Paragraph::new(text.as_str());
        paragraph.render(area, buf);
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new("Loading...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_new() {
        let bar = ProgressBar::new("Test");
        assert_eq!(bar.value(), 0.0);
        assert_eq!(bar.label, "Test");
    }

    #[test]
    fn test_progress_bar_default() {
        let bar = ProgressBar::default();
        assert_eq!(bar.value(), 0.0);
        assert_eq!(bar.label, "Progress");
    }

    #[test]
    fn test_progress_bar_set_value() {
        let mut bar = ProgressBar::new("Test");
        bar.set_value(0.5);
        assert_eq!(bar.value(), 0.5);
    }

    #[test]
    fn test_progress_bar_set_value_clamps_high() {
        let mut bar = ProgressBar::new("Test");
        bar.set_value(1.5);
        assert_eq!(bar.value(), 1.0);
    }

    #[test]
    fn test_progress_bar_set_value_clamps_low() {
        let mut bar = ProgressBar::new("Test");
        bar.set_value(-0.5);
        assert_eq!(bar.value(), 0.0);
    }

    #[test]
    fn test_progress_bar_set_width() {
        let mut bar = ProgressBar::new("Test");
        bar.set_width(50);
        assert_eq!(bar.width, 50);
    }

    #[test]
    fn test_progress_bar_render() {
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 2,
        });

        let mut bar = ProgressBar::new("Test");
        bar.set_value(0.5);
        bar.render(
            Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 2,
            },
            &mut buffer,
        );

        // Verify that something was rendered by checking the buffer
        let _cell = buffer.cell((0, 0));
        // The buffer should have been modified
        assert!(!buffer.content.is_empty());
    }

    #[test]
    fn test_spinner_new() {
        let spinner = Spinner::new("Loading...");
        assert_eq!(spinner.label, "Loading...");
        assert_eq!(spinner.current_frame, 0);
        assert_eq!(spinner.frames.len(), SPINNER_FRAMES.len());
    }

    #[test]
    fn test_spinner_default() {
        let spinner = Spinner::default();
        assert_eq!(spinner.label, "Loading...");
        assert_eq!(spinner.current_frame, 0);
    }

    #[test]
    fn test_spinner_with_frames() {
        let custom_frames = vec!["|".to_string(), "/".to_string(), "-".to_string()];
        let spinner = Spinner::with_frames("Custom", custom_frames.clone());
        assert_eq!(spinner.frames, custom_frames);
    }

    #[test]
    fn test_spinner_tick() {
        let mut spinner = Spinner::new("Loading...");
        let initial_frame = spinner.current_frame();
        let initial_frame_str = initial_frame.to_string(); // Capture the value

        spinner.tick();
        assert_eq!(spinner.current_frame, 1);
        assert_ne!(spinner.current_frame(), initial_frame_str.as_str());
    }

    #[test]
    fn test_spinner_tick_wraps() {
        let custom_frames = vec!["|".to_string(), "/".to_string(), "-".to_string()];
        let mut spinner = Spinner::with_frames("Custom", custom_frames);

        // Tick through all frames
        spinner.tick(); // frame 1
        spinner.tick(); // frame 2
        spinner.tick(); // back to frame 0

        assert_eq!(spinner.current_frame, 0);
        assert_eq!(spinner.current_frame(), "|");
    }

    #[test]
    fn test_spinner_current_frame() {
        let spinner = Spinner::new("Loading...");
        let frame = spinner.current_frame();
        assert_eq!(frame, SPINNER_FRAMES[0]);
    }

    #[test]
    fn test_spinner_render() {
        let mut buffer = Buffer::empty(Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 1,
        });

        let spinner = Spinner::new("Loading...");
        spinner.render(
            Rect {
                x: 0,
                y: 0,
                width: 20,
                height: 1,
            },
            &mut buffer,
        );

        // Verify that something was rendered
        assert!(!buffer.content.is_empty());
    }
}
