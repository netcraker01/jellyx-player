# Delta Specs: Binary IPC for FFT

## Binary Frame Protocol

### ADDED Requirements

**REQ-BF1: Binary Frame Format** — Frame layout (all little-endian): [0-3] sample_rate u32 LE, [4-7] peak f32 LE, [8+] bins N*f32 LE.

**REQ-BF2: Channel-Based Streaming** — Use Tauri v2 `Channel<&[u8]>` via `start_fft_stream` command.

**REQ-BF3: Channel Lifecycle** — Channel created when frontend invokes `start_fft_stream`, dropped when playback stops. FFT thread skips frames if Channel unavailable.

**REQ-BF4: Frontend Binary Decoding** — DataView for sample_rate/peak, Float32Array view for bins.

**REQ-BF5: Visualizer Binary Consumption** — Consume Float32Array directly (no number[] conversion).

## Dead Code Cleanup

### CHANGED Requirements

**REQ-DC1: Remove PlaybackEventEmitter.emit_frequency_data** — Dead code, superseded by Channel.

## Backward Compatibility

### CHANGED Requirements

**REQ-BC1: JSON FrequencyData event removed** — Replaced by Channel-based binary stream.