# Home Recommendations Specification

## home-snapshot

**Acceptance Criteria**: one backend payload; `recentlyPlayed` and `recommendations` sections; backend-owned ordering; client renders without ranking logic.

### REQ-HS-1: Home snapshot command

The system MUST expose a backend `get_home_snapshot` command that returns one Home payload containing section data and render metadata for the Home route.

#### Scenario: Load complete snapshot

- GIVEN history, favorites, and local-library data exist
- WHEN Home requests `get_home_snapshot`
- THEN the response contains `recentlyPlayed` and `recommendations` sections in one payload
- AND the client can render Home without additional ranking requests

#### Scenario: Load snapshot with partial inputs

- GIVEN one or more recommendation inputs are empty
- WHEN Home requests `get_home_snapshot`
- THEN the command still succeeds with the available sections and state metadata

## home-recently-played

**Acceptance Criteria**: recency-ordered history; bounded list; playable rows; empty state when no plays exist.

### REQ-HRP-1: Recently played section

The system SHALL populate `recentlyPlayed` from persisted play history, ordered newest first. Each row MUST include the track data needed to display and replay the item.

#### Scenario: Show recent history

- GIVEN persisted history contains multiple plays
- WHEN Home snapshot is requested
- THEN `recentlyPlayed.items` returns the most recent plays first
- AND each item includes display and playback fields for its track

#### Scenario: Preserve repeated plays

- GIVEN the same track was played in separate sessions
- WHEN Home snapshot is requested
- THEN `recentlyPlayed` may include multiple entries for that track in recency order

## home-recommendations

**Acceptance Criteria**: recommendations come from Rust; use favorites, history, and local-library metadata; avoid stale repetition; return explainable discover items.

### REQ-HR-1: Recommendation assembly

The system MUST derive `recommendations` from favorites, play history, and local-library metadata. It SHOULD prioritize affinity signals from recently engaged artists or albums and MUST exclude items already present in `recentlyPlayed` when reasonable alternatives exist.

#### Scenario: Recommend from strong affinity

- GIVEN favorites and history show repeated engagement with related artists or albums
- WHEN Home snapshot is requested
- THEN `recommendations.items` prefer matching local-library candidates not already dominating the latest history

#### Scenario: Fallback to library discovery

- GIVEN affinity signals are too weak to rank confidently
- WHEN Home snapshot is requested
- THEN `recommendations.items` fall back to deterministic local-library discovery picks

### REQ-HR-2: Explainable recommendation metadata

Each recommendation item MUST include the media identity and display metadata required to render and open it. The response SHOULD expose a human-readable reason or section label that explains why the item was selected.

#### Scenario: Render discover cards

- GIVEN recommendation items are returned
- WHEN Home renders the recommendations section
- THEN each item can be shown as a track, album, or artist card with a stable action target

## home-empty-state

**Acceptance Criteria**: Home never breaks on empty or sparse data; search CTA remains available; sparse libraries still show best-effort content.

### REQ-HE-1: Empty and sparse Home states

The system MUST return a valid Home payload when history, favorites, or local-library inventory is empty or sparse. When no recently played or recommendation content is available, Home MUST expose an explicit empty state with a Search recovery path.

#### Scenario: Fully empty library

- GIVEN no history, no favorites, and no local tracks exist
- WHEN Home snapshot is requested
- THEN the payload marks both sections empty without failing
- AND Home can render an empty state with a Search CTA

#### Scenario: Sparse local-only library

- GIVEN only a small local library exists and engagement data is minimal
- WHEN Home snapshot is requested
- THEN Home still shows the best available recommendation content
- AND missing sections degrade gracefully instead of showing broken placeholders
