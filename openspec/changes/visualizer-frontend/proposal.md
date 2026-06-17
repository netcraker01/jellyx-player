# Proposal: Visualizer Frontend

## Intent

The Svelte frontend has no visualizer implementation — `Visualizer.svelte` is a stub with no canvas, no event subscription, no frequency data type, and no rendering. The Rust backend already emits `frequency-data` Tauri events with JSON-serialized `FrequencyData`. The frontend must receive, store, and render this data in two modes defined by UI_DESIGN.md: Ambient Blur (contextual background during navigation) and Modo Cine (immersive fullscreen visualization).

## Scope

### In Scope
- `FrequencyData` TypeScript interface matching Rust `serde(rename_all = "camelCase")` format
- `onFrequencyData` event subscription in `events.ts`
- `frequencyData` writable store in `player.ts`
- Canvas 2D spectrum bar renderer in `Visualizer.svelte`
- Ambient Blur mode (CSS `backdrop-filter: blur()` + dominant color overlay)
- Modo Cine toggle (fullscreen immersive canvas)
- `requestAnimationFrame` loop decoupled from event rate
- Proper cleanup (rAF cancel + event unlisten on component destroy)
- Visualizer CSS tokens in `tokens.css`
- Transition animations in `animations.css`

### Out of Scope
- Binary IPC (`Uint8Array`) — future optimization, v0.1 uses JSON
- WebGL renderer — deferred to post-v0.1
- Oscilloscope or waveform visualization — spectrum bars only for v0.1
- Audio source selection or FFT configuration from UI
- Album art color extraction (ambient blur uses frequency-derived color)

## Capabilities

### New Capabilities
- `visualizer-rendering`: Canvas 2D spectrum bars, Ambient Blur mode, Modo Cine fullscreen mode, rAF-rendered
- `visualizer-data`: FrequencyData TypeScript type, event subscription, store, lifecycle cleanup

### Modified Capabilities
- None (all new)

## Approach

Canvas 2D + CSS Ambient Blur (Approach 2 from exploration). The Rust backend emits `frequency-data` events with JSON-serialized `FrequencyData { bins, sampleRate, peak }`. The Svelte frontend subscribes via `subscribeEvent`, stores latest data, and draws spectrum bars via `requestAnimationFrame`. Ambient Blur uses CSS `backdrop-filter: blur()` with a color overlay derived from `peak` and dominant frequency bin. Modo Cine expands the canvas to fullscreen with a toggle button. This matches UI_DESIGN.md two-mode spec exactly.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `ui/src/features/player/components/Visualizer.svelte` | Modified | Replace stub with canvas + two-mode rendering |
| `ui/src/services/events.ts` | Modified | Add `onFrequencyData` subscription |
| `ui/src/shared/types/models.ts` | Modified | Add `FrequencyData` interface |
| `ui/src/features/player/stores/player.ts` | Modified | Add `frequencyData` store |
| `ui/src/styles/tokens.css` | Modified | Add visualizer design tokens |
| `ui/src/styles/animations.css` | Modified | Add visualizer transition animations |
| `ui/src/app/App.svelte` | Modified | Add ambient blur overlay container |
| `ui/src/app/layout/BottomBar.svelte` | Modified | Add Modo Cine toggle button |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| CSS backdrop-filter performance on older GPUs | Medium | Feature-detect and disable blur gracefully |
| Canvas memory leaks on component unmount | Low | Strict onMount/onDestroy lifecycle with rAF cancel + event unlisten |
| JSON serialization overhead at 60fps | Low | Acceptable for v0.1 (~128-512 floats); binary IPC planned for future |

## Rollback Plan

Revert `Visualizer.svelte` to stub, remove `onFrequencyData` from `events.ts`, remove `FrequencyData` type, remove store entry. No database or persistent state changes — rollback is purely frontend code removal.

## Dependencies

- Rust `FftBridge` / `PlaybackEventEmitter` emitting `frequency-data` events (already complete)
- Tauri v2 event system (already in use)

## Success Criteria

- [ ] Canvas renders frequency spectrum bars from real-time `frequency-data` events
- [ ] Ambient Blur mode shows during navigation with CSS backdrop-filter
- [ ] Modo Cine expands visualizer to fullscreen immersive view
- [ ] `requestAnimationFrame` loop renders independently of event rate
- [ ] Proper cleanup: no memory leaks on component destroy