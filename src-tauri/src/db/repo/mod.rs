//! Repositories.
//!
//! Every function takes a `&Connection` rather than the `Db` wrapper so it
//! composes inside a `Db::tx` transaction as easily as standalone.

pub mod attachments;
pub mod conversations;
pub mod messages;
pub mod projects;
pub mod prompts;
pub mod search;
pub mod settings;
