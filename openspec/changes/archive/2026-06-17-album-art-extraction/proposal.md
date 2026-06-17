# Proposal: Album Art Extraction

## Intent

Local tracks display no album art despite `Track.thumbnail` existing in the model and UI placeholders already rendering it. Users see broken/empty album art everywhere. Symphonia 0.6 already exposes `Visual` data from `MetadataRevision` — we just aren't reading it.

## Scope

### In Scope
- Extract embedded album art via Symphonia `MetadataRevision.media.visuals` (fix revision consumption bug)
- Cache art to `~/.local/share/helix/art/{content_hash}.{ext}` filesystem
- Configure Tauri `assetProtocol` scope for serving cached art
- Frontend: `convertFileSrc()` in NowPlayingInfo, TrackList, AlbumCard

### Out of Scope
- Lofty fallback (documented as future path, not now)
- Image resizing/thumbnail generation (`image` crate)
- Cache cleanup/invalidation automation
- AlbumCard full implementation (beyond art rendering)

## Capabilities

### New Capabilities
- `album-art-cache`: Filesystem art cache management — write art bytes to content-hashed files, resolve extensions from `media_type`, create cache dir on startup

### Modified Capabilities
None (no existing specs in `openspec/specs/`)

## Approach

1. Fix `extract_metadata()` — consume ALL metadata revisions via `pop()`/`is_latest()` loop (current bug: only reads `current()`, misses art in newer revisions)
2. Iterate `revision.media.visuals`, find `StandardVisualKey::FrontCover`, extract raw `data` + `media_type`
3. Hash bytes (sha256), determine extension from `media_type`, write to `~/.local/share/helix/art/{hash}.{ext}`
4. Set `Track.thumbnail` to cache file path
5. Add `assetProtocol` scope in `tauri.conf.json` for `$APPDATA/art/**`
6. Frontend: `convertFileSrc(track.thumbnail)` → `<img src>`

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/sources/local/scanner.rs` | Modified | Fix revision loop, add visual extraction + cache write |
| `src-tauri/tauri.conf.json` | Modified | Add `assetProtocol` scope for art dir |
| `ui/src/features/player/components/NowPlayingInfo.svelte` | Modified | Use `convertFileSrc()` for cached art |
| `ui/src/shared/components/AlbumCard.svelte` | Modified | Render art from cache |
| `ui/src/shared/components/TrackList.svelte` | Modified | Add thumbnail column |
| `src-tauri/src/main.rs` or lib | Modified | Ensure art cache dir exists at startup |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Symphonia visuals missing for some formats | Med | Test with real MP3/FLAC/M4A/OGG; lofty is documented fallback |
| Large embedded art (4MB+) slows scan | Low | Defer: size limit or `image` crate resizing later |
| Metadata revision bug breaks existing tags | Low | Revision loop replaces single `current()` call; text tags still extracted from same revision |

## Rollback Plan

Revert scanner.rs to `metadata.current()` only (restore original behavior), remove `assetProtocol` config, set `thumbnail: None` again. Art cache dir is inert — no harm leaving it. Delete `~/.local/share/helix/art/` manually.

## Dependencies

- Symphonia 0.6 (already in Cargo.toml)
- Tauri v2 asset protocol (built-in, needs config)

## Success Criteria

- [ ] MP3/FLAC/M4A files with embedded art show album art in NowPlayingInfo
- [ ] All metadata revisions consumed (not just `current()`)
- [ ] Art files written to `~/.local/share/helix/art/` with correct extensions
- [ ] `convertFileSrc()` renders cached art as `<img src>` without base64
- [ ] Tracks without embedded art still show placeholder (no regression)