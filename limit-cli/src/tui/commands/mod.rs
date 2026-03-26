//! Command system for TUI
//!
//! Provides a plugin-like command architecture for handling user commands.

mod branch;
mod browser;
mod builtin;
mod copy;
mod registry;
mod session;
mod share;

pub use branch::{branch_from, list_branches, BranchInfo};
pub use browser::BrowserCommand;
pub use builtin::{ClearCommand, ExitCommand, HelpCommand};
pub use copy::CopyCommand;
pub use registry::{Command, CommandContext, CommandRegistry, CommandResult};
pub use session::SessionCommand;
pub use share::ShareCommand;

/// Create the default command registry with all built-in commands
pub fn create_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(Box::new(HelpCommand));
    registry.register(Box::new(ClearCommand));
    registry.register(Box::new(ExitCommand));
    registry.register(Box::new(SessionCommand::new()));
    registry.register(Box::new(ShareCommand::new()));
    registry.register(Box::new(BrowserCommand::new()));
    registry.register(Box::new(CopyCommand));

    registry
}
