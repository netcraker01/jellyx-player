//! Stream source resolution module.
//!
//! Each source resolver implements `SourceResolver` trait,
//! allowing pluggable backends for YouTube, SoundCloud, etc.
//! `SourceRegistry` manages registered resolvers and provides
//! unified search across all sources.

pub mod local;
pub mod soundcloud;
pub mod youtube;
pub mod yt_dlp;

use crate::errors::types::SourceError;
use helix_core::models::playlist::Playlist;
use helix_core::models::source::Source;
use helix_core::models::track::Track;

/// Trait for stream resolvers.
///
/// Each resolver identifies its source type, can search for tracks,
/// and can resolve a track ID to a full Track with stream URL.
/// Requires Send + Sync for safe sharing across Tauri command threads.
pub trait SourceResolver: Send + Sync {
    /// The source type this resolver handles (YouTube, SoundCloud, Local).
    fn source_type(&self) -> Source;

    /// Search for tracks matching the given query.
    /// Returns up to `limit` results starting at `offset` (0-indexed).
    /// Default implementation ignores pagination and returns all results.
    fn search(&self, query: &str, offset: usize, limit: usize) -> Result<Vec<Track>, SourceError>;

    /// Resolve a track by its source-specific identifier to a full Track
    /// with stream_url populated.
    fn resolve(&self, id: &str) -> Result<Track, SourceError>;

    /// Search for playlists matching the given query.
    /// Default implementation returns an empty list — resolvers that
    /// support playlists should override this method.
    fn search_playlists(&self, _query: &str) -> Result<Vec<Playlist>, SourceError> {
        Ok(Vec::new())
    }

