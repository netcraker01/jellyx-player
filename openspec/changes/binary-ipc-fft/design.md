# Design: Binary IPC for FFT

## Technical Approach
Replace JSON emit with Tauri v2 `Channel<&[u8]>`. Add `start_fft_stream` command. FFT thread encodes binary frames. Frontend decodes with DataView/Float32Array.

## Key Decisions
- D1: Channel API over emit() (binary, no JSON)
- D2: Single Channel for all FFT data (metadata in frame header)
- D3: Channel in AppState behind Arc<Mutex<Option<>>>
- D4: Non-blocking frame sending (drop on failure)
- D5: Frontend bins as Float32Array
- D6: Remove FftBridge and dead emit_frequency_data

## Binary Frame
[0-3] sample_rate u32 LE, [4-7] peak f32 LE, [8+] bins f32 LE
For 1024 FFT: 2056 bytes (vs ~3000-5000 JSON)

## Changed Files
- visualizer/fft_bridge.rs → FftChannel with binary encoding
- ipc/commands.rs → start_fft_stream command + AppState.fft_channel
- playback/service.rs → FFT thread uses Channel
- playback/events.rs → remove dead code
- audio/fft.rs → encode_frequency_data_binary()
- ui/services/events.ts → createFftChannel()
- ui/Visualizer.svelte → Channel subscription + Float32Array
- ui/models.ts → FrequencyData.bins: Float32Array
- ui/player.ts → store type update