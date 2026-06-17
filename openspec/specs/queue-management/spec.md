# queue-management Specification

## Purpose

Make queue mutations backend-owned so remove, clear, and play-next keep playback and queue UI in sync.

## Requirements

### Requirement: Remove Queued Track

The system MUST remove a queued track by ID and return a full queue snapshot that preserves valid queue indexing.

#### Scenario: Remove track before current position

- GIVEN a queue with a currentIndex greater than the removed track index
- WHEN that earlier queued track is removed
- THEN the returned QueueState sets currentIndex to the previous value minus one

#### Scenario: Remove current track

- GIVEN a queued track is the current track
- WHEN that track is removed
- THEN playback stops, current track is cleared, and QueueState.currentIndex becomes null

#### Scenario: Remove track after current position

- GIVEN a queue with a currentIndex lower than the removed track index
- WHEN that later queued track is removed
- THEN the returned QueueState keeps currentIndex unchanged

### Requirement: Clear Queue

The system MUST clear every queued track, stop playback, clear the current track, and emit an empty QueueState snapshot.

#### Scenario: Clear non-empty queue

- GIVEN the queue contains tracks
- WHEN clear queue is requested
- THEN playback stops and QueueState returns tracks as empty with currentIndex null

#### Scenario: Clear already-empty queue

- GIVEN the queue is already empty
- WHEN clear queue is requested
- THEN the system returns an empty QueueState snapshot without reintroducing state

### Requirement: Insert Track As Play Next

The system MUST insert a selected track immediately after the current queue position and return a full snapshot whose currentIndex reflects the inserted position.

#### Scenario: Insert after current track

- GIVEN a queue with a currentIndex and a resolvable selected track
- WHEN play next is requested for that track
- THEN the track is inserted at currentIndex + 1 and QueueState.currentIndex points to the inserted track

#### Scenario: Repeat play-next requests keep newest choice next-up

- GIVEN a queue with a currentIndex and two play-next requests in sequence
- WHEN the second request is processed
- THEN the second inserted track becomes the new currentIndex position in QueueState

### Requirement: Emit Queue Snapshots After Queue Mutations

The system SHALL emit `queue-updated` with the full QueueState after queue mutations and after next/previous change the active index.

#### Scenario: Mutation emits complete snapshot

- GIVEN remove, clear, or play-next changes the queue
- WHEN the mutation completes successfully
- THEN the emitted QueueState includes tracks, currentIndex, shuffle, repeatMode, and playedIndices

#### Scenario: Navigation emits refreshed snapshot

- GIVEN next or previous changes the active queue index
- WHEN playback navigation completes
- THEN the frontend receives a new queue-updated event for the resulting QueueState
