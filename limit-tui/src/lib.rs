// limit-tui: Terminal UI library

pub mod backend;
pub mod components;
pub mod layout;
pub mod syntax;
pub mod vdom;

pub use backend::{render_vdom_to_ratatui, run_event_loop, RatatuiBackend};
pub use components::{
    ChatView, InputPrompt, InputResult, Message, ProgressBar, Role, SelectPrompt, SelectResult,
    Spinner,
};
pub use layout::{AlignItems, FlexDirection, FlexStyle, FlexboxLayout, JustifyContent};
pub use syntax::SyntaxHighlighter;
pub use vdom::{apply, diff, render, Patch, VNode};
