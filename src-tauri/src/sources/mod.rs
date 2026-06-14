//! Stream source resolution module.
//!
//! Each source resolver implements `SourceResolver` trait,
//! allowing pluggable backends for YouTube, SoundCloud, etc.

pub mod youtube;
// pub mod soundcloud;
// pub mod radio;

/// Result from a source search.
#[derive(Debug, Clone)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub thumbnail: String,
    pub stream_url: String,
    pub source: String,
}

/// Trait for stream resolvers.
pub trait SourceResolver {
    fn search(&self, query: &str) -> Result<Vec<Track>, SourceError>;
    fn resolve(&self, id: &str) -> Result<Track, SourceError>;
}

#[derive(Debug)]
pub enum SourceError {
    NetworkError(String),
    ResolveError(String),
    UnsupportedSource,
}
