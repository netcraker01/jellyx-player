# play-history Specification

## Purpose

Record recently played tracks consistently across all playback sources.

## Requirements

### Requirement: Record Playback Starts

The system MUST record one history entry when a track starts playback from local, YouTube, or SoundCloud sources.

#### Scenario: Record cross-source start

- GIVEN a playable track from any supported source
- WHEN playback starts for that track
- THEN one history entry is stored for that start

#### Scenario: Ignore seek and resume

- GIVEN a track already recorded for its current playback start
- WHEN the user seeks or resumes that same playback session
- THEN no additional history entry is stored

### Requirement: Keep Bounded Recent History

The system SHALL keep the 100 most recent history entries, ordered newest first, and evict the oldest entry when the limit is exceeded.

#### Scenario: Return recent entries

- GIVEN multiple plays were recorded
- WHEN recent history is requested
- THEN entries are returned from most recent to oldest

#### Scenario: Evict oldest entry

- GIVEN 100 history entries already exist
- WHEN a new playback start is recorded
- THEN the oldest entry is removed and the new entry is kept
