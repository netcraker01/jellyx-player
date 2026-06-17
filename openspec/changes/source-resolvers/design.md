# Design: source-resolvers

## Technical Approach
Implement YouTube and SoundCloud resolvers using yt-dlp. Introduce SourceRegistry to manage resolvers, replace hardcoded YouTubeResolver calls. All synchronous via std::process::Command. Add SourceError::DependencyMissing.

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Integration tool | yt-dlp CLI | PRD specifies yt-dlp, one dependency for both sources |
| Registry pattern | Vec<Box<dyn SourceResolver>> | Simpler than HashMap for v0.1, trait objects for runtime composition |
| Search execution | Sequential | No tokio, predictable, parallel later without API change |
| Error resilience | Fail-soft per source | Partial results > no results if one source fails |
| Track IDs | uuid v4 | Unique across sources, avoids source_id collisions |

## Data Flow

```
User query → PlaybackService.search(query)
  → SourceRegistry.search_all(query)
    → YouTubeResolver.search() → yt-dlp "ytsearch5:Q" --dump-json
    → SoundCloudResolver.search() → yt-dlp "scsearch5:Q" --dump-json
    → Merge results → Vec<Track> → IPC → Frontend
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `sources/mod.rs` | Modify | Add SourceRegistry, add source_type() to trait |
| `sources/youtube.rs` | Modify | Complete search/resolve with JSON parsing |
| `sources/soundcloud/mod.rs` | Modify | Implement SoundCloudResolver |
| `playback/service.rs` | Modify | Own SourceRegistry, delegate search/resolve |
| `errors/types.rs` | Modify | Add DependencyMissing variant |
| `Cargo.toml` | Modify | Add uuid dependency |

## Interfaces

```rust
pub trait SourceResolver {
    fn source_type(&self) -> Source;
    fn search(&self, query: &str) -> Result<Vec<Track>, SourceError>;
    fn resolve(&self, id: &str) -> Result<Track, SourceError>;
}

pub struct SourceRegistry {
    resolvers: Vec<Box<dyn SourceResolver>>,
}

pub enum SourceError {
    NetworkError(String),
    ResolveError(String),
    UnsupportedSource,
    DependencyMissing(String),
}
```

## Testing Strategy
- Unit: SourceError mapping, JSON parsing, registry merge
- Integration: Real yt-dlp calls (manual/CI)

## Migration
None required — purely additive changes.