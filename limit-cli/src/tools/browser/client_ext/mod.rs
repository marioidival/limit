//! Extension traits for BrowserClient
//!
//! This module provides extension traits that organize browser operations
//! into logical groups. Each trait is implemented in its own file for
//! better compile times and maintainability.

mod interaction;
mod navigation;
mod query;
mod storage;
mod tabs;
mod waiting;

pub use interaction::InteractionExt;
pub use navigation::NavigationExt;
pub use query::QueryExt;
pub use storage::StorageExt;
pub use tabs::TabsExt;
pub use waiting::WaitingExt;
