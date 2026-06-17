# Proposal: source-resolvers

## Intent
Implement YouTube and SoundCloud source resolvers so Helix can search and stream from external sources. Currently YouTubeResolver is a stub, SoundCloudResolver is a placeholder, and PlaybackService::search() only queries YouTube with empty results.

## Scope

### In Scope
- Complete YouTubeResolver: search + resolve via yt-dlp
- Complete SoundCloudResolver: search + resolve via yt-dlp
- SourceRegistry to manage multiple resolvers
- PlaybackService::search() queries all registered sources
- Graceful handling when yt-dlp is not installed (SourceError::DependencyMissing)
- Add uuid dependency for Track IDs

### Out of Scope
- YouTube Data API integration
- Local file resolver implementation
- Async/await refactor
- Caching of search results or stream URLs

## Capabilities

### New Capabilities
- `youtube-resolver`: YouTube search and stream URL resolution via yt-dlp
- `soundcloud-resolver`: SoundCloud search and stream URL resolution via yt-dlp
- `source-registry`: Central registry managing multiple SourceResolver implementations

### Modified Capabilities
- `playback-search`: PlaybackService.search() queries all registered sources

## Approach
Use yt-dlp for both sources. Search via `ytsearch<N>:query` / `scsearch<N>:query` with `--dump-json --no-download`. Resolve via `--get-url`. SourceRegistry holds `Vec<Box<dyn SourceResolver>>`, provides `search_all()`. PlaybackService owns SourceRegistry. Add `SourceError::DependencyMissing` for missing yt-dlp.

## Affected Areas
| Area | Impact | Description |
|------|--------|-------------|
| `sources/mod.rs` | Modified | Add SourceRegistry |
| `sources/youtube.rs` | Modified | Complete search/resolve |
| `sources/soundcloud/mod.rs` | Modified | Implement resolver |
| `playback/service.rs` | Modified | Use SourceRegistry |
| `errors/types.rs` | Modified | Add DependencyMissing |
| `Cargo.toml` | Modified | Add uuid |

## Risks
| Risk | Likelihood | Mitigation |
|------|------------|------------|
| yt-dlp not installed | High | Return DependencyMissing error |
| yt-dlp output format changes | Medium | Parse defensively |
| Slow search | High | Sequential for v0.1 |

## Rollback Plan
Revert to stubs. SourceRegistry removable. All changes are additive.

## Success Criteria
- [ ] YouTube search returns Vec<Track> from yt-dlp
- [ ] SoundCloud search returns Vec<Track> from yt-dlp
- [ ] Resolve returns Track with stream_url
- [ ] Missing yt-dlp returns DependencyMissing, not panic
- [ ] PlaybackService::search() queries YouTube + SoundCloud
- [ ] cargo check + cargo test pass