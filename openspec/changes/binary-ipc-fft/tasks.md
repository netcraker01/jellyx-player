# Tasks: Binary IPC for FFT

## Review Workload Forecast
- Estimated changed lines: ~300
- 400-line budget risk: Low
- Chained PRs recommended: No
- Delivery strategy: auto-chain

## Phase 1: Rust Backend — Binary Encoding & Channel
- [x] 1.1 Add `encode_frequency_data_binary()` to `audio/fft.rs`
- [x] 1.2 Add unit tests for binary encoding
- [x] 1.3 Add `fft_channel` to `AppState`
- [x] 1.4 Add `start_fft_stream` command
- [x] 1.5 Register command in `setup.rs`
- [x] 1.6 Rewrite `FftBridge` as `FftChannel` with binary frames
- [x] 1.7 Update FFT thread in `service.rs` to use Channel
- [x] 1.8 Remove dead code from `events.rs`

## Phase 2: Frontend — Binary Decoding & Channel Subscription
- [x] 2.1 Update `FrequencyData` interface (bins: Float32Array)
- [x] 2.2 Update store type
- [x] 2.3 Add `createFftChannel()` to events.ts
- [x] 2.4 Update Visualizer.svelte for Channel + Float32Array
- [x] 2.5 Remove `onFrequencyData()`

## Phase 3: Build Verification
- [x] 3.1 cargo check — zero errors (pre-existing warnings only)
- [x] 3.2 cargo test — 165 passed, 0 failed
- [x] 3.3 vite build — success, 26 Vitest tests passed