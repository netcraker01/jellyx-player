# Technical Design: Home Recommendations

## Architecture Decisions

### AD-1: Single Home snapshot command
- **Decision**: One `get_home_snapshot` command returns the entire Home payload in a single call.
- **Rationale**: Eliminates frontend orchestration of multiple IPC calls; Rust assembles all sections server-side. Matches ARCHITECTURE.md "dumb client" principle.
- **Tradeoff**: Larger payload per call, but Home is the most visited page and the data is bounded (max 20 recently played + 20 recommendations).

### AD-2: Deterministic recommendation heuristics (no ML)
- **Decision**: Recommendations are derived from simple, explainable heuristics: recently engaged artists/albums from history, favorited tracks' artists, and random local-library discovery.
- **Rationale**: No ML infrastructure, no external APIs. Local-library data is the only inventory. Deterministic results are reproducible and debuggable.
- **Heuristics**:
  1. **Artist affinity**: Artists with most plays in recent history → their unplayed local tracks
  2. **Album affinity**: Albums with multiple plays → other tracks from those albums
  3. **Favorite discovery**: Favorite tracks' artists → other tracks by same artists not recently played
  4. **Library discovery**: Random tracks from local library not in recent history (fallback)

### AD-3: Section-based payload with mixed item types
- **Decision**: HomeSnapshot contains `recently_played` (HistoryEntry[]) and `recommendations` (RecommendationItem[]). RecommendationItem can be a track, artist, or album — discriminated by a `type` field.
- **Rationale**: Recommendations may suggest exploring an artist or album, not just playing a track. The type field lets the frontend render appropriate cards.
- **Payload limit**: 20 recently played entries, 20 recommendation items.

### AD-4: Home store replaces inline data fetching
- **Decision**: Replace the current inline `getHistory()` call in Home.svelte with a dedicated `homeStore` that calls `getHomeSnapshot` once on mount.
- **Rationale**: Consistent with other feature stores (searchGrouped, artistDetail, albumDetail). Centralizes loading/error state.

## New Data Structures

### Rust DTOs (in src-tauri/src/ipc/dto.rs)

```rust
/// A single recommendation item, which may be a track, artist, or album.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum RecommendationItem {
    Track { track: Track, reason: String },
    Artist { id: String, name: String, thumbnail: Option<String>, track_count: u32, reason: String },
    Album { id: String, title: String, artist: String, cover: Option<String>, track_count: u32, reason: String },
}

/// Home snapshot returned by get_home_snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSnapshot {
    pub recently_played: Vec<HistoryEntry>,
    pub recommendations: Vec<RecommendationItem>,
}
```

### TypeScript types (in ui/src/shared/types/models.ts)

```typescript
export type RecommendationItem =
  | { type: 'Track'; track: Track; reason: string }
  | { type: 'Artist'; id: string; name: string; thumbnail?: string; trackCount: number; reason: string }
  | { type: 'Album'; id: string; title: string; artist: string; cover?: string; trackCount: number; reason: string };

export interface HomeSnapshot {
  recentlyPlayed: HistoryEntry[];
  recommendations: RecommendationItem[];
}
```

## Backend Command Design

### New Tauri command (in src-tauri/src/ipc/commands.rs)

```rust
/// Get the Home snapshot with recently played and recommendations.
#[tauri::command]
pub fn get_home_snapshot(state: tauri::State<AppState>) -> Result<HomeSnapshot, AppError>
```

### Implementation in LibraryService

