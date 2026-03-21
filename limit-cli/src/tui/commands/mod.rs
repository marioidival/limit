//! Command system for TUI
//!
//! Provides a plugin-like command architecture for handling user commands.

mod browser;
mod builtin;
mod registry;
mod session;
mod share;
mod tldr;

pub use browser::BrowserCommand;
pub use builtin::{ClearCommand, ExitCommand, HelpCommand};
pub use registry::{Command, CommandContext, CommandRegistry, CommandResult};
pub use session::SessionCommand;
pub use share::ShareCommand;
pub use tldr::TldrCommand;

/// Create the default command registry with all built-in commands
pub fn create_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(Box::new(HelpCommand));
    registry.register(Box::new(ClearCommand));
    registry.register(Box::new(ExitCommand));
    registry.register(Box::new(SessionCommand::new()));
    registry.register(Box::new(ShareCommand::new()));
    registry.register(Box::new(BrowserCommand::new()));
    registry.register(Box::new(TldrCommand));

    registry
}
