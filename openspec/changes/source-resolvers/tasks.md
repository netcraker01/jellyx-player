# Tasks: source-resolvers

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~250 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | single PR |
| Delivery strategy | auto-chain |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

## Phase 1: Foundation

- [x] 1.1 Add `uuid` dependency to `src-tauri/Cargo.toml` with `v4` feature
- [x] 1.2 Add `DependencyMissing(String)` variant to `SourceError` in `src-tauri/src/errors/types.rs`
- [x] 1.3 Add `From<SourceError::DependencyMissing>` for `AppError` mapping to `DEPENDENCY_MISSING` code
- [x] 1.4 Add `source_type(&self) -> Source` method to `SourceResolver` trait

## Phase 2: Core Implementation

- [x] 2.1 Implement `SourceRegistry` struct with `new()`, `register()`, `search_all()`, `resolve()`
- [x] 2.2 Complete `YouTubeResolver::search()` with yt-dlp JSON parsing
- [x] 2.3 Complete `YouTubeResolver::resolve()` with `--get-url` for stream URL
- [x] 2.4 Implement `SoundCloudResolver` with search via `scsearch` and resolve

## Phase 3: Integration

- [x] 3.1 Update `PlaybackService` to own `SourceRegistry`
- [x] 3.2 Update `PlaybackService::search()` to delegate to `SourceRegistry::search_all()`
- [x] 3.3 Update `PlaybackService::add_to_queue()` to use `SourceRegistry::resolve()`
- [x] 3.4 Wire `SourceRegistry` construction in `PlaybackService::new()`

## Phase 4: Verification

- [x] 4.1 Run `cargo check` — verify compilation
- [x] 4.2 Run `cargo test` — verify all existing + new tests pass