# Persistence Specification

## Purpose
SQLite-backed storage layer providing durable data for the library domain.

## Requirements

### Requirement: Database Initialization

The system SHALL create the SQLite database at `~/.local/share/helix/helix.db` on first launch. The parent directory MUST be created if it does not exist.

#### Scenario: Fresh launch creates DB

- GIVEN no database file exists
- WHEN the Database is initialized
- THEN `~/.local/share/helix/helix.db` is created
- AND all required tables exist (favorites, history)

#### Scenario: Existing DB reused

- GIVEN database file already exists
- WHEN the Database is initialized
- THEN existing data is preserved
- AND no tables are recreated

### Requirement: Schema Versioning

The system SHALL track schema version. Schema changes MUST be applied via migrations, never destructive to user data.

#### Scenario: Schema version tracked

- GIVEN a newly created database
- THEN a `schema_version` metadata value equals the current version number

### Requirement: Thread Safety

The system SHALL allow concurrent reads. Writes MUST be serialized via SQLite WAL mode.

#### Scenario: Concurrent reads

- GIVEN multiple LibraryService methods read simultaneously
- THEN no deadlock or data corruption occurs