//! Source resolver trait — abstracts YouTube, SoundCloud, and local sources.
//!
//! The engine defines the trait; frontends register concrete implementations
//! (yt-dlp, local file scanner, etc.) at startup.

use jellyx_core::models::playlist::Playlist;
use jellyx_core::models::source::Source;
use jellyx_core::models::track::Track;

/// Error type for source resolution.
#[derive(Debug, Clone)]
pub enum SourceError {
    NetworkError(String),
    ResolveError(String),
    UnsupportedSource,
    DependencyMissing(String),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "network error: {msg}"),
            Self::ResolveError(msg) => write!(f, "resolve error: {msg}"),
            Self::UnsupportedSource => write!(f, "unsupported source"),
            Self::DependencyMissing(msg) => write!(f, "dependency missing: {msg}"),
        }
    }
}

impl std::error::Error for SourceError {}

/// Trait for stream resolvers.
///
/// Each resolver identifies its source type, can search for tracks,
/// and can resolve a track ID to a full Track with stream URL.
pub trait SourceResolver: Send + Sync {
    fn source_type(&self) -> Source;
    fn search(&self, query: &str, offset: usize, limit: usize) -> Result<Vec<Track>, SourceError>;
    fn resolve(&self, id: &str) -> Result<Track, SourceError>;

    fn resolve_stream_url(&self, id: &str) -> Result<String, SourceError> {
        let track = self.resolve(id)?;
        track
            .stream_url
            .ok_or_else(|| SourceError::ResolveError("no stream URL".into()))
    }

    fn search_playlists(&self, _query: &str) -> Result<Vec<Playlist>, SourceError> {
        Ok(Vec::new())
    }

    fn resolve_playlist(&self, _url: &str) -> Result<Playlist, SourceError> {
        Err(SourceError::UnsupportedSource)
    }
}

/// Registry that manages multiple source resolvers.
pub struct SourceRegistry {
    resolvers: Vec<Box<dyn SourceResolver>>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    pub fn register(&mut self, resolver: Box<dyn SourceResolver>) {
        self.resolvers.push(resolver);
    }

    pub fn search_all(&self, query: &str) -> Vec<Track> {
        self.resolvers
            .iter()
            .filter_map(|r| r.search(query, 0, 50).ok())
            .flatten()
            .collect()
    }

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
            let is_enabled = resolver.source_type() == Source::Local
                || enabled_sources.map_or(true, |set| set.contains(&source_name));
            if !is_enabled {
                continue;
            }
            if let Ok(tracks) = resolver.search(query, offset, limit) {
                all_tracks.extend(tracks);
            }
        }
        all_tracks
    }

    pub fn resolve(&self, source: &Source, id: &str) -> Result<Track, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.resolve(id);
            }
        }
        Err(SourceError::UnsupportedSource)
    }

    pub fn resolve_stream_url(&self, source: &Source, id: &str) -> Result<String, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.resolve_stream_url(id);
            }
        }
        Err(SourceError::UnsupportedSource)
    }

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

    pub fn search_source(&self, source: &Source, query: &str) -> Result<Vec<Track>, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.search(query, 0, 50);
            }
        }
        Err(SourceError::UnsupportedSource)
    }

    pub fn search_playlists_all(&self, query: &str) -> Vec<Playlist> {
        self.resolvers
            .iter()
            .filter_map(|r| r.search_playlists(query).ok())
            .flatten()
            .collect()
    }

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
            if let Ok(playlists) = resolver.search_playlists(query) {
                all_playlists.extend(playlists);
            }
        }
        all_playlists
    }

    pub fn resolve_playlist(&self, source: &Source, url: &str) -> Result<Playlist, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.resolve_playlist(url);
            }
        }
        Err(SourceError::UnsupportedSource)
    }

    pub fn search_playlists_source(
        &self,
        source: &Source,
        query: &str,
    ) -> Result<Vec<Playlist>, SourceError> {
        for resolver in &self.resolvers {
            if resolver.source_type() == *source {
                return resolver.search_playlists(query);
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

    struct FakeResolver {
        source: Source,
    }

    impl SourceResolver for FakeResolver {
        fn source_type(&self) -> Source {
            self.source.clone()
        }
        fn search(
            &self,
            _query: &str,
            _offset: usize,
            _limit: usize,
        ) -> Result<Vec<Track>, SourceError> {
            Ok(vec![Track {
                id: "fake-1".into(),
                source: self.source.clone(),
                source_id: "fake".into(),
                title: "Fake Track".into(),
                artist: "Fake Artist".into(),
                album: None,
                duration: None,
                thumbnail: None,
                stream_url: None,
                local_path: None,
                playlist_id: None,
                metadata: std::collections::HashMap::new(),
            }])
        }
        fn resolve(&self, id: &str) -> Result<Track, SourceError> {
            Ok(Track {
                id: id.to_string(),
                source: self.source.clone(),
                source_id: id.to_string(),
                title: "Resolved".into(),
                artist: "Artist".into(),
                album: None,
                duration: None,
                thumbnail: None,
                stream_url: Some("http://example.com/stream".into()),
                local_path: None,
                playlist_id: None,
                metadata: std::collections::HashMap::new(),
            })
        }
    }

    #[test]
    fn registry_resolves_by_source() {
        let mut reg = SourceRegistry::new();
        reg.register(Box::new(FakeResolver {
            source: Source::Local,
        }));
        let track = reg.resolve(&Source::Local, "t1").unwrap();
        assert_eq!(track.id, "t1");
    }

    #[test]
    fn registry_resolve_all_tries_all() {
        let mut reg = SourceRegistry::new();
        reg.register(Box::new(FakeResolver {
            source: Source::Local,
        }));
        let track = reg.resolve_all("any-id").unwrap();
        assert_eq!(track.title, "Resolved");
    }

    #[test]
    fn registry_search_all_aggregates() {
        let mut reg = SourceRegistry::new();
        reg.register(Box::new(FakeResolver {
            source: Source::Local,
        }));
        reg.register(Box::new(FakeResolver {
            source: Source::YouTube,
        }));
        let results = reg.search_all("test");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn source_error_display() {
        assert!(
            SourceError::UnsupportedSource
                .to_string()
                .contains("unsupported")
        );
        assert!(
            SourceError::NetworkError("timeout".into())
                .to_string()
                .contains("timeout")
        );
    }
}
