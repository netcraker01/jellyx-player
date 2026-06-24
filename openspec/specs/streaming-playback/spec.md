# streaming-playback Specification

## Purpose

Enable audio playback from remote sources via HTTP streaming. The pipeline resolves a stream URL, buffers it through reqwest, decodes via Symphonia, and feeds PCM to the existing audio bus — source-agnostic for any resolver.

## Requirements

### Requirement: Stream Playback Initiation

PlaybackService MUST provide `play_stream(track)` that resolves the track's stream URL via `SourceRegistry.resolve()`, opens an HTTP reader, and feeds it to the decoder pipeline.

#### Scenario: Play a remote track from search result

- GIVEN a track with source type YouTube or SoundCloud
- WHEN `play_stream(track)` is called
- THEN the system resolves the stream URL via the registered resolver
- AND begins buffering the HTTP stream for decoding

#### Scenario: Local track uses existing path

- GIVEN a track with source type local file
- WHEN playback is requested
- THEN the system uses the existing `play_local_track()` path unchanged

### Requirement: HTTP Stream Reader

The system SHALL provide `HttpStreamReader` that fetches a stream URL via reqwest into a buffered cursor suitable for Symphonia's `MediaSourceStream`.

#### Scenario: Successful HTTP stream open

- GIVEN a valid stream URL
- WHEN `HttpStreamReader` opens the connection
- THEN bytes are buffered with backpressure for the decoder
- AND the reader implements `Read + Seek` for Symphonia compatibility

#### Scenario: HTTP connection failure

- GIVEN an unreachable or invalid stream URL
- WHEN `HttpStreamReader` attempts to connect
- THEN the system returns a `StreamError::ConnectionFailed` without panicking

### Requirement: Buffering State Progress

PlaybackState MUST emit `Buffering` with progress percentage before audio output begins for remote tracks.

#### Scenario: Buffering progress emitted

- GIVEN a remote track playback is initiated
- WHEN the HTTP stream is filling the buffer
- THEN `PlaybackState::Buffering(percent)` events are emitted to the frontend
- AND playback transitions to `Playing` once sufficient data is buffered

#### Scenario: Insufficient bandwidth

- GIVEN a remote track is playing and the buffer underruns
- WHEN the decoder cannot read enough data
- THEN the system pauses output and re-emits `Buffering` until the buffer refills

### Requirement: Stream URL Re-resolution on Expiry

The system MUST re-resolve stream URLs on 403 or audio-failure errors instead of failing permanently.

#### Scenario: Expired stream URL auto-recovery

- GIVEN a remote track is playing and the stream URL returns HTTP 403
- WHEN `HttpStreamReader` receives the error
- THEN the system re-resolves the stream URL via `SourceRegistry.resolve()`
- AND resumes playback from the nearest position without manual intervention

#### Scenario: Re-resolution also fails

- GIVEN a stream URL expired and re-resolution was attempted
- WHEN re-resolution also returns an error
- THEN the system emits `PlaybackState::Error(StreamExpired)` and stops playback

### Requirement: Streaming Fallback to Temp File

The system SHOULD fall back to downloading the stream to a temporary file if HTTP streaming fails repeatedly.

#### Scenario: Streaming fails, temp-file fallback

- GIVEN HTTP streaming has failed due to decoder errors or persistent buffer underruns
- WHEN the fallback threshold is reached
- THEN the system downloads the full stream to a temp file and decodes locally

#### Scenario: Temp file also fails

- GIVEN both streaming and temp-file download fail
- WHEN the download cannot complete
- THEN the system emits `PlaybackState::Error(PlaybackFailed)` and stops