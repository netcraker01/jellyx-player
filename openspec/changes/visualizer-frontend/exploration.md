# Exploration: visualizer-frontend

## Current State

The frontend Visualizer is a stub (`<div class="visualizer"><p>Visualizer stub</p></div>`). The Rust backend is fully functional: `FftEngine` computes `FrequencyData { bins: Vec<f32>, sample_rate: u32, peak: f32 }` from the PCM Bus, and `FftBridge`/`PlaybackEventEmitter` emit it as a Tauri event (`frequency-data`) using `serde::Serialize` with `#[serde(rename_all = "camelCase")]`. However, **v0.1 uses JSON serialization via standard `emit()`** — not binary IPC (noted as future optimization in `fft_bridge.rs`). The Svelte `events.ts` service has no `onFrequencyData` subscription yet. The `models.ts` file has no `FrequencyData` type. No canvas/WebGL code exists anywhere in the frontend. The player store (`player.ts`) is a minimal stub with writable stores but no event subscriptions. The `UI_DESIGN.md` defines two visualization modes: **Ambient Blur** (contextual, during navigation) and **Modo Cine** (immersive fullscreen). The architecture doc (§2) calls for binary IPC with `Uint8Array`, but the current Rust implementation uses JSON for v0.1.

## Affected Areas

- `ui/src/features/player/components/Visualizer.svelte` — current stub, needs full implementation with canvas
- `ui/src/services/events.ts` — needs `onFrequencyData` subscription for `frequency-data` event
- `ui/src/shared/types/models.ts` — needs `FrequencyData` TypeScript interface
- `ui/src/features/player/stores/player.ts` — needs `frequencyData` store or reactive subscription
- `ui/src/styles/tokens.css` — may need visualizer-specific tokens (blur radius, colors)
- `ui/src/styles/animations.css` — needs transition/animation definitions for visualizer modes
- `ui/src/app/App.svelte` — integration point for ambient blur background
- `ui/src/app/layout/BottomBar.svelte` — needs "expand visualizer" button for Modo Cine
- `src-tauri/src/visualizer/fft_bridge.rs` — current implementation uses JSON emit, not binary IPC
- `src-tauri/src/playback/events.rs` — confirms event name `frequency-data`

## Approaches

### 1. Canvas 2D Bars (Spectrum Bars)

Render frequency bins as vertical bars on a `<canvas>` element using CanvasRenderingContext2D. Subscribe to `frequency-data` events, update bins array, draw bars in `requestAnimationFrame` loop.

- **Pros**: Simplest to implement, no WebGL complexity, good v0.1, works on all devices, easy to debug
- **Cons**: Limited visual wow-factor compared to WebGL, CPU-bound rendering at 60fps, no shader effects for ambient blur
- **Effort**: Low

### 2. Canvas 2D + Ambient Blur

Canvas 2D for bars, plus CSS `backdrop-filter: blur()` + `background-color` derived from `peak` and dominant frequency bin for the ambient effect described in UI_DESIGN.md. Use a dedicated `<canvas>` for Modo Cine, and CSS blur overlay for Ambient mode.

- **Pros**: Matches UI_DESIGN.md spec for two modes, CSS blur is hardware-accelerated, moderate complexity
- **Cons**: Two rendering paths (ambient vs cinema), CSS blur performance varies across platforms, need to extract dominant color from frequency data
- **Effort**: Medium

### 3. WebGL Renderer

Full WebGL canvas with shaders for spectrum visualization, glow effects, and smooth animations.

- **Pros**: Maximum visual impact, GPU-accelerated, best for 60fps sustained rendering, enables shader-based effects
- **Cons**: Significantly higher complexity for v0.1, requires WebGL boilerplate (shader compilation, buffer management), harder to debug, larger change scope
- **Effort**: High

## Recommendation

**Approach 2: Canvas 2D + Ambient Blur** — This is the right balance for v0.1 because:

1. UI_DESIGN.md explicitly defines two visualization modes (Ambient Blur + Modo Cine), so we MUST implement both.
2. Canvas 2D is sufficient for spectrum bars in Modo Cine — we don't need WebGL for a bar visualization.
3. CSS `backdrop-filter: blur()` is the correct way to implement the Ambient Blur effect (the doc says "muy desenfocado" — very blurred background).
4. The approach is incremental: start with the `onFrequencyData` event subscription + store, then the canvas renderer, then the ambient overlay.
5. The architecture already planned for binary IPC as a future optimization. JSON serialization at 60fps is acceptable for v0.1 (the bins array is small — typically 128-512 floats).

## Key Implementation Details

### FrequencyData format (from Rust)

```typescript
interface FrequencyData {
  bins: number[];      // f32 array, length = fft_size/2 (e.g., 512 bins for 1024 FFT)
  sampleRate: number;  // u32, e.g., 44100 or 48000
  peak: number;        // f32, max bin value for quick amplitude reference
}
```

Note: `serde(rename_all = "camelCase")` in Rust means the JSON keys are `bins`, `sampleRate`, `peak` — the TypeScript interface must match exactly.

### Event flow

```
Rust FftEngine.analyze_if_ready() → FrequencyData
  → PlaybackEventEmitter.emit_frequency_data() or FftBridge.emit_frequency_data()
  → Tauri event "frequency-data" (JSON serialized, serde rename_all = camelCase)
  → Svelte subscribeEvent<FrequencyData>('frequency-data', cb)
  → frequencyData store update
  → requestAnimationFrame → Canvas 2D draw bars
```

### Performance strategy

- JSON serialization at 60fps with ~128-512 floats is negligible overhead for v0.1
- Use `requestAnimationFrame` to batch canvas draws — do NOT draw on every Tauri event
- Throttle: store latest `FrequencyData` in a Svelte store, read it in `requestAnimationFrame` callback
- This decouples render rate from event rate (events may come at different frequency than 60fps)

## Risks

- **JSON at 60fps**: Current implementation uses `serde::Serialize` JSON emit. For 128-512 bins at 60fps, this creates ~60 JSON serializations/sec. Acceptable for v0.1 but MUST migrate to binary IPC for production. The architecture doc already notes this.
- **Canvas memory leaks**: Svelte `onMount`/`onDestroy` lifecycle must properly clean up the `requestAnimationFrame` loop and Tauri event listener to avoid memory leaks on component unmount/remount.
- **Cross-platform Canvas performance**: CSS `backdrop-filter: blur()` may not perform well on older GPUs. Need a fallback strategy for systems that don't support it.
- **No existing FrequencyData type in frontend**: The TypeScript interface needs to be created matching the Rust `#[serde(rename_all = "camelCase")]` serialization format — `sampleRate` (not `sample_rate`).
- **Two rendering paths**: Ambient Blur (CSS overlay) and Modo Cine (Canvas fullscreen) need a mode toggle mechanism. This adds complexity but is required by UI_DESIGN.md.

## Ready for Proposal

Yes. The gap is clearly defined. The change should implement:

1. `FrequencyData` TypeScript type
2. `onFrequencyData` event subscription
3. `frequencyData` Svelte store
4. Canvas 2D bar spectrum renderer in Visualizer.svelte
5. Ambient blur overlay (CSS-based, using dominant frequency color)
6. Modo Cine toggle (expand to fullscreen)
7. Proper lifecycle management (rAF cleanup, event unlisten)