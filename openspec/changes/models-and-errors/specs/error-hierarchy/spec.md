# Error Hierarchy Specification

## Purpose

Domain-specific error types with structured `AppError` conversion, covering all current and near-term error domains. Ensures all IPC-bound types are serializable for Tauri command responses.

## Requirements

### ME-007: Expanded AppError Hierarchy

The system SHALL define `AppError` as a structured error with `code: String` and `details: Option<String>`, with `From` implementations for: `SourceError`, `AudioError`, `PlaybackError`, `LibraryError`, `PersistenceError`, `ValidationError`, and `IPCError`. Each domain error type MUST map to a distinct, stable error `code` string.

#### Scenario: SourceError converts to AppError

- GIVEN `SourceError::NetworkError("timeout")`
- WHEN converted to AppError
- THEN `code` MUST be `"NETWORK_TIMEOUT"` and `details` MUST be `Some("timeout")`

#### Scenario: PlaybackError converts to AppError

- GIVEN `PlaybackError::AlreadyStopped`
- WHEN converted to AppError
- THEN `code` MUST be `"PLAYBACK_ERROR"`

#### Scenario: LibraryError converts to AppError

- GIVEN `LibraryError::NotFound("track xyz")`
- WHEN converted to AppError
- THEN `code` MUST be `"NOT_FOUND"` and `details` MUST contain the identifier

#### Scenario: PersistenceError converts to AppError

- GIVEN `PersistenceError::DatabaseError("disk full")`
- WHEN converted to AppError
- THEN `code` MUST be `"PERSISTENCE_ERROR"`

#### Scenario: ValidationError converts to AppError

- GIVEN `ValidationError::InvalidInput("empty query")`
- WHEN converted to AppError
- THEN `code` MUST be `"VALIDATION_ERROR"`

#### Scenario: IPCError converts to AppError

- GIVEN `IPCError::CommandFailed("search")`
- WHEN converted to AppError
- THEN `code` MUST be `"IPC_ERROR"`

### ME-008: Serialize on IPC-Bound Types

`PlaybackState`, `AudioError`, and `FrequencyData` MUST derive `Serialize` so they can be sent across Tauri IPC. `PlaybackState` variants MUST serialize as PascalCase (`"Playing"`, `"Stopped"`, etc.). `AudioError` variant names MUST serialize as `snake_case` per serde convention.

#### Scenario: PlaybackState serializes for IPC

- GIVEN `PlaybackState::Playing`
- WHEN serialized to JSON
- THEN the result MUST be `"Playing"`

#### Scenario: AudioError serializes with message

- GIVEN `AudioError::DecodeError("bad frame")`
- WHEN serialized to JSON
- THEN the JSON MUST include variant and message data

#### Scenario: FrequencyData serializes for binary IPC fallback

- GIVEN a `FrequencyData` with `bins`, `sample_rate`, and `peak`
- WHEN serialized with serde
- THEN all three fields MUST appear in the output

### ME-009: Model Serialization Roundtrip Tests

The system MUST include unit tests verifying that each domain model (Track, Artist, Album, Source) serializes to JSON and deserializes back to an equal Rust value.

#### Scenario: Track roundtrip test

- GIVEN a Track with all fields populated (including optional fields and metadata)
- WHEN the Track is serialized to JSON then deserialized
- THEN the result MUST equal the original Track

#### Scenario: Track with None fields roundtrip

- GIVEN a Track with `album = None`, `duration = None`
- WHEN serialized then deserialized
- THEN the result MUST equal the original Track
- AND None fields MUST be absent from JSON

#### Scenario: Source enum roundtrip

- GIVEN each Source variant (YouTube, SoundCloud, Local)
- WHEN serialized then deserialized
- THEN each MUST equal its original value

### ME-010: Error Conversion Unit Tests

The system MUST include unit tests verifying every `From<DomainError> for AppError` implementation produces the correct `code` and `details`.

#### Scenario: All SourceError variants tested

- GIVEN each SourceError variant
- WHEN converted to AppError via `From`
- THEN each MUST produce the documented `code` and `details`

#### Scenario: All AudioError variants tested

- GIVEN each AudioError variant
- WHEN converted to AppError via `From`
- THEN each MUST produce the documented `code` and `details`

#### Scenario: New error domain conversions tested

- GIVEN each variant of PlaybackError, LibraryError, PersistenceError, ValidationError, IPCError
- WHEN converted to AppError via `From`
- THEN each MUST produce a distinct, documented `code`