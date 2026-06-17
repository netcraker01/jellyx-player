# Favorites Specification

## Purpose
User-curated collection of saved tracks. Favorites persist across sessions via SQLite.

## Requirements

### Requirement: Add Track to Favorites

The system SHALL add a Track to the favorites collection. Duplicate Track IDs MUST be rejected with `ALREADY_EXISTS`.

#### Scenario: Add a new track

- GIVEN no track with ID "t1" exists in favorites
- WHEN `add_favorite` is called with a Track having id "t1"
- THEN the track is stored in the database
- AND the track appears in `get_favorites` results

#### Scenario: Add duplicate track

- GIVEN track with ID "t1" already exists in favorites
- WHEN `add_favorite` is called with a Track having id "t1"
- THEN the system returns `ALREADY_EXISTS` error

### Requirement: Remove Track from Favorites

The system SHALL remove a Track from favorites by its Helix ID.

#### Scenario: Remove existing favorite

- GIVEN track with ID "t1" exists in favorites
- WHEN `remove_favorite` is called with "t1"
- THEN the track is removed from the database
- AND the track no longer appears in `get_favorites` results

#### Scenario: Remove non-existent favorite

- GIVEN no track with ID "t99" exists in favorites
- WHEN `remove_favorite` is called with "t99"
- THEN the system returns `NOT_FOUND` error

### Requirement: List Favorites

The system SHALL return all favorited tracks ordered by most recently added first.

#### Scenario: List when favorites exist

- GIVEN tracks "t1" and "t2" are favorited (t2 added after t1)
- WHEN `get_favorites` is called
- THEN the result contains both tracks with "t2" first

#### Scenario: List when no favorites

- GIVEN no tracks are favorited
- WHEN `get_favorites` is called
- THEN the result is an empty list