```rust
impl LibraryService {
    /// Build the Home snapshot: recently played + recommendations.
    pub fn get_home_snapshot(&self) -> Result<HomeSnapshot, AppError> {
        let history = self.db.get_history()?;
        let favorites = self.db.get_favorites()?;
        let local_tracks = self.db.get_all_local_tracks()?;

        let recently_played = history.iter().take(20).cloned().collect();

        let recommendations = self.build_recommendations(
            &history,
            &favorites,
            &local_tracks,
        );

        Ok(HomeSnapshot {
            recently_played,
            recommendations,
        })
    }

    fn build_recommendations(
        &self,
        history: &[HistoryEntry],
        favorites: &[FavoriteEntry],
        local_tracks: &[LocalTrackEntry],
    ) -> Vec<RecommendationItem> {
        // 1. Artist affinity: artists with most recent plays
        // 2. Album affinity: albums with multiple plays
        // 3. Favorite discovery: favorite artists' other tracks
        // 4. Library discovery: random local tracks not recently played
        // ... (deterministic heuristics)
    }
}
```

### Registration (in src-tauri/src/app/setup.rs)

Add to invoke_handler: `crate::ipc::commands::get_home_snapshot`

### New DB method needed (in src-tauri/src/persistence/db.rs)

```rust
/// Get all local tracks (for recommendation inventory).
pub fn get_all_local_tracks(&self) -> Result<Vec<LocalTrackEntry>, rusqlite::Error>
```

## Recommendation Heuristics (Detail)

1. **Artist affinity** (max 8 items):
   - Count artist occurrences in recent 100 history entries
   - For top artists (by play count), find their local tracks NOT in recent 20 plays
   - Present as `Artist` recommendation items with reason "Because you listened to {artist}"

2. **Album affinity** (max 4 items):
   - Find albums where ≥2 tracks appear in recent history
   - Present as `Album` recommendation items with reason "Based on your listening"

3. **Favorite discovery** (max 4 items):
   - Find favorite tracks' artists
   - Find other local tracks by those artists not recently played
   - Present as `Track` items with reason "From your favorites"

4. **Library discovery** (max 4 items):
   - Random selection from local tracks not recently played
   - Deterministic random seed based on current date
   - Present as `Track` items with reason "Discover from your library"

Total: max 20 recommendations. If signals are weak, library discovery fills the gap.

## Frontend Store Design

### New store: ui/src/features/home/stores/home.ts

```typescript
interface HomeStore {
  subscribe: Readable<HomeSnapshot | null>['subscribe'];
  loading: Readable<boolean>;
  error: Readable<string | null>;
  load(): Promise<void>;
  clear(): void;
}
```

- Calls `commands.getHomeSnapshot()` on `load()`
- Replaces the inline `getHistory()` call in Home.svelte

## Component Design

### Modified: ui/src/routes/Home/Page.svelte

- Replace inline history fetching with `homeStore.load()`
- Render `recentlyPlayed` section using existing TrackRow pattern
- Render `recommendations` section with type-specific rendering:
  - Track items → TrackRow with play action
  - Artist items → ArtistCard with navigation to /artist/:id
  - Album items → AlbumCard with navigation to /album/:id
- Show reason labels next to recommendation items
- Empty state when both sections are empty (Search CTA)
- Loading state while fetching

### Section layout

```
┌─────────────────────────────────┐
│ Home                            │
├─────────────────────────────────┤
│ Recently Played                 │
│ ─ track row (play/artist/album) │
│ ─ track row                     │
│ ...                             │
├─────────────────────────────────┤
│ Recommended for You             │
│ ─ Artist card → /artist/:id     │
│ ─ Album card → /album/:id      │
│ ─ Track row (play)              │
│ ...                             │
└─────────────────────────────────┘
```

## Data Flow

```
Home page mount → homeStore.load()
→ Tauri IPC → get_home_snapshot command
→ LibraryService.get_home_snapshot()
  → db.get_history() (recently played, max 20)
  → db.get_favorites() (affinity signals)
  → db.get_all_local_tracks() (recommendation inventory)
  → build_recommendations() (deterministic heuristics)
→ HomeSnapshot serialized → Svelte store updated
→ Home page renders sections
```

## Error Handling

- **Empty history**: recently_played is empty array, recommendations may still have library discovery items
- **Empty everything**: Both sections empty, show Search CTA
- **DB error**: Surface as toast via notification system, show empty state
- **Partial data**: Show whatever sections have data, degrade gracefully