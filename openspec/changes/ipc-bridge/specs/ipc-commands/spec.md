# Delta for ipc-commands

## ADDED Requirements

### Requirement: IB-001 Play command routes through PlaybackService

The system MUST route the `play` Tauri command through `PlaybackService.play()`, passing a URL string parameter. The command SHALL accept a `url: String` parameter and delegate to the service layer.

#### Scenario: Play command invoked with URL

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `play` with `{ url: "https://stream.test/audio.mp3" }`
- THEN PlaybackService.play() is called with the URL
- AND the result (success or AppError) is returned to the frontend

#### Scenario: Play command with missing URL

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `play` without a URL parameter
- THEN an AppError with code `VALIDATION_ERROR` is returned

### Requirement: IB-002 Pause command routes through PlaybackService

The system MUST route the `pause` Tauri command through `PlaybackService.pause()`.

#### Scenario: Pause command invoked

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `pause`
- THEN PlaybackService.pause() is called
- AND the result is returned to the frontend

### Requirement: IB-003 Next and Previous commands route through PlaybackService

The system MUST provide `next` and `previous` Tauri commands that delegate to `PlaybackService.next()` and `PlaybackService.previous()` respectively.

#### Scenario: Next command invoked

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `next`
- THEN PlaybackService.next() is called
- AND the result is returned to the frontend

#### Scenario: Previous command invoked

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `previous`
- THEN PlaybackService.previous() is called
- AND the result is returned to the frontend

#### Scenario: Next with empty queue

- GIVEN PlaybackService has an empty queue
- WHEN the frontend invokes `next`
- THEN an AppError with code `PLAYBACK_ERROR` and details "queue is empty" is returned

### Requirement: IB-004 Volume control command routes through PlaybackService

The system MUST route the `set_volume` Tauri command through `PlaybackService.set_volume()`. The command SHALL accept a `volume: f32` parameter.

#### Scenario: Set volume to valid level

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `set_volume` with `{ volume: 0.5 }`
- THEN PlaybackService.set_volume(0.5) is called
- AND success is returned

### Requirement: IB-005 Search command returns Vec<Track>

The system MUST route the `search` Tauri command through `PlaybackService.search()`. The command SHALL accept a `query: String` parameter and return `Vec<Track>`.

#### Scenario: Search with valid query

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `search` with `{ query: "queen" }`
- THEN PlaybackService.search("queen") is called
- AND `Vec<Track>` is returned serialized in camelCase

#### Scenario: Search with empty query

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `search` with an empty query
- THEN an AppError with code `VALIDATION_ERROR` is returned

### Requirement: IB-006 Add to queue command

The system MUST provide an `add_to_queue` Tauri command that routes through `PlaybackService.add_to_queue()`. The command SHALL accept a `trackId: String` parameter.

#### Scenario: Add track to queue

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `add_to_queue` with `{ trackId: "track-1" }`
- THEN PlaybackService.add_to_queue("track-1") is called
- AND success is returned

### Requirement: IB-007 Get queue command returns Vec<Track>

The system MUST provide a `get_queue` Tauri command that routes through `PlaybackService.get_queue()` and returns `Vec<Track>`.

#### Scenario: Get current queue

- GIVEN a PlaybackService is available via AppState
- WHEN the frontend invokes `get_queue`
- THEN PlaybackService.get_queue() is called
- AND `Vec<Track>` is returned in camelCase

### Requirement: IB-008 Command parameters use camelCase matching TypeScript

All `#[tauri::command]` function parameter names MUST use camelCase (Rust snake_case with `rename_all` or `serde` attributes) so that Tauri's IPC serialization matches the TypeScript frontend exactly.

#### Scenario: Parameter naming consistency

- GIVEN a Tauri command `set_volume(volume: f32)`
- WHEN the frontend invokes `invoke('set_volume', { volume: 0.5 })`
- THEN the parameter is correctly deserialized from camelCase JSON to Rust snake_case