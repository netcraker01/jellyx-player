# Technical Design: Artist/Album Detail Views

## Architecture Decisions

### AD-1: Grouped search replaces flat Vec<Track>
- **Decision**: The existing `search(query) -> Vec<Track>` command is replaced by `search_grouped(query, filter?) -> GroupedSearchResult` returning separate `songs`, `artists`, and `albums` collections.
- **Rationale**: The PRD requires typed result blocks with navigation. Keeping the flat API would force the frontend to synthesize entities, violating the Rust-as-Source-of-Truth principle.
- **Migration**: The old `search` command remains functional for backward compatibility, but the frontend will switch to `search_grouped`.

### AD-2: Artist and Album detail are backend-first IPC commands
- **Decision**: New `get_artist_detail(id) -> ArtistDetail` and `get_album_detail(id) -> AlbumDetail` commands return aggregated payloads from the SQLite database.
- **Rationale**: Detail views need tracks, metadata, and relationships (artist's albums, album's tracks). Building these client-side from `Vec<Track>` would duplicate logic and break offline/reload resilience.
- **Data source**: Local scanner metadata is the primary source. Remote source enrichment is out of scope.

### AD-3: Artist/Album IDs are derived from Track metadata
- **Decision**: Artist IDs are `artist:{name}` (lowercase, normalized) and Album IDs are `album:{name}:{artist}` (for local tracks). These deterministic IDs allow direct navigation without a separate entity table.
- **Rationale**: The local scanner already populates `artist` and `album` string fields on each Track. Adding a separate artists/albums table is a future enhancement; for MVP, deterministic IDs from metadata strings are sufficient and avoid schema migration complexity.
- **Edge case**: Two artists with the same name will share an ID. Acceptable for MVP.

### AD-4: Dynamic routes in svelte-routing
- **Decision**: Add `/artist/:id` and `/album/:id` routes to App.svelte using svelte-routing's `Route` component with `let:params`.
- **Rationale**: Consistent with existing routing pattern. svelte-routing supports dynamic segments natively.

### AD-5: Play-album queues all album tracks in order
- **Decision**: `play_album(album_id)` command replaces the queue with the album's tracks in album order and starts playback from the first track.
- **Rationale**: Matches PRD requirement for "play full album". Using a dedicated command avoids manual queue manipulation from the frontend.

## New Data Structures

### Rust DTOs (in `src-tauri/src/ipc/dto.rs` or inline in commands)

```rust
/// Grouped search result returned by `search_grouped`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupedSearchResult {
    pub songs: Vec<Track>,
    pub artists: Vec<ArtistSummary>,
    pub albums: Vec<AlbumSummary>,
}

/// Lightweight artist summary for search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    pub thumbnail: Option<String>,
    pub track_count: u32,
}

/// Lightweight album summary for search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub cover: Option<String>,
    pub year: Option<u32>,
    pub track_count: u32,
}

/// Full artist detail for `/artist/:id` view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub id: String,
    pub name: String,
    pub thumbnail: Option<String>,
    pub top_tracks: Vec<Track>,
    pub albums: Vec<AlbumSummary>,
}

/// Full album detail for `/album/:id` view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    pub cover: Option<String>,
    pub year: Option<u32>,
    pub tracks: Vec<Track>,
}
```

### TypeScript types (in `ui/src/shared/types/models.ts`)

```typescript
export interface GroupedSearchResult {
  songs: Track[];
  artists: ArtistSummary[];
  albums: AlbumSummary[];
}

export interface ArtistSummary {
  id: string;
  name: string;
  thumbnail?: string;
  trackCount: number;
}

export interface AlbumSummary {
  id: string;
  title: string;
  artist: string;
  cover?: string;
  year?: number;
  trackCount: number;
}

export interface ArtistDetail {
  id: string;
  name: string;
  thumbnail?: string;
  topTracks: Track[];
  albums: AlbumSummary[];
}

export interface AlbumDetail {
  id: string;
  title: string;
  artist: string;
  artistId: string;
  cover?: string;
  year?: number;
  tracks: Track[];
}
```

## Backend Command Design

### New Tauri commands (in `src-tauri/src/ipc/commands.rs`)

```rust
/// Search with grouped results (songs, artists, albums).
/// Optional filter: "songs", "artists", "albums", or None for all.
#[tauri::command]
pub fn search_grouped(
    state: tauri::State<AppState>,
    query: &str,
    filter: Option<&str>,
) -> Result<GroupedSearchResult, AppError>

/// Get full artist detail by artist ID.
#[tauri::command]
pub fn get_artist_detail(
    state: tauri::State<AppState>,
    id: &str,
) -> Result<ArtistDetail, AppError>

/// Get full album detail by album ID.
#[tauri::command]
pub fn get_album_detail(
    state: tauri::State<AppState>,
    id: &str,
) -> Result<AlbumDetail, AppError>

/// Play all tracks in an album, replacing the current queue.
#[tauri::command]
pub fn play_album(
    state: tauri::State<AppState>,
    album_id: &str,
) -> Result<(), AppError>
```

