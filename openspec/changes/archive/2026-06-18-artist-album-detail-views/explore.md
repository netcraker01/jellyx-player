## Exploration: artist-album-detail-views

### Current State
Routing is defined in `ui/src/app/App.svelte` with `svelte-routing`. The active routes are `/`, `/search`, `/favorites`, and `/now-playing`, mirrored by the sidebar links in `ui/src/app/layout/Sidebar.svelte`. There are no dynamic routes for artist or album detail pages, and `ui/src/routes/Library/Page.svelte` exists but is not mounted anywhere.

The current Search page is track-only. `ui/src/routes/Search/Page.svelte` renders a `SearchBar` plus `ResultsList`, and `ui/src/features/search/stores/search.ts` calls `commands.search(query)` and stores `Track[]`. `ui/src/features/search/components/ResultsList.svelte` renders a flat list of tracks with play, play-next, add-to-queue, and favorite actions. There is no grouping by songs/artists/albums, no result-type filtering, and no navigation into artist/album views.

On the backend, shared models exist for `Track`, `Artist`, and `Album` in `src-tauri/src/models/`, and the frontend mirrors them in `ui/src/shared/types/models.ts`. In practice, only `Track` is wired through current search and library flows. The local scanner extracts `artist`, `album`, `duration`, and album art into `Track` records, and persistence stores favorites/history/local tracks as serialized `Track` JSON plus scanner metadata.

Tauri command coverage is good for playback, queue, favorites, history, and local scanning, but not for artist/album detail queries. `src-tauri/src/ipc/commands.rs` exposes `search`, `get_favorites`, `get_history`, `scan_folder`, `get_local_tracks`, `get_watched_folders`, and related playback commands. The only search/library discovery command is `search(query) -> Vec<Track>`. There are no commands for grouped search results, artist lookup, album lookup, artist top tracks, artist albums, or album track listings.

Frontend store architecture is feature-scoped and backend-driven: `player.ts`, `search.ts`, `favorites.ts`, and `library.ts`. This matches the Rust Source-of-Truth model from `docs/ARCHITECTURE.md`, but there is currently no store dedicated to navigation state for detail views or to artist/album entities.

`docs/ARCHITECTURE.md` says the frontend should use a hybrid structure with `routes/` for pages and `features/` for domain logic, and `docs/UI_DESIGN.md` says the center content area should render Home, Search results, and Artist/Album detail views. Two architecture details are stale versus the code: it still references `+page.svelte` route files, and its search flow mentions a `search_results` event even though the current implementation returns search results directly from the invoke command.

`docs/PRD.md` is clear about the target: search must support song/artist/album results, optionally filtered by type, presented in separate blocks; artist results must open their own view; artist detail needs name, image, top songs, and albums; album detail needs cover, title, artist, track list, and play-full-album. The PRD also says Now Playing quick actions should open artist/album.

`docs/UI_DESIGN.md` adds only high-level visual guidance for these views: they belong in the central content area, within the existing Spotify-like shell, dark-mode-only design, and bottom bar/Sidebar navigation model. There is no component-level layout spec yet for artist/album detail pages.

### Affected Areas
- `ui/src/app/App.svelte` — current route table; needs new artist/album routes.
- `ui/src/app/layout/Sidebar.svelte` — reflects current top-level navigation constraints.
- `ui/src/routes/Search/Page.svelte` — current search page orchestration.
- `ui/src/features/search/stores/search.ts` — currently stores only `Track[]` search results.
- `ui/src/features/search/components/ResultsList.svelte` — flat track list UI; no typed/grouped rendering.
- `ui/src/routes/NowPlaying/Page.svelte` — PRD expects open-artist/open-album actions from this flow.
- `ui/src/features/player/components/NowPlayingInfo.svelte` — likely entry point for artist/album navigation actions.
- `ui/src/features/*/stores/` — existing store pattern to extend for detail view state.
- `ui/src/shared/components/ArtistCard.svelte` — currently a stub.
- `ui/src/shared/components/AlbumCard.svelte` — usable visual base, but not wired to navigation/data.
- `src-tauri/src/ipc/commands.rs` — missing artist/album query commands.
- `src-tauri/src/app/setup.rs` — command registration will need expansion.
- `src-tauri/src/models/{artist.rs,album.rs,track.rs}` — canonical entity shapes already exist.
- `src-tauri/src/sources/mod.rs` — current search contract is `Vec<Track>` only.
- `src-tauri/src/sources/local/scanner.rs` — already extracts local artist/album metadata that detail views can reuse.
- `docs/ARCHITECTURE.md` — routing/search flow needs alignment with real implementation.
- `docs/PRD.md` — source of functional requirements for these views.
- `docs/UI_DESIGN.md` — source of visual/layout constraints.

### Approaches
1. **Route-driven detail pages with new backend entity queries** — Add `/artist/:id` and `/album/:id` routes plus dedicated IPC commands returning typed detail payloads.
   - Pros: Matches PRD/UI docs, keeps navigation explicit, fits current routes/features architecture, scalable for deep linking and future grouped search.
   - Cons: Requires new backend aggregation layer because current backend only exposes tracks.
   - Effort: Medium

2. **Reuse track-only search and synthesize details on the frontend** — Build artist/album pages by grouping existing `Track` results or local library entries client-side.
   - Pros: Faster initial UI spike, less backend work.
   - Cons: Violates Rust Source-of-Truth direction, weak for remote sources, no reliable artist/album identities, hard to support direct navigation and complete detail views.
   - Effort: Medium

### Recommendation
Use **route-driven detail pages with new backend entity queries**. The codebase is already organized around backend-owned state and typed IPC, so artist/album detail should be introduced as first-class backend queries instead of frontend-derived guesses. That also unlocks the PRD requirements for typed search blocks, artist result navigation, album playback, and Now Playing deep links without bending the architecture.

### Risks
- The current backend search contract is `Vec<Track>` only, so grouped search and entity detail views likely require new DTOs and command surfaces, not just UI work.
- Remote sources may not yet expose enough stable artist/album metadata for rich detail views; local files are better positioned because scanner metadata already captures artist/album fields.
- `ArtistCard.svelte` is still a stub, so UI implementation will need both data plumbing and new reusable presentation components.
- Documentation drift exists in `ARCHITECTURE.md`; proposal/design should explicitly reconcile doc intent with current implementation.

### Ready for Proposal
Yes — but the proposal should treat this as a **backend + IPC + routing + UI** change, not a route-only polish task. It should include: typed search result grouping, new artist/album detail commands, dynamic routes, frontend stores for entity/detail state, and wiring from Search/Now Playing into those views.
