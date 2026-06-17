# Design: Album Art Extraction

## Technical Approach

Fix the Symphonia metadata revision consumption bug in `ScannerService::extract_metadata()` by replacing the single `metadata.current()` call with a `pop()` loop until `is_latest()`. During iteration, extract `FrontCover` visuals from `revision.media.visuals`, hash the bytes with SHA-256, write to filesystem cache, and set `Track.thumbnail` to the cache path. Frontend uses `convertFileSrc()` to serve cached art via Tauri's asset protocol.

## Architecture Decisions

### Decision: Revision consumption strategy

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `skip_to_latest()` — jump to newest | Loses tags from earlier revisions | Rejected |
| `pop()` loop — consume each revision | More code, but no data loss | **Chosen** |

**Rationale**: `skip_to_latest()` discards all intermediate revisions — if ID3v2 tags exist in revision 1 and VorbisComment art in revision 2, we'd lose the tags. `pop()` loop lets us merge tags AND collect visuals from every revision. Per spec: "later revision overwrites earlier for same tag key."

### Decision: Art cache storage

| Option | Tradeoff | Decision |
|--------|----------|----------|
| SQLite BLOB in local_tracks | Simple query, bloated DB, no dedup | Rejected |
| Filesystem files keyed by content hash | Dedup by hash, DB stays lean | **Chosen** |

**Rationale**: Content-hash deduplication means two tracks with identical art share one file. Database stays small. The `thumbnail` field already stores a string — a filesystem path is a natural fit. Cache cleanup is `rm -rf ~/.local/share/helix/art/`.

### Decision: Image serving mechanism

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Base64 data URI in JSON | No protocol config, bloats IPC payloads | Rejected |
| Tauri asset protocol + `convertFileSrc()` | Requires scope config, zero-copy native | **Chosen** |

**Rationale**: Tauri v2's asset protocol serves files directly from disk into the webview. Zero-copy, no IPC overhead. Requires `assetProtocol.enable: true` + scope in `tauri.conf.json` and CSP `img-src` update. `convertFileSrc()` from `@tauri-apps/api/core` converts the path.

### Decision: SHA-256 hashing dependency

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `sha2` crate | Extra dependency, feature-rich | Rejected |
| `sha256` from `std` via `ring` | Extra dependency | Rejected |
| Rust `sha2` (already pulled by tauri) | No new dep, verify | Needs verification |

**Fallback**: If `sha2` is NOT already in the dependency tree, add it — it's tiny and universally used. Alternatively, use a simpler hash (blake3, etc.), but SHA-256 is the standard for content-addressable storage.

## Data Flow

```
Audio File
    │
    ▼
ScannerService::extract_metadata()
    │
    ├─ format_reader.probe()
    │
    ├─ metadata.pop() loop ──► consume ALL revisions
    │       │
    │       ├─ revision.media.tags ──► title, artist, album (merge across revisions)
    │       │
    │       └─ revision.media.visuals ──► find StandardVisualKey::FrontCover
    │               │
    │               ▼
    │           extract_visual()
    │               │
    │               ├─ sha256(visual.data) ──► content_hash
    │               ├─ media_type ──► extension (.jpg/.png/.bin)
    │               └─ write ~/.local/share/helix/art/{hash}.{ext}
    │               │
    │               ▼
    │           Track.thumbnail = cache_path
    │
    ▼
Database (upsert_local_track with thumbnail in track_json)
    │
    ▼
Frontend: convertFileSrc(track.thumbnail) ──► <img src>
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/sources/local/scanner.rs` | Modify | Replace `metadata.current()` with `pop()` loop; add `extract_visual()` and `cache_art()` functions; import `StandardVisualKey`, `sha2` |
| `src-tauri/src/app/setup.rs` | Modify | Add `ensure_art_cache_dir()` call before AppState creation |
| `src-tauri/src/shared/utils.rs` | Modify | Add `art_cache_dir()` and `ensure_art_cache_dir()` utility functions |
| `src-tauri/tauri.conf.json` | Modify | Add `assetProtocol` config with `$APPDATA/art/**` scope; update CSP `img-src` |
| `src-tauri/Cargo.toml` | Modify | Add `sha2` dependency if not already transitive |
| `ui/src/shared/utils/assetUrl.ts` | Create | Wrapper for `convertFileSrc()` from `@tauri-apps/api/core` |
| `ui/src/features/player/components/NowPlayingInfo.svelte` | Modify | Use `convertFileSrc(track.thumbnail)` for `<img src>` |
| `ui/src/shared/components/TrackList.svelte` | Modify | Add thumbnail `<img>` in track row using `convertFileSrc()` |
| `ui/src/shared/components/AlbumCard.svelte` | Modify | Implement art rendering with `convertFileSrc()` |

## Interfaces / Contracts

### Rust: `extract_visual` (private, in scanner.rs)

```rust
fn extract_visual(visuals: &[Visual]) -> Option<(&Box<[u8]>, &Option<String>)> {
    visuals.iter()
        .find(|v| v.usage == Some(StandardVisualKey::FrontCover))
        .map(|v| (&v.data, &v.media_type))
}
```

### Rust: `cache_art` (private, in scanner.rs)

```rust
fn cache_art(data: &[u8], media_type: &Option<String>) -> Result<PathBuf, ScannerError> {
    // hash data, derive ext, write to art_cache_dir() if not exists, return path
}
```

### Rust: `media_type_to_ext` (private helper)

```rust
fn media_type_to_ext(media_type: &Option<String>) -> &str {
    match media_type.as_deref() {
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        Some("image/png") => "png",
        _ => "bin",
    }
}
```

### TypeScript: `assetUrl.ts`

```typescript
import { convertFileSrc } from '@tauri-apps/api/core';

export function albumArtUrl(thumbnail: string | undefined): string | undefined {
  return thumbnail ? convertFileSrc(thumbnail) : undefined;
}
```

### Tauri config addition (`tauri.conf.json`)

```json
{
  "app": {
    "security": {
      "assetProtocol": {
        "enable": true,
        "scope": ["$APPDATA/art/**/*"]
      },
      "csp": "default-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost"
    }
  }
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `media_type_to_ext` mapping | Direct assertions for jpeg/png/unknown |
| Unit | `extract_visual` finds `FrontCover` | Mock `Vec<Visual>` with usage variants |
| Unit | `cache_art` writes file and returns path | Temp dir, verify file exists and content matches |
| Unit | `cache_art` dedup — same hash skips write | Two calls with identical bytes, one file |
| Unit | `pop()` loop merges tags across revisions | Mock `MetadataLog` with two revisions |
| Integration | Full scanner extracts art from real MP3/FLAC | Test files with embedded art in `tests/fixtures/` |
| E2E | Frontend renders `<img>` from `convertFileSrc()` | Visual check in dev mode |

## Migration / Rollout

No database schema change. Existing `local_tracks.track_json` stores `Track.thumbnail` as `None` — re-scanning a folder will upsert tracks with the new `thumbnail` value. Users must re-scan to populate art for existing libraries.

## Open Questions

- [ ] Is `sha2` already a transitive dependency of Tauri? If not, confirm adding it to `Cargo.toml`.
- [ ] Should `AlbumCard.svelte` be fully implemented in this change or remain a stub with just art rendering?