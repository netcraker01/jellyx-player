## Verification Report

**Change**: playback-ux
**Version**: N/A
**Mode**: Standard

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 15 |
| Tasks complete | 15 |
| Tasks incomplete | 0 |

Notes:
- `openspec/changes/playback-ux/tasks.md` shows all 15 tasks complete.
- Engram task artifact `#561 sdd/playback-ux/tasks` is stale and still shows Phase 3/4 unchecked.

### Build & Tests Execution
**Build**: ✅ Passed
```text
Command: npx vite build (workdir: ui/)
Result: ✓ built in 21.24s
Note: Vite reported a non-blocking dynamic import chunking warning for @tauri-apps/api/core.js.
```

**Tests**: ✅ 214 passed / ❌ 0 failed / ⚠️ 0 skipped
```text
Command: cargo test --lib (workdir: src-tauri/)
Result: 188 passed; 0 failed

Command: npx vitest run (workdir: ui/)
Result: 26 passed; 0 failed
```

**Coverage**: ➖ Not available

### Spec Compliance Matrix
| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| Record Playback Starts | Record cross-source start | `src-tauri/src/playback/service.rs` + `playback::service::tests::playback_error_play_returns_platform_not_supported` | ❌ FAILING |
| Record Playback Starts | Ignore seek and resume | (none found) | ❌ UNTESTED |
| Keep Bounded Recent History | Return recent entries | `src-tauri/src/persistence/db.rs > history_ordered_by_played_at_desc` | ✅ COMPLIANT |
| Keep Bounded Recent History | Evict oldest entry | `src-tauri/src/persistence/db.rs > history_evicts_oldest_at_101st_entry` | ✅ COMPLIANT |
| Toggle Favorite State | Add favorite from Now Playing | `src-tauri/src/library/service.rs > toggle_favorite_adds_when_not_favorited` | ⚠️ PARTIAL |
| Toggle Favorite State | Remove existing favorite | `src-tauri/src/library/service.rs > toggle_favorite_removes_when_favorited` | ⚠️ PARTIAL |
| Persist Favorite State | Restore persisted favorite | (none found) | ❌ UNTESTED |
| Persist Favorite State | Keep unique favorite entry | `src-tauri/src/persistence/db.rs > duplicate_favorite_rejected` | ✅ COMPLIANT |
| Preserve Queue Order During Shuffle | Shuffle picks next without reordering | `src-tauri/src/playback/service.rs > shuffle_next_track_picks_unplayed_indices` | ⚠️ PARTIAL |
| Preserve Queue Order During Shuffle | Exhaust shuffled queue | `src-tauri/src/playback/service.rs > shuffle_next_track_returns_none_when_exhausted_and_repeat_off` | ⚠️ PARTIAL |
| Cycle Repeat Modes | Repeat all loops queue | `src-tauri/src/playback/service.rs > sequential_next_index_respects_repeat_modes` | ⚠️ PARTIAL |
| Cycle Repeat Modes | Repeat one replays current track | `src-tauri/src/playback/service.rs > sequential_next_index_respects_repeat_modes` | ⚠️ PARTIAL |

**Compliance summary**: 3/12 scenarios compliant

### Correctness (Static Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| Record Playback Starts | ❌ Incomplete | History is recorded only in `PlaybackService::play_local()`; `play()` still returns platform-not-supported and non-local `next()` paths do not record history. |
| Keep Bounded Recent History | ✅ Implemented | `Database::insert_history()` evicts oldest overflow rows and `get_history()` reads newest first with a 100-item cap. |
| Toggle Favorite State | ✅ Implemented | Atomic backend toggle exists in `LibraryService::toggle_favorite()` and is exposed through `toggle_favorite` IPC plus frontend heart control/store wiring. |
| Persist Favorite State | ✅ Implemented (static) | Favorites come from SQLite-backed commands, `favorites.load()` hydrates UI state, and current-track heart state is derived from persisted favorites. |
| Preserve Queue Order During Shuffle | ✅ Implemented | `QueueState.tracks` remains untouched while `shuffle_next_track()` selects by index and `Queue.svelte` renders original order. |
| Cycle Repeat Modes | ✅ Implemented | `cycle_repeat()` follows Off → All → One → Off and queue snapshots keep frontend state in sync. |

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| Inject LibraryService into PlaybackService | ✅ Yes | `PlaybackService::new()` receives `Arc<LibraryService>` from `app/setup.rs`. |
| Extend QueueState with shuffle/repeat/playedIndices | ✅ Yes | Rust and TypeScript queue models match. |
| Shuffle selects next randomly without reordering queue | ✅ Yes | Selection mutates indices only; queue order is preserved. |
| Rich queue snapshot event replaces plain track array | ✅ Yes | `queue-updated` now emits full `QueueState`. |
| `toggle_favorite` IPC command on backend | ✅ Yes | Command resolves track and delegates to atomic library toggle. |
| Record history exactly once per track start | ⚠️ Partial | Implemented for local starts only; missing cross-source playback support and no runtime test for seek/resume exclusion. |

### Issues Found
**CRITICAL**:
- The `play-history` spec is not met for cross-source playback. `PlaybackService::record_history()` is only called from `play_local()`, while `PlaybackService::play()` still returns `PlatformNotSupported` and remote queue advancement does not record history.
- No passing covering test proves history is not duplicated on seek/resume.
- No passing covering test proves persisted favorite state is restored in Now Playing after restart.
- No passing covering UI test proves the Now Playing heart control covers add/remove behavior end-to-end.
- Shuffle/repeat scenarios only have helper-level coverage, not end-of-track/runtime service verification required by the spec gate.

**WARNING**:
- Engram task artifact `sdd/playback-ux/tasks` is out of sync with `openspec/changes/playback-ux/tasks.md`.
- Design deviation accepted: `set_shuffle` and `set_repeat` return `()`, and `cycle_repeat` returns `String`; frontend sync depends on `queue-updated` events rather than return payloads.
- Frontend build emits a non-blocking Vite chunking warning for `@tauri-apps/api/core.js` dynamic import usage.

**SUGGESTION**:
- Add playback-service integration tests that exercise actual start, seek, resume, next, and end-of-track paths instead of helper-only assertions.
- Add frontend tests for favorite-heart restore/toggle behavior and queue-mode sync from `queue-updated` events.
- Re-sync the Engram `sdd/playback-ux/tasks` artifact so future verify/archive phases do not read stale task status.

### Verdict
FAIL
Required spec scenarios lack runtime proof, and cross-source history behavior is not implemented as specified.
