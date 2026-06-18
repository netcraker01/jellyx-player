# Tasks: Artist/Album Detail Views

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~700-950 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR1 Rust search/detail APIs → PR2 TS stores + grouped Search → PR3 detail routes + Now Playing links |
| Delivery strategy | force-chained |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Backend DTOs, IDs, commands, playback hook | PR 1 | Rust-only slice; base branch TBD |
| 2 | Frontend types, command wrappers, stores, grouped Search UI | PR 2 | Depends on PR 1 |
| 3 | Artist/Album routes, Now Playing links, polish | PR 3 | Depends on PR 2 |

## Phase 1: Rust Contracts and Services

- [x] 1.1 RED: Add failing Rust tests in `src-tauri/src/library/service.rs` for normalized artist/album IDs and grouped summary shaping. AC: tests cover mixed matches and empty groups from REQ-MS-1/2.
- [x] 1.2 GREEN: Add `GroupedSearchResult`, `ArtistSummary`, `AlbumSummary`, `ArtistDetail`, `AlbumDetail` plus ID helpers in `src-tauri/src/ipc/dto.rs` and exports. AC: serde uses camelCase and IDs match design rules.
- [x] 1.3 RED: Add failing command/service tests in `src-tauri/src/library/service.rs` or `src-tauri/src/ipc/commands.rs` for `get_artist_detail`, `get_album_detail`, not-found errors, and album track ordering. AC: covers REQ-AD-1 and REQ-AL-1/2.
- [x] 1.4 GREEN: Implement `search_grouped`, `get_artist_detail`, `get_album_detail`, and `play_album` in `src-tauri/src/library/service.rs`, `src-tauri/src/ipc/commands.rs`, and `src-tauri/src/app/setup.rs`. AC: old `search` remains unchanged; `play_album` replaces queue in album order.
- [x] 1.5 REFACTOR: Extract shared DB/query parsing helpers in `src-tauri/src/library/service.rs`. AC: duplicated normalization/grouping logic is removed without changing test results.

## Phase 2: Frontend Types and Stores

- [ ] 2.1 RED: Add failing Vitest coverage in `ui/src/tests/search-grouped.test.ts` for grouped-search store states, filter forwarding, and detail-store not-found handling. AC: tests assert loading, error, clear, and empty-group behavior.
- [ ] 2.2 GREEN: Extend `ui/src/shared/types/models.ts` and `ui/src/services/commands.ts` for grouped search/detail/play-album contracts. AC: TS models mirror Rust DTO fields exactly.
- [ ] 2.3 GREEN: Create `ui/src/features/search/stores/searchGrouped.ts`, `ui/src/features/library/stores/artistDetail.ts`, and `ui/src/features/library/stores/albumDetail.ts`. AC: stores call new commands and expose readable loading/error state.
- [ ] 2.4 REFACTOR: Update `ui/src/routes/Search/Page.svelte` to depend on grouped-search store while leaving legacy `search.ts` untouched for compatibility. AC: Search page compiles without direct flat-results usage.

## Phase 3: Routes and UI Wiring

- [ ] 3.1 RED: Add failing route/component tests in `ui/src/tests/detail-routes.test.ts` for grouped sections, artist placeholder, album not-found state, and play-album action. AC: scenarios map to REQ-MS-3, REQ-AD-1/2, REQ-AL-1/2.
- [ ] 3.2 GREEN: Build `ui/src/features/search/components/GroupedResults.svelte` and update `ui/src/shared/components/ArtistCard.svelte` plus `ui/src/shared/components/AlbumCard.svelte`. AC: cards navigate to `/artist/:id` and `/album/:id`; songs keep play/queue actions.
- [ ] 3.3 GREEN: Add `ui/src/routes/Artist/Page.svelte`, `ui/src/routes/Album/Page.svelte`, and route entries in `ui/src/app/App.svelte`. AC: pages load backend data, show placeholders, and clear stale state on route change.
- [ ] 3.4 GREEN: Update `ui/src/features/player/components/NowPlayingInfo.svelte`, `ui/src/routes/NowPlaying/Page.svelte`, and `ui/src/shared/utils/actions.ts` for open-artist/open-album/play-album actions. AC: navigation works only when resolvable and album play surfaces toast errors.

## Phase 4: Verification

- [ ] 4.1 Run `cargo test`. AC: new Rust search/detail/play-album coverage passes with no regressions.
- [ ] 4.2 Run `cd ui && npx vitest run`. AC: new grouped-search and detail-route tests pass.
- [ ] 4.3 Manually verify Search, Artist, Album, and Now Playing flows against `openspec/changes/artist-album-detail-views/spec.md`. AC: mixed search, unknown album, and full-album queue scenarios all match spec.
