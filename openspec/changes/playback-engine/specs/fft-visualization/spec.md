# Delta for fft-visualization

## ADDED Requirements

### Requirement: PE-007 FFT Engine computes frequency data from PCM frames

The FFT Engine MUST receive PCM frames from the PCM Bus and compute frequency spectrum data using the existing AudioAnalyzer.

#### Scenario: Compute spectrum from playing audio

- GIVEN audio is playing and PCM frames are flowing through the PCM Bus
- WHEN the FFT Engine receives a block of PCM samples
- THEN it SHALL compute frequency bins via AudioAnalyzer.analyze() and produce FrequencyData

#### Scenario: No audio playing

- GIVEN no audio is playing and no PCM frames are available
- WHEN the FFT Engine has no data
- THEN it SHALL emit an empty/silent frequency dataset (all bins at 0.0)

### Requirement: PE-008 fft_bridge sends frequency data to frontend via Tauri binary IPC

The FFT bridge MUST send frequency bin data to the Svelte frontend using Tauri v2's binary event system (typed arrays, not JSON).

#### Scenario: Send frequency data during playback

- GIVEN audio is playing and FrequencyData is available
- WHEN the FFT bridge processes new frequency data
- THEN it SHALL emit a Tauri binary event with a Uint8Array of frequency bins to the frontend

#### Scenario: Stop sending when playback stops

- GIVEN audio was playing and transitions to Stopped
- WHEN playback stops
- THEN the FFT bridge SHALL stop emitting frequency data events

### Requirement: PE-009 Frontend Visualizer receives Uint8Array and renders

The frontend Visualizer component MUST receive Uint8Array frequency data from the FFT bridge and render it.

#### Scenario: Receive and render frequency data

- GIVEN the Svelte frontend has a Visualizer component mounted
- WHEN a binary frequency data event arrives
- THEN the Visualizer SHALL receive the Uint8Array and update the canvas visualization

#### Scenario: Handle no data gracefully

- GIVEN the frontend Visualizer is mounted but no audio is playing
- WHEN no frequency events arrive
- THEN the Visualizer SHALL render an idle/empty state without errors