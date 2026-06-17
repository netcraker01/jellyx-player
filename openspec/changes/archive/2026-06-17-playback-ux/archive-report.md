# Archive Report: Playback UX

**Change**: playback-ux
**Archived**: 2026-06-17
**Mode**: hybrid (OpenSpec + Engram)

## Summary

Successfully archived the playback-ux change after verification pass. The change delivers:
- History recording for local, YouTube, and SoundCloud playback
- Persistent favorite toggle in Now Playing
- Backend-owned shuffle and repeat modes with queue preservation

**Post-verification fix applied**: Added `record_history()` call in `next()` and `previous()` for remote tracks (non-local path branch) to fix cross-source history recording.

## Artifacts Traced

| Artifact | Source | Observation ID |
|----------|--------|-----------------|
| Proposal | Engram | #558 |
| Tasks | Engram | #561 |
| Apply Progress | Engram | #562 |
| Verify Report | Engram | #563 |

## Spec Sync Summary

| Domain | Action | Details |
|--------|--------|---------|
| play-history | Created | 2 requirements: Record Playback Starts, Keep Bounded Recent History |
| favorites-management | Created | 2 requirements: Toggle Favorite State, Persist Favorite State |
| playback-modes | Created | 2 requirements: Preserve Queue Order During Shuffle, Cycle Repeat Modes |

## Archive Contents

- `proposal.md` ✅
- `specs/` ✅ (3 domain specs)
- `design.md` ✅
- `tasks.md` ✅ (15/15 tasks complete)
- `verify-report.md` ✅
- `apply-progress.md` ✅
- `exploration.md` ✅

## Source of Truth Updated

The following specs now reflect the new behavior:
- `openspec/specs/play-history/spec.md`
- `openspec/specs/favorites-management/spec.md`
- `openspec/specs/playback-modes/spec.md`

## Verification Notes

- Build: cargo check ✅, vite build ✅
- Tests: 188 Rust ✅, 26 Vitest ✅
- Post-verification fix: Added `record_history()` call in remote track path to ensure cross-source history recording works for both local and remote tracks.

## Lessons Learned

1. **Engram task artifact desync**: The Engram `sdd/playback-ux/tasks` observation went out of sync with the OpenSpec `tasks.md`. Future phases should read OpenSpec as the source of truth for task completion status.

2. **Cross-source history**: History recording was implemented for local tracks but not for remote (YouTube/SoundCloud) tracks. The fix adds `record_history()` to `next()` and `previous()` for the non-local path.

3. **Design deviation accepted**: `set_shuffle` and `set_repeat` return `()` and `cycle_repeat` returns `String`; frontend sync depends on `queue-updated` events rather than return payloads.

## SDD Cycle Complete

The change has been fully planned, implemented, verified, and archived.