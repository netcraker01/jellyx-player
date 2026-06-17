# Exploration: Album Art Extraction

## Current State

Helix Player's `ScannerService` extracts text metadata (title, artist, album, duration) from audio files using Symphonia 0.6's `probe()` API. It iterates `StandardTag` variants (`TrackTitle`, `Album`, `Artist`) from `metadata.current().media.tags`. **Album art extraction is completely absent** — the `Track.thumbnail` field is always set to `None` for local files.

The frontend already has UI placeholders for album art:
- `NowPlayingInfo.svelte` renders `<img class="album-art" src={track.thumbnail}>` or a placeholder div when no thumbnail exists
- `TrackList.svelte` and `AlbumCard.svelte` (stub) have no art rendering yet
- The `Track` model (Rust + TypeScript) already has a `thumbnail: Option<String>` field — this is the natural place to store an art reference

## Affected Areas

- `src-tauri/src/sources/local/scanner.rs` — must extract art alongside metadata during scan
- `src-tauri/src/models/track.rs` — `thumbnail` field already exists, may need to hold art cache path or asset URL
- `src-tauri/src/persistence/db.rs` — schema may need `album_art` column or new table for art cache
- `src-tauri/src/persistence/models.rs` — may need `AlbumArtEntry` or art_path field on `LocalTrackEntry`
- `src-tauri/src/ipc/commands.rs` — may need `get_album_art` command if serving on-demand
- `src-tauri/Cargo.toml` — may need `lofty` dependency (if chosen over Symphonia for art)
- `src-tauri/tauri.conf.json` — must configure `assetProtocol` scope for serving cached art files
- `ui/src/shared/types/models.ts` — `Track.thumbnail` already exists
- `ui/src/shared/components/AlbumCard.svelte` — stub needs implementation
- `ui/src/features/player/components/NowPlayingInfo.svelte` — already renders thumbnail
- `ui/src/routes/Library/Page.svelte` — track table could show art thumbnails

## Approaches

### 1. Symphonia-only (Visual from MetadataRevision) — Extend existing probe

Use Symphonia 0.6's `MetadataRevision.media.visuals` which already contains `Visual` structs with `data: Box<[u8]>`, `media_type`, and `usage: Option<StandardVisualKey>`. The `StandardVisualKey::FrontCover` variant maps directly to album art.

- Pros:
  - No new dependency — Symphonia already in Cargo.toml
  - Consistent with current metadata extraction flow (same `probe()` call)
  - `StandardVisualKey::FrontCover` gives semantic meaning
  - Works for MP3 (ID3v2 APIC), FLAC (METADATA_BLOCK_PICTURE), OGG (Vorbis Comment), M4A (MP4 ilst)
- Cons:
  - Symphonia's Visual extraction reliability varies by format — some formats may not parse visuals
  - No write support (read-only, which is fine for this use case)
  - Need to consume all metadata revisions (current code only reads `current()`, but visuals may be in newer revisions)
  - Larger art images will slow the probe step
- Effort: Low-Medium

### 2. Lofty crate — Dedicated metadata library

Replace or supplement Symphonia metadata extraction with `lofty`, a crate purpose-built for audio metadata parsing. Lofty has first-class `Picture` support with `PictureType::CoverFront`, `MimeType`, and raw `data()` access.

- Pros:
  - Purpose-built for metadata — richer, more reliable art extraction across formats
  - `TagExt::pictures()` gives direct access to embedded images
  - Supports MP3, FLAC, OGG, M4A, WAV, AIFF, APE, WavPack, and more
  - `PictureType::CoverFront` matches our exact use case
  - `MimeType` enum for correct content-type detection
  - Can extract art WITHOUT decoding the full audio — much faster for large files
  - Active maintenance, good docs
- Cons:
  - New dependency (~40 additional crates via transitive deps)
  - Partial overlap with Symphonia for text metadata (artist, title, album) — need to decide: replace Symphonia metadata entirely or use both?
  - If we keep both Symphonia (playback) + Lofty (metadata), we open the file TWICE per track during scan
  - Lofty cannot decode audio — Symphonia still required for playback
- Effort: Low-Medium (for art-only addition), Medium (if replacing Symphonia metadata)

### 3. Hybrid — Lofty for art, Symphonia for playback/text metadata

Use Lofty specifically for album art extraction during scanning. Keep Symphonia for audio playback and text metadata (since the probe is already working). Scanner opens file with Lofty to get art, then with Symphonia for text metadata + duration.

