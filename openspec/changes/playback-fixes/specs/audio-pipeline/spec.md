# Delta for audio-pipeline

## MODIFIED Requirements

### Requirement: PF-001 Seek restarts decoder at new position

PlaybackService.seek() MUST stop the current decoder at the old position, seek SymphoniaDecoder to the requested position, restart the decoder thread, and emit a progress-tick event with the new position.

(Previously: seek() only updated InternalState.position without restarting the decoder)

#### Scenario: Seek while playing

- GIVEN audio is Playing at position 30s
- WHEN seek(60.0) is called
- THEN the decoder SHALL seek to 60.0s and decode from that position
- AND a progress-tick event SHALL be emitted with position=60.0

#### Scenario: Seek while paused

- GIVEN audio is Paused at position 30s
- WHEN seek(60.0) is called
- THEN the decoder SHALL seek to 60.0s
- AND PlaybackState SHALL remain Paused
- AND a progress-tick event SHALL be emitted with position=60.0

#### Scenario: Seek beyond duration clamped

- GIVEN audio is Playing, duration=180.0
- WHEN seek(200.0) is called
- THEN the position SHALL be clamped to 180.0 (duration)
- AND the decoder SHALL seek to 180.0s

#### Scenario: Seek to negative clamped to zero

- GIVEN audio is Playing
- WHEN seek(-5.0) is called
- THEN the position SHALL be clamped to 0.0
- AND the decoder SHALL seek to 0.0s

### Requirement: PF-002 set_volume forwards to CpalBackend

PlaybackService.set_volume() MUST update InternalState.volume AND propagate the volume level to CpalBackend's software volume control so the audible output changes.

(Previously: set_volume() only updated InternalState.volume without forwarding to CpalBackend)

#### Scenario: Volume change while playing

- GIVEN audio is Playing at volume 1.0
- WHEN set_volume(0.5) is called
- THEN InternalState.volume SHALL be 0.5
- AND CpalBackend volume SHALL be set to 0.5
- AND the audible output level SHALL decrease

#### Scenario: Volume clamped to valid range

- GIVEN audio is Playing
- WHEN set_volume(1.5) is called
- THEN the effective volume SHALL be clamped to 1.0 in both InternalState and CpalBackend

#### Scenario: Volume change while stopped

- GIVEN audio is Stopped
- WHEN set_volume(0.7) is called
- THEN InternalState.volume SHALL be 0.7
- AND the volume SHALL take effect when playback next starts

### Requirement: PF-003 Zero compiler warnings

The project SHALL compile with zero warnings after `cargo check`. Dead code that serves a documented future purpose SHALL be annotated with `#[allow(dead_code)]`. Truly dead code (unused imports, unnecessary mut, unreachable items) SHALL be removed.

(Previously: 34 compiler warnings from unused imports, dead code, unnecessary mut)

#### Scenario: Clean build

- GIVEN the project source code
- WHEN `cargo check` is executed
- THEN zero warnings SHALL be reported

#### Scenario: Forward-looking code preserved with allow

- GIVEN a struct or method planned for future use (e.g., Artist, Album, VisualizerConfig)
- WHEN it would otherwise trigger a dead_code warning
- THEN it SHALL be annotated with `#[allow(dead_code)]` rather than deleted