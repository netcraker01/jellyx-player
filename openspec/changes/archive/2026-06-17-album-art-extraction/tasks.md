# Tasks: Album Art Extraction

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 250–350 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-always |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Backend: revision loop + art cache + setup | PR 1 | main; Rust-only, testable independently |
| 2 | Frontend: asset URL util + component wiring | PR 1 | same PR; depends on Unit 1 |

## Phase 1: Foundation

- [x] 1.1 Add `sha2` crate to `src-tauri/Cargo.toml` (verify not transitive via `cargo tree` first)
- [x] 1.2 Add `art_cache_dir() -> PathBuf` and `ensure_art_cache_dir()` to `src-tauri/src/shared/utils.rs`
- [x] 1.3 Call `ensure_art_cache_dir()` in `src-tauri/src/app/setup.rs` before `Database::open`

## Phase 2: Core Implementation

- [x] 2.1 Add `media_type_to_ext(media_type: &Option<String>) -> &str` to `scanner.rs` — jpeg→jpg, png→png, else→bin
- [x] 2.2 Add `cache_art(data: &[u8], media_type: &Option<String>) -> Result<PathBuf, ScannerError>` to `scanner.rs` — SHA-256 hash, write to `art_cache_dir()` if not exists
- [x] 2.3 Add `extract_visual(visuals: &[Visual]) -> Option<(&Box<[u8]>, &Option<String>)>` to `scanner.rs` — find `StandardVisualKey::FrontCover`
- [x] 2.4 Replace `metadata.current()` block in `extract_metadata()` with `pop()` loop — merge tags across revisions, call `extract_visual` + `cache_art`, set `Track.thumbnail`

## Phase 3: Tauri Config

- [x] 3.1 Add `assetProtocol.enable: true` and `scope: ["$APPDATA/art/**/*"]` to `src-tauri/tauri.conf.json` security section
- [x] 3.2 Update CSP in `tauri.conf.json` to include `img-src 'self' asset: http://asset.localhost`

## Phase 4: Frontend Wiring

- [x] 4.1 Create `ui/src/shared/utils/assetUrl.ts` — wrapper for `convertFileSrc()` from `@tauri-apps/api/core`
- [x] 4.2 Update `ui/src/features/player/components/NowPlayingInfo.svelte` — use `albumArtUrl(track.thumbnail)` for `<img src>`
- [x] 4.3 Update `ui/src/shared/components/TrackList.svelte` — add thumbnail `<img>` column using `albumArtUrl()`
- [x] 4.4 Update `ui/src/shared/components/AlbumCard.svelte` — render art from `albumArtUrl()` with placeholder fallback

## Phase 5: Testing

- [x] 5.1 Unit test `media_type_to_ext` in `scanner.rs` — jpeg/png/unknown cases (spec: Unsupported media type scenario)
- [x] 5.2 Unit test `extract_visual` with mock `Vec<Visual>` — FrontCover found, no FrontCover, empty visuals (spec scenarios)
- [x] 5.3 Unit test `cache_art` with temp dir — write file, dedup same hash, verify path returned
- [x] 5.4 Integration test `extract_metadata` with real MP3/FLAC fixtures in `tests/fixtures/` — verify thumbnail populated