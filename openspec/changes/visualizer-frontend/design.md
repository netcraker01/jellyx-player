# Design: Visualizer Frontend

## Technical Approach

Canvas 2D + CSS Ambient Blur per exploration recommendation. The Rust backend emits `frequency-data` events with JSON-serialized `FrequencyData { bins, sampleRate, peak }`. The Svelte frontend subscribes via `subscribeEvent<FrequencyData>('frequency-data', cb)`, stores latest data in a writable store, and renders via `requestAnimationFrame` loop. Two visualization modes: Ambient Blur (CSS overlay during navigation) and Modo Cine (fullscreen canvas). This matches UI_DESIGN.md §5 and specs VF-001 through VF-009.

## Architecture Decisions

### Decision: Canvas 2D over WebGL

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Canvas 2D | Simple, sufficient for bars, CPU-bound at 60fps | ✅ Chosen for v0.1 |
| WebGL | GPU-accelerated, shader effects, high complexity | ❌ Deferred |

**Rationale**: Spectrum bars are geometric primitives — Canvas 2D handles this trivially. WebGL adds shader compilation, buffer management, and debugging overhead disproportionate to v0.1 needs. Binary IPC (future) may warrant WebGL revisit.

### Decision: JSON serialization for v0.1

| Option | Tradeoff | Decision |
|--------|----------|----------|
| JSON via `emit()` | ~128-512 floats at 60fps, negligible overhead for v0.1 | ✅ Chosen |
| Binary IPC (`Uint8Array`) | Lower CPU on serialization, requires Tauri custom protocol | ❌ Future |

**Rationale**: The Rust `FftBridge` already uses `emit()` with `serde::Serialize`. JSON serialization of 128-512 floats at 60fps is negligible. The architecture doc (§2) notes binary IPC as future optimization.

### Decision: CSS backdrop-filter for Ambient Blur

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `backdrop-filter: blur()` | Hardware-accelerated on modern GPUs, matches UI_DESIGN.md | ✅ Chosen |
| Canvas-based blur | Full control, no GPU feature dependency, heavier CPU | ❌ More complex |
| SVG filter | Cross-platform, less performant, harder to animate | ❌ |

**Rationale**: UI_DESIGN.md specifies "muy desenfocado" (very blurred) — `backdrop-filter: blur()` is the CSS-native, GPU-accelerated solution. Feature-detect and fallback to semi-transparent overlay.

### Decision: Svelte store for frequency data (not direct prop passing)

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Writable store | Decouples event subscription from component, reusable | ✅ Chosen |
| Component-local state | Simpler but tightly coupled, harder to share | ❌ |

**Rationale**: `frequencyData` store allows Ambient Blur overlay (in `App.svelte`) and Canvas renderer (in `Visualizer.svelte`) to both read the same data without prop drilling.

## Data Flow

```
Rust FftEngine.analyze_if_ready()
  → PlaybackEventEmitter.emit_frequency_data()
  → Tauri event "frequency-data" (JSON)
  → subscribeEvent<FrequencyData>() in events.ts
  → frequencyData store update (player.ts)
  ├──→ App.svelte (ambient blur overlay reads $frequencyData)
  └──→ Visualizer.svelte (rAF loop reads $frequencyData → Canvas 2D draw)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `ui/src/shared/types/models.ts` | Modify | Add `FrequencyData` interface |
| `ui/src/services/events.ts` | Modify | Add `onFrequencyData()` subscription |
| `ui/src/features/player/stores/player.ts` | Modify | Add `frequencyData` writable store |
| `ui/src/features/player/components/Visualizer.svelte` | Modify | Replace stub with canvas + two-mode renderer |
| `ui/src/styles/tokens.css` | Modify | Add visualizer tokens (--viz-blur-radius, --viz-bar-gap, etc.) |
| `ui/src/styles/animations.css` | Modify | Add .viz-enter/.viz-leave transitions and .modo-cine-transition |
| `ui/src/app/App.svelte` | Modify | Add ambient blur overlay element |
| `ui/src/app/layout/BottomBar.svelte` | Modify | Add Modo Cine toggle button |

## Interfaces / Contracts

```typescript
// ui/src/shared/types/models.ts
export interface FrequencyData {
  bins: number[];      // f32 array from FFT, length = fft_size/2
  sampleRate: number;   // u32, matches Rust serde camelCase
  peak: number;         // f32, max bin value for amplitude reference
}
```

```typescript
// ui/src/services/events.ts — addition
export function onFrequencyData(
  cb: (data: FrequencyData) => void
): Promise<UnlistenFn> {
  return subscribeEvent<FrequencyData>('frequency-data', cb);
}
```

```typescript
// ui/src/features/player/stores/player.ts — addition
export const frequencyData = writable<FrequencyData | null>(null);
export const modoCineActive = writable<boolean>(false);
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `FrequencyData` type shape matches Rust JSON | TypeScript compile check + manual verify field names |
| Unit | `onFrequencyData` subscribes to correct event name | Mock `subscribeEvent`, verify event name `'frequency-data'` |
| Unit | `frequencyData` store updates on event | Set store value, verify reactive update |
| Integration | Canvas renders bars from FrequencyData | Manual visual test (no automated canvas testing infra yet) |
| Integration | Ambient Blur overlay appears during playback | Manual visual test |
| Integration | Modo Cine toggle enters/exits fullscreen | Manual visual test |
| E2E | Full flow: Rust emit → Svelte render | Manual test with running app |

Note: Project has no test infrastructure yet (per `openspec/config.yaml`). Testing is manual visual verification for v0.1.

## Migration / Rollback

No migration required. Rollback: revert `Visualizer.svelte` to stub, remove `onFrequencyData`, `FrequencyData` type, `frequencyData` store, CSS tokens, and BottomBar button additions.

## Open Questions

- [ ] Should Ambient Blur derive color from `peak` mapped to HSL hue, or from the dominant frequency bin index? (Leaning toward peak → HSL for simplicity)
- [ ] What idle animation should the canvas show when no frequency data is available? (Minimal pulse or blank)