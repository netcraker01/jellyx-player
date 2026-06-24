# playlist-model Specification

## Purpose

Define the Playlist domain model, SourceResolver playlist trait extensions, and yt-dlp playlist extraction logic. Covers search, resolve, and Track→Playlist composition.

## Requirements

### Requirement: Playlist Model

The system MUST define a `Playlist` struct with fields: `id`, `title`, `thumbnail`, `track_count`, and `tracks` (Vec<Track>), with full serde serialization support.

#### Scenario: Playlist deserialization from resolver output

- GIVEN a resolver returns playlist JSON from yt-dlp
- WHEN the system parses the output
- THEN a `Playlist` struct is populated with all fields
- AND each entry in `tracks` contains at minimum an id, title, and duration

#### Scenario: Playlist with many tracks

- GIVEN a playlist containing 200+ tracks
- WHEN the system resolves the playlist
- THEN all tracks are parsed without truncation
- AND `track_count` matches the length of `tracks`

### Requirement: SourceResolver Playlist Trait Methods

`SourceResolver` trait MUST include `search_playlists(query)` and `resolve_playlist(url)` as default no-op methods.

#### Scenario: Resolver that supports playlists

- GIVEN `YouTubeResolver` which implements playlist methods
- WHEN `search_playlists(query)` is called
- THEN it returns a list of `Playlist` search results

#### Scenario: Resolver without playlist support

- GIVEN a resolver that does not override playlist defaults
- WHEN `search_playlists(query)` is called
- THEN it returns an empty result without error

#### Scenario: Resolve playlist from URL

- GIVEN a valid YouTube playlist URL
- WHEN `resolve_playlist(url)` is called on YouTubeResolver
- THEN it returns a complete `Playlist` with all tracks populated

### Requirement: YouTube Playlist Extraction

YouTubeResolver MUST extract playlists using yt-dlp `--yes-playlist` and playlist dump-json output.

#### Scenario: Search for playlists by query

- GIVEN a user search query like "lofi study mix"
- WHEN `YouTubeResolver.search_playlists()` is invoked
- THEN yt-dlp is called with `--yes-playlist` flag
- AND results are parsed into `Playlist` objects

#### Scenario: Playlist URL direct resolution

- GIVEN a YouTube playlist URL (contains `/playlist?list=`)
- WHEN `resolve_playlist(url)` is called
- THEN yt-dlp extracts all track entries via dump-json
- AND each entry is mapped to a `Track` with source metadata

#### Scenario: Defensively parse variable yt-dlp output

- GIVEN a playlist where yt-dlp JSON format varies (missing fields, mixed types)
- WHEN the system parses the output
- THEN missing optional fields default to reasonable values (empty string, zero)
- AND parsing does not panic on malformed entries

### Requirement: Track-Playlist Composition

A Track MAY reference its parent playlist via an optional `playlist_id` field.

#### Scenario: Track from playlist carries playlist reference

- GIVEN a track extracted from a playlist resolve operation
- WHEN the track is added to the playback queue
- THEN the track includes its `playlist_id` for traceability

#### Scenario: Standalone track has no playlist reference

- GIVEN a track from a regular search (not playlist)
- WHEN the track is created
- THEN `playlist_id` is None