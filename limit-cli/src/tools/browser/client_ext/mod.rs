//! Extension traits for BrowserClient
//!
//! This module provides extension traits that organize browser operations
//! into logical groups. Each trait is implemented in its own file for
//! better compile times and maintainability.

mod navigation;
mod waiting;
mod interaction;
mod query;
mod tabs;
mod storage;

pub use navigation::NavigationExt;
pub use waiting::WaitingExt;
pub use interaction::InteractionExt;
pub use query::QueryExt;
pub use tabs::TabsExt;
pub use storage::StorageExt;
