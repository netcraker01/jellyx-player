# audio-pipeline Specification

## Purpose

Extend the SymphoniaDecoder to accept HTTP streams via a reqwest buffered reader, enabling remote audio decode alongside existing local-file playback.

## Requirements

### Requirement: HTTP Stream Input for Decoder

SymphoniaDecoder MUST accept `HttpStreamReader` as input in addition to local file paths.

#### Scenario: Decode an HTTP stream

- GIVEN a valid HTTP stream URL from a remote source
- WHEN the decoder receives the `HttpStreamReader`
- THEN Symphonia decodes the stream into PCM samples
- AND output is fed to the PCM Bus identical to local file decode

#### Scenario: Stream format not supported

- GIVEN an HTTP stream with a codec Symphonia cannot decode
- WHEN the decoder attempts to process it
- THEN a `DecodeError::UnsupportedFormat` is returned without panic

### Requirement: Source-Agnostic Decode Path

The decode pipeline MUST NOT contain source-specific logic. HTTP streams from any resolver use the same path.

#### Scenario: YouTube and SoundCloud use identical decode path

- GIVEN stream URLs from both YouTube and SoundCloud resolvers
- WHEN each is decoded
- THEN both use `HttpStreamReader` → SymphoniaDecoder → PCM Bus
- AND no source-specific branching exists in the decode path

### Requirement: Buffered Reader Backpressure

The HTTP stream reader MUST apply backpressure so that buffering does not consume unbounded memory.

#### Scenario: Fast network, slow decode

- GIVEN a high-bandwidth HTTP stream and a slow decoder
- WHEN data arrives faster than it is consumed
- THEN the buffer is capped at a configurable maximum
- AND backpressure pauses the HTTP download until the buffer drains

#### Scenario: Slow network, fast decode

- GIVEN a low-bandwidth HTTP stream and a fast decoder
- WHEN the buffer underruns
- THEN the system pauses PCM output and emits `Buffering` state
- AND resumes when sufficient data is buffered