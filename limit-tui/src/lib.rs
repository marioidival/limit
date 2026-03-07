// limit-tui: Terminal UI library

pub mod backend;
pub mod layout;
pub mod vdom;
pub mod components;

pub use backend::{render_vdom_to_ratatui, run_event_loop, RatatuiBackend};
pub use components::{ChatView, Message, Role, ProgressBar, Spinner, InputPrompt, InputResult, SelectPrompt, SelectResult};
pub use layout::{AlignItems, FlexDirection, FlexStyle, FlexboxLayout, JustifyContent};
pub use vdom::{apply, diff, render, Patch, VNode};
