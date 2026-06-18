# Proposal: Artist/Album Detail Views

## Intent

Deliver PRD-required artist and album browsing so Search can return songs, artists, and albums in separate blocks, and users can open dedicated detail views from Search and Now Playing. This closes a core MVP gap in navigation clarity and keeps Rust as the source of truth instead of deriving entities in Svelte.

## Scope

### In Scope
- Grouped search results for songs, artists, and albums, with optional type filtering.
- Route-driven `/artist/:id` and `/album/:id` views in the central content area.
- Artist detail payloads with name, image, top songs, and albums; album detail payloads with cover, artist, tracks, and play-full-album.
- Now Playing quick actions that open the current track's artist or album.

### Out of Scope
- New remote-source metadata enrichment beyond what current providers/scanner expose reliably.
- Visual redesign outside the existing dark shell, sidebar, bottom bar, and central content layout.
- Library-wide browse pages, recommendations, playlists, or doc cleanup beyond touched references.

## Capabilities

### New Capabilities
- `media-search`: grouped song/artist/album search results with optional type filtering and result navigation.
- `artist-detail-view`: backend-driven artist detail route with header metadata, top tracks, and album list.
- `album-detail-view`: backend-driven album detail route with metadata, ordered tracks, and full-album playback.

### Modified Capabilities
- None.

## Approach

Add backend-first query DTOs and Tauri commands in `src-tauri/src/ipc/commands.rs`, backed primarily by local scanner/library metadata and existing `Artist`, `Album`, and `Track` models. Extend `ui/src/services/commands.ts`, `ui/src/features/search/stores/search.ts`, and new entity stores under `ui/src/features/*/stores/`; wire dynamic routes in `ui/src/app/App.svelte`; update Search and Now Playing to navigate into artist/album pages.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/ipc/commands.rs` | Modified | Add grouped search and artist/album detail commands |
| `src-tauri/src/app/setup.rs` | Modified | Register new Tauri commands |
| `ui/src/app/App.svelte` | Modified | Add artist/album routes |
| `ui/src/features/search/` | Modified | Replace flat track search state/UI with grouped typed results |
| `ui/src/routes/NowPlaying/Page.svelte`, `ui/src/features/player/components/NowPlayingInfo.svelte` | Modified | Add open-artist/open-album actions |
| `ui/src/routes/Artist/`, `ui/src/routes/Album/` | New | Detail pages and supporting entity stores/components |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Search contract is track-only today | High | Introduce explicit DTOs and backend tests first |
| Remote metadata is incomplete/stable IDs may be weak | Med | Make local scanner metadata the primary detail source; degrade gracefully |
| UI components for artist detail are missing/stubbed | Med | Build reusable cards/detail sections behind route-level tests |

## Rollback Plan

Revert new routes, stores, and IPC commands together, returning Search to `Track[]` results and removing artist/album entry points from Now Playing.

## Dependencies

- Existing `Track`, `Artist`, `Album` models
- Local scanner/library metadata already persisted for artist/album fields
- Tauri IPC command pattern in `src-tauri/src/ipc/commands.rs`

## Success Criteria

- [ ] Search returns grouped song/artist/album results and supports type filtering per PRD.
- [ ] Clicking an artist or album result opens a dedicated detail route with required metadata.
- [ ] Album detail can start full-album playback, and Now Playing exposes open-artist/open-album actions.
