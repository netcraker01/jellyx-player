# youtube-ipc-commands Specification

## Purpose

Tauri IPC commands that expose search, play, resolve, and playlist operations to the Svelte frontend. All commands delegate to backend services; the frontend is a "dumb client."

## Requirements

### Requirement: Search Command

The system MUST provide a `search` IPC command that accepts a query string and source filter, returning track and playlist results.

#### Scenario: Search with source filter

- GIVEN the frontend sends a search query with source type YouTube
- WHEN the `search` command is invoked
- THEN the backend delegates to the appropriate resolver
- AND returns matching tracks and playlists to the frontend

#### Scenario: Search with no results

- GIVEN a query that matches nothing
- WHEN the `search` command is invoked
- THEN the backend returns an empty result list without error

### Requirement: Play Track Command

The system MUST provide a `play_track` IPC command that starts playback of a given track, routing to local or stream playback automatically.

#### Scenario: Play a local track

- GIVEN a track with source type local
- WHEN `play_track` is invoked
- THEN playback uses the local file path

#### Scenario: Play a remote track

- GIVEN a track with source type YouTube
- WHEN `play_track` is invoked
- THEN playback resolves the stream URL and uses `play_stream()`

#### Scenario: Play track with missing yt-dlp

- GIVEN yt-dlp is not installed on the system
- WHEN `play_track` is invoked for a remote track
- THEN a clear error is returned to the frontend (no panic)

### Requirement: Play Playlist Command

The system MUST provide a `play_playlist` IPC command that enqueues all tracks from a playlist and begins playback from the first track.

#### Scenario: Play a playlist

- GIVEN a resolved Playlist with multiple tracks
- WHEN `play_playlist` is invoked
- THEN all tracks are enqueued in order
- AND playback starts from the first track

#### Scenario: Play empty playlist

- GIVEN a Playlist with zero tracks
- WHEN `play_playlist` is invoked
- THEN the system returns an error indicating the playlist is empty

### Requirement: Resolve Track Command

The system MUST provide a `resolve_track` IPC command that resolves a track's stream URL without starting playback.

#### Scenario: Resolve a track for preview

- GIVEN a track that needs stream URL resolution
- WHEN `resolve_track` is invoked
- THEN the stream URL is returned without starting playback
- AND the URL can be used for pre-buffering or metadata extraction

#### Scenario: Resolve fails

- GIVEN a track whose stream URL cannot be resolved
- WHEN `resolve_track` is invoked
- THEN a `ResolveError` is returned to the frontend

### Requirement: Startup Dependency Check

The system SHOULD check for yt-dlp availability at application startup and warn the user if missing.

#### Scenario: yt-dlp missing at startup

- GIVEN yt-dlp is not on the system PATH
- WHEN the application starts
- THEN a user-facing warning is emitted
- AND remote source features are marked unavailable