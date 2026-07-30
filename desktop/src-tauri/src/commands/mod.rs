//! Tauri IPC command handlers for Knowledge OS Desktop.
//!
//! Implements the 11 stateless commands defined in ADR-0022,
//! wrapping existing port traits via `StoreWrapper` delegation.
//! Each command locks the already-internally-synchronized `SqliteStore`
//! (its inner `Mutex<Connection>`), performs the operation, and returns
//! a serializable response.

mod response;
mod store;

pub mod chat;
pub mod entity;
pub mod file;
pub mod import;
pub mod provider;
pub mod search;
pub mod view;

#[allow(unused_imports)]
pub use response::*;
pub use store::AppState;
pub use store::StoreWrapper;
