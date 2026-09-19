//! IPC surface.
//!
//! Commands stay thin: validate, delegate, return. Business logic belongs in
//! `chat`, `db::repo` and `attachments` so it can be tested without a running
//! Tauri app.

pub mod attachments;
pub mod chat;
pub mod conversations;
pub mod projects;
pub mod search;
pub mod secrets;
pub mod settings;
pub mod system;
