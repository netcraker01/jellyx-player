# favorites-management Specification

## Purpose

Let Now Playing reflect and change persisted favorite state.

## Requirements

### Requirement: Toggle Favorite State

The system MUST toggle the current track's favorite state: add it when absent and remove it when already favorited.

#### Scenario: Add favorite from Now Playing

- GIVEN the current track is not favorited
- WHEN the user activates the heart control
- THEN the track becomes favorited

#### Scenario: Remove existing favorite

- GIVEN the current track is already favorited
- WHEN the user activates the heart control
- THEN the track is removed from favorites

### Requirement: Persist Favorite State

The system SHALL persist favorite state between sessions and report the current track's saved state when playback UI loads.

#### Scenario: Restore persisted favorite

- GIVEN a track was favorited in a previous session
- WHEN the track appears in Now Playing after restart
- THEN the heart control shows favorited state

#### Scenario: Keep unique favorite entry

- GIVEN the same track is toggled on more than once across sessions
- WHEN favorites are listed
- THEN only one persisted favorite entry exists for that track
