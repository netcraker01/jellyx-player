# History Specification

## Purpose
Timestamped record of played tracks. History persists across sessions via SQLite.

## Requirements

### Requirement: Record Play Event

The system SHALL record a play event when a track starts playing, storing the Track data and a UTC timestamp.

#### Scenario: Record first play

- GIVEN no history exists
- WHEN a track with ID "t1" starts playing
- THEN a history entry is created with track "t1" and current timestamp
- AND `get_history` returns the entry

#### Scenario: Record repeat play

- GIVEN track "t1" was played before
- WHEN track "t1" plays again
- THEN a NEW history entry is created (not deduplicated) with updated timestamp

### Requirement: List History

The system SHALL return play history ordered by most recent first.

#### Scenario: List with entries

- GIVEN tracks "t1" and "t2" were played (t2 more recently)
- WHEN `get_history` is called
- THEN results contain both entries with "t2" first

#### Scenario: List with limit

- GIVEN 100 history entries exist
- WHEN `get_history` is called
- THEN at most 50 entries are returned (default limit)

### Requirement: Clear History

The system SHALL remove all history entries.

#### Scenario: Clear all history

- GIVEN history entries exist
- WHEN `clear_history` is called
- THEN all entries are removed
- AND `get_history` returns empty list