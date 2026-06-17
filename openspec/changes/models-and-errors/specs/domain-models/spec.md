# Domain Models Specification

## Purpose

Core domain models (Source, Track, Artist, Album) shared across playback, library, search, and IPC. These types form the contract between Rust backend and TypeScript frontend, matching ARCHITECTURE.md §4.

## Requirements

### ME-001: Source Enum

The system SHALL define a `Source` enum with `YouTube`, `SoundCloud`, and `Local` variants. It MUST derive `Serialize`, `Deserialize`, `Clone`, `PartialEq`, and `Debug`. Variant names MUST serialize as PascalCase strings matching the TypeScript `Source` enum values exactly.

#### Scenario: Source serializes to PascalCase JSON

- GIVEN a `Source::YouTube` value
- WHEN serialized to JSON
- THEN the output MUST be `"YouTube"`
- AND deserializing `"YouTube"` MUST yield `Source::YouTube`

#### Scenario: All three variants roundtrip

- GIVEN `Source::YouTube`, `Source::SoundCloud`, `Source::Local`
- WHEN each is serialized then deserialized
- THEN each MUST equal its original value

### ME-002: Track Struct with Enrichment

The system SHALL define a `Track` struct matching ARCHITECTURE.md §4.1 with fields: `id`, `source` (Source enum), `source_id`, `title`, `artist`, `album` (Option), `duration` (Option<f64>), `thumbnail` (Option), `stream_url` (Option), `local_path` (Option), `metadata` (HashMap<String,String>). It MUST derive `Serialize`, `Deserialize`, `Clone`, `Debug` with `#[serde(rename_all = "camelCase")]`.

#### Scenario: Track serializes with camelCase field names

- GIVEN a Track with `source_id = "abc123"` and `stream_url = Some("http://...")`
- WHEN serialized to JSON
- THEN field names MUST be `sourceId`, `streamUrl`, `localPath`
- AND `source` MUST serialize as the Source enum value (PascalCase)

#### Scenario: Track with optional fields set to None

- GIVEN a Track where `album = None`, `duration = None`
- WHEN serialized to JSON
- THEN those fields MUST be absent from JSON (skip_serializing_if = "Option::is_none")

#### Scenario: Track deserialization from frontend JSON

- GIVEN a JSON object matching the TypeScript Track interface
- WHEN deserialized into a Rust Track
- THEN all fields MUST populate correctly, including Source enum and optional fields

### ME-003: Artist Struct

The system SHALL define an `Artist` struct with `id`, `name`, `thumbnail` (Option), `source` (Source), `source_id`. It MUST derive `Serialize`, `Deserialize`, `Clone`, `Debug` with `#[serde(rename_all = "camelCase")]`.

#### Scenario: Artist roundtrip serialization

- GIVEN an Artist with `source = Source::SoundCloud`
- WHEN serialized then deserialized
- THEN the result MUST equal the original Artist

### ME-004: Album Struct

The system SHALL define an `Album` struct with `id`, `title`, `artist`, `cover` (Option), `year` (Option<u32>), `source` (Source), `source_id`, `tracks` (Vec<String>). It MUST derive `Serialize`, `Deserialize`, `Clone`, `Debug` with `#[serde(rename_all = "camelCase")]`.

#### Scenario: Album with track list serializes

- GIVEN an Album with `tracks = ["t1", "t2"]`
- WHEN serialized to JSON
- THEN `tracks` MUST appear as a JSON array of strings
- AND `year` MUST serialize as a number when present

### ME-005: SourceResolver Trait Uses Source Enum

The `SourceResolver` trait MUST use `Source` enum (not `String`) in its method signatures. `search()` SHALL return `Result<Vec<Track>, SourceError>` and `resolve()` SHALL return `Result<Track, SourceError>` where Track now contains `source: Source`.

#### Scenario: SourceResolver returns Track with Source enum

- GIVEN any SourceResolver implementation
- WHEN `search()` returns tracks
- THEN each Track's `source` field MUST be a `Source` variant, not a String

### ME-006: YouTube Resolver Constructs Source::YouTube Tracks

The `YouTubeResolver` MUST construct Track instances with `source: Source::YouTube` instead of `source: "YouTube".to_string()`.

#### Scenario: YouTube search produces Source::YouTube tracks

- GIVEN a YouTubeResolver and a search query
- WHEN `search()` succeeds
- THEN every returned Track MUST have `source == Source::YouTube`