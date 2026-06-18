# Artist/Album Detail Views Specification

## media-search

**Acceptance Criteria**: grouped songs/artists/albums results; optional type filter; song actions preserved; artist/album results navigate.

### REQ-MS-1: Grouped search results

The system MUST return search results in separate `songs`, `artists`, and `albums` groups for any non-empty query. Each group item MUST include the identifier and display metadata needed to render the result and open playback or detail navigation.

#### Scenario: Query returns mixed result types

- GIVEN library metadata contains matching songs, artists, and albums
- WHEN the user searches for `daft`
- THEN the response contains separate `songs`, `artists`, and `albums` groups
- AND each group preserves only items of its own type

#### Scenario: Query returns no matches

- GIVEN no indexed media matches `zzzz`
- WHEN the user searches for `zzzz`
- THEN the response returns empty groups without failing the request

### REQ-MS-2: Optional type filter

The system SHALL support an optional search type filter of `songs`, `artists`, or `albums`. When omitted, the system MUST search all supported types.

#### Scenario: Artist-only filtering

- GIVEN matching songs, artists, and albums exist for `queen`
- WHEN the user searches for `queen` with filter `artists`
- THEN the response returns matching artists only
- AND `songs` and `albums` are empty

### REQ-MS-3: Result actions and navigation

Song results MUST remain directly playable and queueable. Artist and album results MUST open `/artist/:id` and `/album/:id` respectively.

#### Scenario: Open artist from grouped results

- GIVEN grouped search results include an artist with id `artist-1`
- WHEN the user selects that artist result
- THEN the app navigates to `/artist/artist-1`

## artist-detail-view

**Acceptance Criteria**: `/artist/:id` route; header with name/image; top songs list; albums list; navigation from Search and Now Playing.

### REQ-AD-1: Artist detail content

The system MUST render `/artist/:id` using backend-provided artist detail data. The view MUST show the artist name, an image or placeholder, a top songs list, and the artist's albums.

#### Scenario: Render artist detail page

- GIVEN artist `artist-1` has metadata, top songs, and albums
- WHEN the user opens `/artist/artist-1`
- THEN the page shows the artist header, top songs section, and albums section

#### Scenario: Artist image is unavailable

- GIVEN artist `artist-2` has no image
- WHEN the user opens `/artist/artist-2`
- THEN the page renders a placeholder image state without failing the route

### REQ-AD-2: Artist detail interactions

Top songs MUST be directly playable. Album items MUST navigate to the related album detail route. Now Playing SHOULD expose an action that opens the current track's artist when artist detail data is resolvable.

#### Scenario: Open artist from Now Playing

- GIVEN a current track is associated with artist `artist-1`
- WHEN the user activates `Open artist`
- THEN the app navigates to `/artist/artist-1`

## album-detail-view

**Acceptance Criteria**: `/album/:id` route; cover/title/artist; ordered tracks; play-full-album action; navigation from Search and Now Playing.

### REQ-AL-1: Album detail content

The system MUST render `/album/:id` using backend-provided album detail data. The view MUST show album cover or placeholder, album title, artist name, and the album track list in playback order.

#### Scenario: Render ordered album tracks

- GIVEN album `album-1` contains ordered track metadata
- WHEN the user opens `/album/album-1`
- THEN the page shows the album header and the tracks in album order

#### Scenario: Unknown album id

- GIVEN no album exists for `album-missing`
- WHEN the user opens `/album/album-missing`
- THEN the app shows a recoverable not-found state instead of stale or partial data

### REQ-AL-2: Full album playback and navigation

The album view MUST provide a play-full-album action that starts playback from the first album track and queues the remaining album tracks in order. Now Playing SHOULD expose an action that opens the current track's album when album detail data is resolvable.

#### Scenario: Play full album

- GIVEN album `album-1` has three ordered tracks
- WHEN the user selects `Play album`
- THEN playback starts with track one
- AND tracks two and three become the next queued tracks in album order