# Proposal: Home Recommendations

## Intent

Home currently fulfills only the `Recently Played` slice of the PRD. This change adds a first discovery experience so Home becomes the musical return point required by `docs/PRD.md`, while keeping Rust as the source of truth for recommendation assembly.

## Scope

### In Scope
- Backend `get_home_snapshot`-style command returning Home data in one payload
- Recommendation heuristics derived from history, favorites, and local library metadata
- Home sections for `Recently Played` and `Recommendations / Discover`
- Dedicated Home frontend store consuming the snapshot and rendering reusable track/artist/album sections
- Empty and low-data states when recommendation inputs are sparse

### Out of Scope
- Streaming-catalog recommendations from YouTube, SoundCloud, or external APIs
- Genre/mood shortcuts from the PRD; deferred until recommendation inventory is richer
- ML/personalization models, online profiles, or cross-device sync
- New first-level navigation beyond Home/Search/Now Playing/Favorites

## Capabilities

### New Capabilities
- `home-recommendations`: Backend-computed Home snapshot with recommendation/discover sections for local-library-driven listening

### Modified Capabilities
- None

## Approach

Add a Rust Home snapshot service/IPC command that aggregates `getHistory`, `getFavorites`, local tracks, and scanner metadata into a single DTO. Heuristics stay simple and explainable: favor recently engaged artists/albums, prefer favorited affinity, exclude over-repeated tracks, and degrade gracefully to recency/library picks when signals are weak. Svelte stays a thin client: fetch snapshot, render sections, handle loading/error states.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/library/service.rs` | Modified | Build Home snapshot + heuristics |
| `src-tauri/src/ipc/commands.rs` | Modified | Expose Home snapshot command |
| `ui/src/services/commands.ts` | Modified | Add Home snapshot IPC wrapper |
| `ui/src/features/home/` | New | Home store and section composition |
| `ui/src/routes/Home/Page.svelte` | Modified | Replace history-only page with snapshot-driven Home |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Weak recommendation quality with sparse data | High | Deterministic fallback ordering + explicit empty states |
| Home logic drifting into Svelte | Medium | Keep ranking/assembly in Rust DTO/service |
| Local-only inventory feels narrow | Medium | Scope MVP as local discovery, leave streaming enrichment for later |

## Rollback Plan

Remove the Home snapshot command and frontend store, then restore the current history-only Home page. No persistence or schema migration is required.

## Dependencies

- Artist/album detail views (done)
- Grouped search (done)
- Local scanner and local track metadata (done)

## Success Criteria

- [ ] Home shows both `Recently Played` and `Recommendations / Discover`
- [ ] Recommendations are returned by Rust in a single snapshot payload
- [ ] Empty or sparse libraries still produce a valid, non-broken Home state
- [ ] No recommendation ranking logic is required in page components