    /// Resolve a playlist by its URL or identifier to a full Playlist with tracks.
    /// Default implementation returns UnsupportedSource — resolvers that
    /// support playlists should override this method.
    fn resolve_playlist(&self, _url: &str) -> Result<Playlist, SourceError> {
        Err(SourceError::UnsupportedSource)
    }
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
        self.search_all_enabled(query, None, 0, 50)
    }

    /// Search all enabled sources and merge results with pagination.
    ///
    /// If `enabled_sources` is provided, only resolvers whose source type
    /// is in the set are queried. Local is always included.
    /// `offset` and `limit` control pagination (0-indexed offset, max results).
    pub fn search_all_enabled(
        &self,
        query: &str,
        enabled_sources: Option<&std::collections::HashSet<String>>,
        offset: usize,
        limit: usize,
    ) -> Vec<Track> {
        let mut all_tracks = Vec::new();

        for resolver in &self.resolvers {
            let source_name = format!("{:?}", resolver.source_type());
            // Local source is always included; remote sources must be in enabled set
            let is_enabled = resolver.source_type() == Source::Local
                || enabled_sources.map_or(true, |set| set.contains(&source_name));

            if !is_enabled {
                continue;
            }

            match resolver.search(query, offset, limit) {
                Ok(tracks) => all_tracks.extend(tracks),
                Err(e) => {
                    eprintln!("Search failed for {:?}: {:?}", resolver.source_type(), e);
                }
            }
        }

        all_tracks
    }

    /// Search for playlists across all registered sources and merge results,
    /// filtering by enabled sources.
    pub fn search_playlists_all_enabled(
        &self,
        query: &str,
        enabled_sources: Option<&std::collections::HashSet<String>>,
    ) -> Vec<Playlist> {
        let mut all_playlists = Vec::new();

        for resolver in &self.resolvers {
            let source_name = format!("{:?}", resolver.source_type());
            let is_enabled = resolver.source_type() == Source::Local
                || enabled_sources.map_or(true, |set| set.contains(&source_name));

            if !is_enabled {
                continue;
            }

            match resolver.search_playlists(query) {
                Ok(playlists) => all_playlists.extend(playlists),
                Err(e) => {
                    eprintln!(
                        "Playlist search failed for {:?}: {:?}",
                        resolver.source_type(),
                        e
                    );
                }
            }
        }

        all_playlists
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

    /// Try to resolve a track ID through every registered resolver.
    ///
    /// This is a fallback used when the source type is unknown; it returns the
    /// first successful resolution.
    #[allow(dead_code)]
    pub fn resolve_all(&self, id: &str) -> Result<Track, SourceError> {
        for resolver in &self.resolvers {
            if let Ok(track) = resolver.resolve(id) {
                return Ok(track);
            }
        }
        Err(SourceError::ResolveError(format!(
            "Could not resolve track: {}",
            id
        )))
    }

    /// Search for playlists across all registered sources and merge results.
    #[allow(dead_code)]
    pub fn search_playlists_all(&self, query: &str) -> Vec<Playlist> {
        self.search_playlists_all_enabled(query, None)
    }

    /// Search a specific source for tracks matching the given query.
    pub fn search_source(&self, source: &Source, query: &str) -> Result<Vec<Track>, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.search(query, 0, 50);
            }
        }
        Err(SourceError::UnsupportedSource)
    }

    /// Search a specific source for playlists matching the given query.
    #[allow(dead_code)]
    pub fn search_playlists_source(&self, source: &Source, query: &str) -> Result<Vec<Playlist>, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.search_playlists(query);
            }
        }
        Err(SourceError::UnsupportedSource)
    }

    /// Resolve a playlist by source type and URL/identifier.
    ///
    /// Routes to the first resolver matching the given source type.
    pub fn resolve_playlist(
        &self,
        source: &Source,
        url: &str,
    ) -> Result<Playlist, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.resolve_playlist(url);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal resolver that doesn't override playlist methods,
    /// so the default implementations are tested.
    struct StubResolver;

    impl SourceResolver for StubResolver {
        fn source_type(&self) -> Source {
            Source::Local
        }

        fn search(&self, _query: &str, _offset: usize, _limit: usize) -> Result<Vec<Track>, SourceError> {
            Ok(Vec::new())
        }

        fn resolve(&self, _id: &str) -> Result<Track, SourceError> {
            Err(SourceError::UnsupportedSource)
        }
    }

    #[test]
    fn default_search_playlists_returns_empty() {
        let resolver = StubResolver;
        let result = resolver.search_playlists("test query").unwrap();
        assert!(result.is_empty(), "Default search_playlists should return empty vec");
    }

    #[test]
    fn default_resolve_playlist_returns_unsupported_source() {
        let resolver = StubResolver;
        let result = resolver.resolve_playlist("https://example.com/playlist");
        assert!(
            matches!(result, Err(SourceError::UnsupportedSource)),
            "Default resolve_playlist should return UnsupportedSource"
        );
    }

    #[test]
    fn registry_search_playlists_all_merges_results() {
        let mut registry = SourceRegistry::new();
        registry.register(Box::new(StubResolver));
        let results = registry.search_playlists_all("test");
        assert!(results.is_empty(), "Stub resolver returns empty playlists");
    }

    #[test]
    fn registry_resolve_playlist_routes_to_matching_source() {
        let mut registry = SourceRegistry::new();
        registry.register(Box::new(StubResolver));
        let result = registry.resolve_playlist(&Source::Local, "https://example.com/pl");
        assert!(
            matches!(result, Err(SourceError::UnsupportedSource)),
            "Stub resolver should return UnsupportedSource for playlists"
        );
    }

    #[test]
    fn registry_search_source_routes_to_matching_source() {
        let mut registry = SourceRegistry::new();
        registry.register(Box::new(StubResolver));
        let result = registry.search_source(&Source::Local, "test").unwrap();
        assert!(result.is_empty(), "Stub resolver returns empty tracks");
    }

    #[test]
    fn registry_search_source_returns_unsupported_for_unknown_source() {
        let registry = SourceRegistry::new();
        let result = registry.search_source(&Source::YouTube, "test");
        assert!(
            matches!(result, Err(SourceError::UnsupportedSource)),
            "No resolver registered for YouTube should return UnsupportedSource"
        );
    }

    #[test]
    fn registry_search_playlists_source_routes_to_matching_source() {
        let mut registry = SourceRegistry::new();
        registry.register(Box::new(StubResolver));
        let result = registry.search_playlists_source(&Source::Local, "test").unwrap();
        assert!(result.is_empty(), "Stub resolver returns empty playlists");
    }

    #[test]
    fn registry_search_playlists_source_returns_unsupported_for_unknown_source() {
        let registry = SourceRegistry::new();
        let result = registry.search_playlists_source(&Source::YouTube, "test");
        assert!(
            matches!(result, Err(SourceError::UnsupportedSource)),
            "No resolver registered for YouTube should return UnsupportedSource"
        );
    }
}
