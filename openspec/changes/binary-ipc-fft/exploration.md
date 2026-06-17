## Exploration: Binary IPC for FFT Data Transfer

### Current State
- `FftBridge` emits `FrequencyData` as JSON via `AppHandle.emit("frequency-data", data)`
- `FrequencyData` struct: `{ bins: Vec<f32>, sample_rate: u32, peak: f32 }` with `serde(rename_all = "camelCase")`
- Frontend subscribes via `subscribeEvent<FrequencyData>('frequency-data', cb)` → JSON parsing
- FFT runs at ~60Hz, each frame serializing bins as JSON

### Affected Areas
- `src-tauri/src/visualizer/fft_bridge.rs` — primary: change emit mechanism
- `src-tauri/src/audio/fft.rs` — primary: add binary serialization
- `src-tauri/src/playback/events.rs` — secondary: cleanup dead code
- `src-tauri/src/playback/service.rs` — secondary: FFT thread uses Channel
- `ui/src/services/events.ts` — primary: receive binary Uint8Array
- `ui/src/features/player/components/Visualizer.svelte` — primary: Float32Array reconstruction
- `ui/src/features/player/stores/player.ts` — secondary: store type changes
- `ui/src/shared/types/models.ts` — secondary: FrequencyData interface changes

### Recommended Approach
**Channel with serialized metadata header**: Use Tauri v2 `Channel<&[u8]>` to stream binary FFT frames. Frame format: [4B sample_rate u32 LE][4B peak f32 LE][N*4B bins f32 LE]. Frontend reconstructs via DataView/Float32Array.

### Risks
- Channel requires command parameter — FFT thread needs stored reference
- Backpressure at 60fps — drop stale frames
- Custom binary protocol must be documented