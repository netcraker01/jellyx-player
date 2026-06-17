//! Stream source resolution module.
//!
//! Each source resolver implements `SourceResolver` trait,
//! allowing pluggable backends for YouTube, SoundCloud, etc.
//! `SourceRegistry` manages registered resolvers and provides
//! unified search across all sources.

pub mod youtube;
pub mod soundcloud;
pub mod local;

use crate::errors::types::SourceError;
use crate::models::source::Source;
use crate::models::track::Track;

/// Trait for stream resolvers.
///
/// Each resolver identifies its source type, can search for tracks,
/// and can resolve a track ID to a full Track with stream URL.
/// Requires Send + Sync for safe sharing across Tauri command threads.
pub trait SourceResolver: Send + Sync {
    /// The source type this resolver handles (YouTube, SoundCloud, Local).
    fn source_type(&self) -> Source;

    /// Search for tracks matching the given query.
    fn search(&self, query: &str) -> Result<Vec<Track>, SourceError>;

    /// Resolve a track by its source-specific identifier to a full Track
    /// with stream_url populated.
    fn resolve(&self, id: &str) -> Result<Track, SourceError>;
}

/// Registry that manages multiple source resolvers.
///
/// Provides unified search across all registered sources and
/// routes resolve calls to the appropriate resolver by source type.
pub struct SourceRegistry {
    resolvers: Vec<Box<dyn SourceResolver + Send + Sync>>,
}

impl SourceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Register a new source resolver.
    pub fn register(&mut self, resolver: Box<dyn SourceResolver + Send + Sync>) {
        self.resolvers.push(resolver);
    }

    /// Search all registered sources and merge results.
    ///
    /// Queries each resolver sequentially. If a resolver fails,
    /// its error is silently logged and other resolvers continue.
    /// This ensures partial results even if one source is unavailable.
    pub fn search_all(&self, query: &str) -> Vec<Track> {
        let mut all_tracks = Vec::new();

        for resolver in &self.resolvers {
            match resolver.search(query) {
                Ok(tracks) => all_tracks.extend(tracks),
                Err(e) => {
                    // Log the error but continue with other sources
                    eprintln!(
                        "Search failed for {:?}: {:?}",
                        resolver.source_type(),
                        e
                    );
                }
            }
        }

        all_tracks
    }

    /// Resolve a track by source type and identifier.
    ///
    /// Routes to the first resolver matching the given source type.
    pub fn resolve(&self, source: &Source, id: &str) -> Result<Track, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.resolve(id);
            }
        }
        Err(SourceError::UnsupportedSource)
    }
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
