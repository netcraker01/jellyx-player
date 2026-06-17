# Proposal: Binary IPC for FFT

## Intent
Migrate FFT frequency data transfer from JSON serialization to binary IPC using Tauri v2's `Channel` API. Eliminates JSON parsing overhead at 60fps, matching ARCHITECTURE.md §2.

## Scope

### In Scope
- Replace `FftBridge` JSON emit with `Channel<&[u8]>` binary streaming
- Binary frame format: `[4B sample_rate u32 LE][4B peak f32 LE][N*4B bins f32 LE]`
- Add `start_fft_stream` Tauri command to establish Channel
- Frontend: binary Channel subscription, Float32Array reconstruction
- Update Visualizer.svelte to consume binary data
- Clean up dead code in PlaybackEventEmitter
- Update FrequencyData TypeScript type

### Out of Scope
- Other event types (stay JSON), WebGL rendering, FFT algorithm, PcmBus changes

## Approach
1. Add `start_fft_stream` command with `Channel<&[u8]>` parameter
2. Store Channel in AppState for FFT thread access
3. FFT thread sends binary frames via channel
4. Frontend creates Channel, decodes Uint8Array frames
5. Visualizer uses Float32Array directly

## Rollback Plan
Revert to JSON event model. Small surface area (4 files).