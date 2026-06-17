# playback-modes Specification

## Purpose

Provide backend-owned shuffle and repeat behavior without changing visible queue order.

## Requirements

### Requirement: Preserve Queue Order During Shuffle

The system MUST keep the displayed queue order unchanged while shuffle selects the next track randomly from remaining unplayed tracks.

#### Scenario: Shuffle picks next without reordering

- GIVEN shuffle mode is on and multiple unplayed queue tracks remain
- WHEN the current track ends or next is requested
- THEN the next track is chosen randomly from remaining unplayed tracks
- AND the visible queue order stays unchanged

#### Scenario: Exhaust shuffled queue

- GIVEN shuffle mode is on and every queue track has been played once
- WHEN repeat-all is off and the current track ends
- THEN playback stops instead of replaying a finished shuffled queue

### Requirement: Cycle Repeat Modes

The system SHALL cycle repeat mode in the order off → repeat all → repeat one → off and apply the selected mode at track end.

#### Scenario: Repeat all loops queue

- GIVEN repeat-all mode is active and the last queue track finishes
- WHEN playback advances
- THEN playback continues from the start of the queue

#### Scenario: Repeat one replays current track

- GIVEN repeat-one mode is active
- WHEN the current track finishes
- THEN the same track starts again
