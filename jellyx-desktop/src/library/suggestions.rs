//! Suggestion categories — re-exported from the engine.
//!
//! The engine owns the pure suggestion category logic so both Tauri and
//! Ratatui frontends share a single source of truth.

pub use jellyx_engine::suggestions::{get_suggestion_categories, SuggestionCategory};
