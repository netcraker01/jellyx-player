# Tasks: Visualizer Frontend

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 250-350 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | FrequencyData type + event subscription + store | PR 1 | base: main; foundation for rendering |
| 2 | Canvas 2D renderer + visualizer modes + CSS tokens | PR 2 | base: main; depends on Unit 1 |

## Phase 1: Foundation (Types & Data Layer)

- [x] 1.1 Add `FrequencyData` interface to `ui/src/shared/types/models.ts` with `bins: number[]`, `sampleRate: number`, `peak: number` matching Rust serde camelCase
- [x] 1.2 Add `onFrequencyData(cb)` function to `ui/src/services/events.ts` using `subscribeEvent<FrequencyData>('frequency-data', cb)`
- [x] 1.3 Add `frequencyData` writable store (`Writable<FrequencyData | null>`) and `modoCineActive` writable store (`Writable<boolean>`) to `ui/src/features/player/stores/player.ts`

## Phase 2: Core Implementation (Canvas Renderer & Modes)

- [x] 2.1 Add visualizer CSS tokens to `ui/src/styles/tokens.css`: `--viz-blur-radius`, `--viz-bar-gap`, `--viz-bar-min-height`, `--viz-color-accent`
- [x] 2.2 Add transition animations to `ui/src/styles/animations.css`: `.viz-enter`, `.viz-leave`, `.modo-cine-transition`
- [x] 2.3 Replace `Visualizer.svelte` stub with full implementation: canvas element, rAF loop that reads `$frequencyData` and draws spectrum bars with `--color-accent`
- [x] 2.4 Add Ambient Blur overlay to `App.svelte`: read `$frequencyData`, compute dominant color from `peak`, apply `backdrop-filter: blur(var(--viz-blur-radius))` with fallback
- [x] 2.5 Add Modo Cine toggle button to `BottomBar.svelte`: icon button that flips `$modoCineActive`, with aria-label

## Phase 3: Integration & Lifecycle

- [x] 3.1 Wire `onFrequencyData` subscription in `Visualizer.svelte` `onMount` — call `onFrequencyData(data => $frequencyData = data)` and store `UnlistenFn`
- [x] 3.2 Implement `onDestroy` cleanup in `Visualizer.svelte`: `cancelAnimationFrame(rafId)` + call `unlisten()` from Tauri subscription
- [x] 3.3 Implement Modo Cine mode toggle in `Visualizer.svelte`: when `$modoCineActive` is true, canvas fills viewport and hides sidebar/bottom bar via CSS classes
- [x] 3.4 Implement Ambient Blur visibility logic in `App.svelte`: show overlay when `$frequencyData !== null` and `!$modoCineActive`

## Phase 4: Testing & Polish

- [x] 4.1 Manual visual test: launch app with Rust backend, verify spectrum bars render on canvas during playback
- [x] 4.2 Manual visual test: verify Ambient Blur overlay shows color derived from frequency data during navigation
- [x] 4.3 Manual visual test: verify Modo Cine toggle expands canvas to fullscreen and Escape exits back to Ambient mode
- [x] 4.4 Verify no memory leaks: navigate away from visualizer and back, confirm rAF and event listener are properly cleaned up