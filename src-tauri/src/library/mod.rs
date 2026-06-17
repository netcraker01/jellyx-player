//! Library domain module.

pub mod service;
pub mod state;
pub mod models;

pub use service::LibraryService;
pub use models::{FavoriteEntry, HistoryEntry};