- Pros:
  - Best art extraction reliability (Lofty's specialty)
  - No risk of breaking existing metadata extraction
  - Lofty can extract art from formats Symphonia visuals may miss
- Cons:
  - Two file opens per track during scan (performance cost)
  - Two metadata dependencies to maintain
  - Could optimize by checking if Symphonia found visuals first, falling back to Lofty
- Effort: Medium

## Storage Approaches

### A. Filesystem cache with path in DB

Extract art during scan, save to `~/.local/share/helix/art/{hash}.jpg`, store the path in the `Track.thumbnail` field or a new `album_art_path` column.

- Pros:
  - No BLOB bloat in SQLite (art images are 100KB-2MB each)
  - Can be served via Tauri asset protocol (`convertFileSrc()`) — native `<img>` tag
  - Cache can be cleaned independently of database
  - Deduplication by content hash (same art for all tracks in an album)
- Cons:
  - Must manage cache directory lifecycle (create, clean, migrate)
  - Path references break if cache moved
  - Need to handle multiple art formats (PNG, JPEG, WebP) — save with correct extension

### B. BLOB in SQLite

Store raw image bytes in a new `album_art` BLOB column in `local_tracks` table.

- Pros:
  - Simple — single database file contains everything
  - No filesystem management
  - Atomic with track data (delete track = delete art)
- Cons:
  - Database bloat — 1000 tracks × 500KB art = ~500MB SQLite file
  - WAL mode helps with concurrent reads, but large BLOBs still problematic
  - Must serve via IPC command returning base64 — expensive per-track
  - No native `<img src>` rendering without base64 data URI

### C. On-demand extraction (no cache)

Don't extract during scan. Extract art on-demand when the frontend requests it via an IPC command.

- Pros:
  - Scan is fast — no art processing
  - No storage overhead until art is actually viewed
  - Art always matches file (no stale cache)
- Cons:
  - Slow first load — must open and parse file for each art request
  - Frontend must request art per-track, no batch
  - No pre-computed thumbnails for library views
  - Album grid view would be painfully slow

## IPC/Serving Approaches

### i. Tauri Asset Protocol (recommended with cache approach A)

Save art to filesystem cache, configure `assetProtocol` in `tauri.conf.json`, use `convertFileSrc()` on frontend to create `<img src>` URLs.

- Pros: Native browser image loading, caching by WebView, no IPC overhead per render
- Cons: Must configure CSP and scope in tauri.conf.json

### ii. Tauri Command returning base64

New `get_album_art(track_id)` command returns base64 data URI. Frontend sets `track.thumbnail = "data:image/jpeg;base64,..."`.

- Pros: Works without asset protocol configuration
- Cons: Large payloads through IPC, no browser cache, base64 overhead (~33% size increase), re-sent on every render

### iii. Custom Protocol Handler

Register a custom `helix-art://` protocol handler in Tauri that serves art from cache.

- Pros: Full control, can implement thumbnails/resizing server-side
- Cons: More complex implementation, Tauri v2 custom protocol API is less documented

## Recommendation

**Use Approach 1 (Symphonia Visuals) as primary + Storage A (filesystem cache) + IPC i (asset protocol).**

Rationale:
1. **Symphonia already parses the file** — `MetadataRevision.media.visuals` contains `Visual` structs with raw bytes. We're already doing `probe()` for text metadata. Adding `visuals` iteration is a few lines of code, zero new deps.
2. **Filesystem cache is the right choice** — art images are 100KB-2MB each. Putting them in SQLite would bloat the database catastrophically. Cache files served via asset protocol give native `<img src>` with browser-level caching.
3. **Asset protocol is native Tauri** — `convertFileSrc()` works with standard `<img>` tags. The frontend already expects `track.thumbnail` to be a URL/path. No base64 overhead, no custom protocol complexity.
4. **Add Lofty as fallback LATER if needed** — if Symphonia visuals are insufficient for certain formats, we can add Lofty incrementally. Don't add a second dep prematurely.

Implementation plan:
1. Extend `extract_metadata()` in scanner.rs to iterate `revision.media.visuals`, find `StandardVisualKey::FrontCover`, write raw bytes to `~/.local/share/helix/art/{sha256}.{ext}`
2. Set `Track.thumbnail` to the cache file path
3. Add `assetProtocol` config to `tauri.conf.json` with scope `"$APPDATA/art/**"`
4. Frontend uses `convertFileSrc(track.thumbnail)` for `<img src>`
5. Schema migration v3: no DB changes needed — `thumbnail` is already in `track_json`

## Risks

- **Symphonia Visual reliability**: Some format readers may not populate `visuals`. Need to test with real MP3/FLAC/M4A/OGG files that have embedded art. If Symphonia misses art frequently, lofty fallback becomes mandatory.
- **Large art files**: Some tracks embed 4MB+ cover art. Need size limits and/or thumbnail generation. Could add `image` crate for resizing later.
- **Concurrent scan + art write**: Multiple scans writing to cache dir simultaneously — use hash-based filenames (content-addressed) to avoid conflicts.
- **Cache invalidation**: When a file's art changes (re-tagged), the old cache is orphaned. Content-hash filenames mean new art gets a new hash; old file stays until manual cleanup.
- **CSP configuration**: Asset protocol requires CSP changes in `tauri.conf.json`. Current config has `"csp": null` (permissive), but production builds will need specific `img-src` directives.
- **Metadata revision consumption**: Current scanner only reads `metadata.current()`. Symphonia docs say to consume revisions via `pop()` until `is_latest()`. Art might be in a newer revision that isn't reached.

## Ready for Proposal

Yes — the exploration is complete. Key decisions for the proposal:
1. Symphonia visuals as primary extraction (with lofty as documented fallback path)
2. Filesystem cache at `~/.local/share/helix/art/` with content-hash filenames
3. Asset protocol serving via `convertFileSrc()`
4. No database schema changes needed
5. Need to test Symphonia visuals with real files before committing to approach