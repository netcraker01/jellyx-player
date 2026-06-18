# Tasks: Home Recommendations

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 550-800 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 Rust domain+DB → PR 2 IPC+TS types/store → PR 3 Home UI |
| Delivery strategy | force-chained |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Rust DTOs, DB inventory, recommendation service | PR 1 | Base slice; includes cargo test |
| 2 | IPC + TS models + home store | PR 2 | Depends on PR 1; includes vitest |
| 3 | Home page rendering + empty/error states | PR 3 | Depends on PR 2; UI verification |

## Phase 1: Rust DTO + Persistence (RED → GREEN)

- [x] 1.1 Add failing DTO serialization tests in `src-tauri/src/ipc/dto.rs` for `RecommendationItem` and `HomeSnapshot`. AC: `cargo test` proves camelCase fields and tagged `type` variants.
- [x] 1.2 Implement `RecommendationItem` and `HomeSnapshot` in `src-tauri/src/ipc/dto.rs`. AC: tests from 1.1 pass without changing frontend code.
- [x] 1.3 Add failing DB tests in `src-tauri/src/persistence/db.rs` for `get_all_local_tracks()` ordering and empty inventory. AC: tests fail before method exists.
- [x] 1.4 Implement `get_all_local_tracks()` in `src-tauri/src/persistence/db.rs`. AC: returns every `LocalTrackEntry` and passes new DB tests.

## Phase 2: Rust Snapshot Service + IPC (RED → GREEN → REFACTOR)

- [x] 2.1 Add failing service tests in `src-tauri/src/library/service.rs` for recent-history limit, affinity ordering, recent exclusion, and library fallback. AC: each spec scenario has a named failing test.
- [x] 2.2 Implement `LibraryService::get_home_snapshot()` and recommendation helpers in `src-tauri/src/library/service.rs`. AC: max 20 recent + 20 recommendations, deterministic heuristic caps 8/4/4/4, Rust owns ranking.
- [x] 2.3 Add command-level wiring test coverage in `src-tauri/src/ipc/commands.rs` or adjacent Rust tests for `get_home_snapshot`. AC: command returns service snapshot and propagates `AppError`.
- [x] 2.4 Register `get_home_snapshot` in `src-tauri/src/ipc/commands.rs` and `src-tauri/src/app/setup.rs`. AC: Tauri invoke handler exposes the command exactly once.

## Phase 3: TypeScript Contract + Home Store (RED → GREEN)

- [ ] 3.1 Add failing type/wrapper tests in `ui/src/shared/types/models.test.ts` and `ui/src/services/commands.test.ts` for `HomeSnapshot`, `RecommendationItem`, and `getHomeSnapshot()`. AC: Vitest fails until wrappers/types exist.
- [ ] 3.2 Implement TS models in `ui/src/shared/types/models.ts` and wrapper in `ui/src/services/commands.ts`. AC: union narrows by `type`; wrapper invokes `get_home_snapshot`.
- [ ] 3.3 Add failing store tests in `ui/src/features/home/stores/home.test.ts` for load success, error toast, and clear reset. AC: tests assert loading/error transitions.
- [ ] 3.4 Implement `ui/src/features/home/stores/home.ts`. AC: one `load()` call fetches snapshot, surfaces notifications, and clears state cleanly.

## Phase 4: Home Page Integration (RED → GREEN)

- [ ] 4.1 Add failing page tests in `ui/src/routes/Home/Page.test.ts` for loading, mixed recommendation rendering, empty Search CTA, and partial-data rendering. AC: scenarios map to REQ-HS-1, REQ-HR-2, and REQ-HE-1.
- [ ] 4.2 Refactor `ui/src/routes/Home/Page.svelte` to use `homeStore`, `TrackRow`, `ArtistCard`, and `AlbumCard`. AC: no inline ranking/history fetch remains; reason labels and navigation render by item type.
- [ ] 4.3 Run `cargo test` and `cd ui && npx vitest run`; fix refactors only after green. AC: both suites pass and tasks stay reviewable per PR slice.
