//! Stream source resolution module.
//!
//! Each source resolver implements `SourceResolver` trait,
//! allowing pluggable backends for YouTube, SoundCloud, etc.

pub mod youtube;
pub mod soundcloud;
pub mod local;

/// Trait for stream resolvers.
pub trait SourceResolver {
    fn search(&self, query: &str) -> Result<Vec<crate::models::track::Track>, crate::errors::types::SourceError>;
    fn resolve(&self, id: &str) -> Result<crate::models::track::Track, crate::errors::types::SourceError>;
}

/// Backward-compat re-exports; will be removed in Phase 5.
pub use crate::errors::types::SourceError;
pub use crate::models::track::Track;
