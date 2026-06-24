# youtube-resolver Specification

## Purpose

YouTube source resolver with track search, playlist search, and playlist resolve via yt-dlp. Extends the existing resolver with playlist capabilities.

## Requirements

### Requirement: Playlist Search Mode

YouTubeResolver MUST support playlist search queries using yt-dlp `--yes-playlist` flag.

#### Scenario: Search returns playlists

- GIVEN a search query like "study music mix"
- WHEN `YouTubeResolver.search_playlists(query)` is called
- THEN yt-dlp is invoked with `--yes-playlist` flag
- AND results are parsed into `Playlist` objects with title, thumbnail, and track count

#### Scenario: Playlist search with no results

- GIVEN a query that matches no playlists
- WHEN `search_playlists` is called
- THEN an empty list is returned without error

### Requirement: Playlist Resolve Method

YouTubeResolver MUST implement `resolve_playlist(url)` that extracts all tracks from a playlist URL.

#### Scenario: Resolve a valid playlist URL

- GIVEN a YouTube playlist URL containing `/playlist?list=`
- WHEN `resolve_playlist(url)` is called
- THEN yt-dlp dumps the full playlist JSON
- AND all entries are parsed into Track objects with metadata

#### Scenario: Resolve an invalid or private playlist

- GIVEN a playlist URL that is private, deleted, or malformed
- WHEN `resolve_playlist(url)` is called
- THEN a `ResolveError::PlaylistNotFound` or `ResolveError::PlaylistPrivate` is returned

#### Scenario: Large playlist pagination

- GIVEN a playlist with 200+ videos
- WHEN `resolve_playlist` processes it
- THEN all tracks are extracted without truncation

### Requirement: Defensive yt-dlp Output Parsing

YouTubeResolver MUST parse yt-dlp JSON output defensively to handle format variations across playlist types.

#### Scenario: Missing optional fields in entries

- GIVEN yt-dlp output where some entries lack `thumbnail` or `duration`
- WHEN the system parses the output
- THEN missing fields default to empty string or zero
- AND parsing continues without panic

#### Scenario: Mixed media types in playlist

- GIVEN a playlist containing both videos and non-audio entries
- WHEN the system parses the output
- THEN non-audio entries are filtered or gracefully skipped
- AND valid audio tracks are returned