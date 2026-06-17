# Delta for ipc-events

## ADDED Requirements

### Requirement: IB-009 track_changed event with Track payload

The system MUST emit a `track_changed` event via Tauri's event system whenever the current track changes. The event payload SHALL be a `Track` struct serialized in camelCase.

#### Scenario: Track changes during playback

- GIVEN PlaybackService starts playing a new track
- WHEN the current track changes
- THEN a `track_changed` event is emitted via AppHandle
- AND the payload is a `Track` struct serialized with `serde(rename_all = "camelCase")`

#### Scenario: Frontend subscribes to track_changed

- GIVEN the frontend calls `subscribeEvent<Track>('track_changed', cb)`
- WHEN a track_changed event is emitted from Rust
- THEN the callback receives a fully typed `Track` object

### Requirement: IB-010 state_changed event with PlaybackState payload

The system MUST emit a `state_changed` event whenever the playback state transitions (Playing, Paused, Stopped, Buffering). The payload SHALL be the `PlaybackState` enum serialized in PascalCase.

#### Scenario: Playback state transitions to Playing

- GIVEN PlaybackService transitions from Stopped to Playing
- WHEN the state changes
- THEN a `state_changed` event is emitted with payload `"Playing"`

#### Scenario: Playback state transitions to Paused

- GIVEN PlaybackService transitions from Playing to Paused
- WHEN the state changes
- THEN a `state_changed` event is emitted with payload `"Paused"`

### Requirement: IB-011 queue_updated event with Vec<Track> payload

The system MUST emit a `queue_updated` event whenever the playback queue changes (track added, removed, reordered). The payload SHALL be `Vec<Track>` serialized in camelCase.

#### Scenario: Track added to queue

- GIVEN a track is added to the queue via `add_to_queue`
- WHEN the queue is modified
- THEN a `queue_updated` event is emitted with the full queue as `Vec<Track>`

### Requirement: IB-012 progress_tick event with position and duration

The system MUST emit a `progress_tick` event periodically during playback with current position and duration. The payload SHALL be a `ProgressTick` struct with `position: f64` and `duration: f64` fields, serialized in camelCase.

#### Scenario: Progress tick during playback

- GIVEN a track is playing
- WHEN a progress tick fires
- THEN a `progress_tick` event is emitted with `{ position: 45.2, duration: 240.0 }`

#### Scenario: Frontend receives typed progress

- GIVEN the frontend calls `subscribeEvent<ProgressTick>('progress_tick', cb)`
- WHEN a progress_tick event is emitted
- THEN the callback receives `{ position: 45.2, duration: 240.0 }`

### Requirement: IB-013 All events use camelCase field names

All event payload structs MUST use `#[serde(rename_all = "camelCase")]` to ensure field names match the TypeScript frontend type definitions exactly.

#### Scenario: Event payload field naming

- GIVEN a `ProgressTick` struct with field `position: f64`
- WHEN the event is emitted and serialized
- THEN the JSON key is `"position"` (camelCase matches TypeScript)
- AND the `Track` struct fields (`source_id`, `stream_url`) serialize as `"sourceId"`, `"streamUrl"`