### Implementation approach

- `search_grouped`: Queries local DB for matching tracks, then groups by artist/album to build summaries. Uses `LibraryService` or direct DB queries.
- `get_artist_detail`: Resolves artist ID → name, queries all tracks by that artist, computes top tracks (by play count from history), and lists distinct albums.
- `get_album_detail`: Resolves album ID → title + artist, queries all tracks for that album, ordered by metadata or filename.
- `play_album`: Gets album detail, replaces queue with album tracks, starts playback from first track.

### ID scheme for local entities

- Artist ID: `artist:{normalized_name}` where normalized_name = lowercase, trimmed, spaces→hyphens
- Album ID: `album:{normalized_title}:{normalized_artist}`
- These IDs are stable and derivable from Track metadata without a separate table.

### Registration (in `src-tauri/src/app/setup.rs`)

Add to `invoke_handler`:
```rust
crate::ipc::commands::search_grouped,
crate::ipc::commands::get_artist_detail,
crate::ipc::commands::get_album_detail,
crate::ipc::commands::play_album,
```

## Frontend Store Design

### New store: `ui/src/features/search/stores/searchGrouped.ts`

```typescript
interface SearchGroupedStore {
  subscribe: Readable<GroupedSearchResult | null>['subscribe'];
  search(query: string, filter?: SearchFilter): Promise<void>;
  clear(): void;
}
```

Replaces the existing flat `searchResults` store in Search page. Uses `commands.searchGrouped()`.

### New store: `ui/src/features/library/stores/artistDetail.ts`

```typescript
interface ArtistDetailStore {
  subscribe: Readable<ArtistDetail | null>['subscribe'];
  loading: Readable<boolean>;
  error: Readable<string | null>;
  load(id: string): Promise<void>;
  clear(): void;
}
```

### New store: `ui/src/features/library/stores/albumDetail.ts`

```typescript
interface AlbumDetailStore {
  subscribe: Readable<AlbumDetail | null>['subscribe'];
  loading: Readable<boolean>;
  error: Readable<string | null>;
  load(id: string): Promise<void>;
  clear(): void;
}
```

## Routing Design

### App.svelte additions

```svelte
<Route path="/artist/:id" let:params>
  <ArtistPage id={params.id} />
</Route>
<Route path="/album/:id" let:params>
  <AlbumPage id={params.id} />
</Route>
```

### New route files

- `ui/src/routes/Artist/Page.svelte` — loads artist detail, renders header + top tracks + albums
- `ui/src/routes/Album/Page.svelte` — loads album detail, renders header + track list + play-album button

## Component Design

### New components

- `ui/src/shared/components/ArtistCard.svelte` — update stub to show name, thumbnail, track count; click navigates to `/artist/:id`
- `ui/src/shared/components/AlbumCard.svelte` — update to show cover, title, artist, track count; click navigates to `/album/:id`
- `ui/src/features/search/components/GroupedResults.svelte` — replaces flat ResultsList; renders songs, artists, albums in separate sections with type filter tabs
- `ui/src/routes/Artist/Page.svelte` — artist header with image placeholder, top tracks list, albums grid
- `ui/src/routes/Album/Page.svelte` — album header with cover, artist link, track list, play-album button

### Modified components

- `ui/src/routes/Search/Page.svelte` — use `GroupedResults` instead of `ResultsList`; add type filter
- `ui/src/features/player/components/NowPlayingInfo.svelte` — add artist/album links that navigate to detail pages
- `ui/src/routes/NowPlaying/Page.svelte` — wire open-artist/open-album actions

## Data Flow

### Search → Grouped Results
```
User types query → searchGrouped store calls commands.searchGrouped(query, filter)
→ Tauri IPC → search_grouped command → LibraryService.search_grouped()
→ SQLite query: LIKE %query% on tracks, group by artist/album
→ GroupedSearchResult serialized → Svelte store updated → GroupedResults renders
```

### Open Artist Detail
```
User clicks artist card → navigate to /artist/:id
→ ArtistPage onMount calls artistDetailStore.load(id)
→ Tauri IPC → get_artist_detail command → LibraryService.get_artist_detail(id)
→ SQLite: SELECT tracks WHERE artist = ?, group albums, compute top tracks from history
→ ArtistDetail serialized → Svelte store updated → page renders
```

### Play Album
```
User clicks "Play Album" → commands.playAlbum(albumId)
→ Tauri IPC → play_album command → get album tracks → replace queue → start playback
→ PlaybackService replaces queue, emits queue-updated + track-changed events
```

## Error Handling

- **Unknown artist/album ID**: Return `NOT_FOUND` error code. Frontend shows "Artist not found" / "Album not found" with a back button.
- **Empty search results**: Return empty groups (no error). Frontend shows "No results" message.
- **No image/thumbnail**: Frontend renders a placeholder icon (same pattern as album art extraction).
- **Queue replace failure on play_album**: Surface as toast error via existing notification system.