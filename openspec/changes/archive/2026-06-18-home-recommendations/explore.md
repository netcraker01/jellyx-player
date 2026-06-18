## Exploration: home-recommendations

### Current State
The Home page is no longer pure stub content, but it is still MVP-incomplete. `ui/src/routes/Home/Page.svelte` fetches `getHistory()` on mount and renders only one section: **Recently Played**. If history is empty, it shows a centered empty state with a Search CTA linking to `/search`. If history exists, it renders a simple inline list of history rows with play button, title, artist, duration, and source badge. There is no recommendations/discover section, no genre/mood shortcuts, no reusable store abstraction for Home data, and no backend command that computes recommendations.

### Affected Areas
- `ui/src/routes/Home/Page.svelte` — current Home implementation; likely needs layout and state refactor.
- `ui/src/services/commands.ts` — exposes available IPC wrappers (`getHistory`, `getFavorites`, `getLocalTracks`, etc.).
- `src-tauri/src/ipc/commands.rs` — current backend command surface; no recommendation-specific command exists.
- `src-tauri/src/library/service.rs` — library/history business logic; likely place to assemble recommendation inputs.
- `src-tauri/src/persistence/models.rs` — confirms available persisted shapes: `FavoriteEntry`, `HistoryEntry`, `LocalTrackEntry`, `WatchedFolder`.
- `ui/src/features/favorites/stores/favorites.ts` — existing IPC-backed favorites store pattern.
- `ui/src/features/library/stores/library.ts` — existing IPC-backed local library store pattern.
- `ui/src/features/search/stores/searchGrouped.ts` — good reference for page-specific async store pattern.
- `ui/src/shared/components/TrackRow.svelte` / `TrackList.svelte` — reusable track list UIs.
- `ui/src/shared/components/ArtistCard.svelte` / `AlbumCard.svelte` — reusable card UIs for recommendation/discover blocks.
- `ui/src/features/search/components/GroupedResults.svelte` — demonstrates mixed sections (tracks + cards) in one page.
- `docs/PRD.md` — product requirements for Home.
- `docs/UI_DESIGN.md` — layout expectations for Home as central content area.
- `docs/ARCHITECTURE.md` — backend-source-of-truth and frontend IPC data flow constraints.

### Approaches
1. **Frontend composition from existing commands** — Build Home by calling `getHistory`, `getFavorites`, and `getLocalTracks` from Svelte, then derive sections client-side.
   - Pros: Minimal backend work; fastest path; uses existing commands only.
   - Cons: No true recommendation API; duplicates ranking logic in UI; weak long-term architecture because Home becomes smarter than the "dumb client" principle.
   - Effort: Medium

2. **Backend-driven Home snapshot command** — Add a Rust command/service method that returns a Home payload (recently played + derived recommendations/discover + optional genres/moods), then render it from a dedicated Home store.
   - Pros: Matches architecture docs; centralizes ranking/derivation; easier to evolve later; cleaner testing boundary.
   - Cons: More backend work up front; requires defining new DTOs and heuristics.
   - Effort: Medium

### Recommendation
Use **Backend-driven Home snapshot command**. The architecture explicitly says Rust is the source of truth and Svelte should behave like a dumb client. Home recommendations are derived domain data, not just presentation state. A new backend DTO/command should aggregate existing sources:

- `history` for recently played and recency signals,
- `favorites` for affinity signals,
- `local tracks` for recommendation inventory,
- optionally grouped artist/album derivation already present in `LibraryService` patterns.

Then expose it through a dedicated `ui/src/features/home/stores/...` store and render reusable sections using `TrackRow`/`TrackList`, `ArtistCard`, and `AlbumCard`.

### Risks
- There is **no existing recommendation model or command**, so heuristics must be defined in the change proposal/spec.
- Current available indexed recommendation inventory is mostly **local-library-driven**; streaming catalogs are not represented in current library/history APIs.
- There is **no history store** today; Home currently fetches history directly inside the page, so implementation should avoid growing page-level async logic.
- `UI_DESIGN.md` gives only high-level layout guidance for Home, not concrete section visuals, so the change must define section composition and card/list balance.

### Ready for Proposal
Yes — enough evidence exists to propose the change. The orchestrator should tell the user that the codebase already has the raw inputs for a first MVP recommendation/discover experience (history, favorites, local tracks), but it still lacks a backend Home aggregation API and a dedicated Home store/component structure.
