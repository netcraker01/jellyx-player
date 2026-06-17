# Album Art Cache Specification

## Purpose

Manages extraction of embedded album art from local audio files, filesystem caching of extracted images, and serving them to the frontend via Tauri's asset protocol.

## Requirements

### Requirement: Metadata Revision Consumption

The scanner MUST consume ALL metadata revisions from Symphonia's `MetadataQueue` by calling `pop()` repeatedly until `is_latest()` returns true. The scanner MUST NOT stop at `current()`.

#### Scenario: Single revision file

- GIVEN a file with one metadata revision
- WHEN the scanner extracts metadata
- THEN all tags and visuals are read from that single revision
- AND no revision data is missed

#### Scenario: Multi-revision file (ID3v2 + VorbisComment)

- GIVEN a FLAC file with both ID3v2 and VorbisComment metadata revisions
- WHEN the scanner iterates revisions via `pop()` loop
- THEN tags from ALL revisions are merged (later revision overwrites earlier for same tag key)
- AND visuals from ALL revisions are inspected

### Requirement: Front Cover Visual Extraction

The scanner MUST iterate `revision.media.visuals` across all consumed revisions and select the first `Visual` whose `StandardVisualKey` equals `FrontCover`. The scanner MUST extract the `data` bytes and `media_type` string from the selected visual.

#### Scenario: File with FrontCover visual

- GIVEN an MP3 file with an embedded `FrontCover` JPEG image
- WHEN the scanner processes its visuals
- THEN the scanner extracts that image's raw bytes and media type `image/jpeg`

#### Scenario: File with visuals but no FrontCover

- GIVEN an OGG file with `BackCover` and `BandLogo` visuals but no `FrontCover`
- WHEN the scanner processes its visuals
- THEN `Track.thumbnail` remains `None`
- AND no cache file is written

#### Scenario: File with no visuals at all

- GIVEN a WAV file with no embedded visuals
- WHEN the scanner processes metadata
- THEN `Track.thumbnail` remains `None`
- AND scanning completes without error

### Requirement: Filesystem Art Cache Write

The system MUST write extracted art bytes to `~/.local/share/helix/art/{sha256_hash}.{ext}` where `{sha256_hash}` is the SHA-256 hex digest of the art bytes and `{ext}` is derived from `media_type` (`image/jpeg` → `jpg`, `image/png` → `png`). The system MUST NOT overwrite an existing cache file with identical content hash.

#### Scenario: Cache new JPEG art

- GIVEN extracted FrontCover art of 50KB with media type `image/jpeg`
- WHEN the system writes the cache file
- THEN a file at `~/.local/share/helix/art/{hash}.jpg` is created with the exact art bytes

#### Scenario: Duplicate art across tracks (same image)

- GIVEN two tracks sharing the same album art bytes
- WHEN both are scanned
- THEN only one cache file exists (second write is skipped — same hash)

#### Scenario: Unsupported media type

- GIVEN a FrontCover visual with media type `image/webp`
- WHEN the system derives the extension
- THEN the system SHALL fall back to `bin` extension
- AND the cache file is written as `{hash}.bin`

### Requirement: Cache Directory Initialization

The system MUST create the `~/.local/share/helix/art/` directory at application startup if it does not exist. The system MUST NOT fail if the directory already exists.

#### Scenario: First launch — directory missing

- GIVEN the art cache directory does not exist
- WHEN the application starts
- THEN the directory is created with default permissions

#### Scenario: Subsequent launch — directory exists

- GIVEN the art cache directory already exists
- WHEN the application starts
- THEN no error occurs and no files are modified

### Requirement: Track Thumbnail Population

After successful art extraction and cache write, the scanner MUST set `Track.thumbnail` to the absolute filesystem path of the cache file. If no art is extracted, `Track.thumbnail` MUST remain `None`.

#### Scenario: Art extracted — thumbnail set to cache path

- GIVEN a track with extractable FrontCover art
- WHEN cache file is written at `/home/user/.local/share/helix/art/abc123.jpg`
- THEN `Track.thumbnail` equals that absolute path string

#### Scenario: No art — thumbnail remains None

- GIVEN a track with no embedded visuals
- WHEN the scanner completes metadata extraction
- THEN `Track.thumbnail` is `None`

### Requirement: Asset Protocol Scope

The Tauri application MUST configure `assetProtocol.scope` to allow reads from the `$APPDATA/art/` directory. The frontend MUST use `convertFileSrc()` to convert cache file paths into loadable `<img>` URLs.

#### Scenario: Frontend loads cached album art

- GIVEN `Track.thumbnail` contains a cache file path
- WHEN the frontend calls `convertFileSrc(track.thumbnail)`
- THEN the returned URL loads the image via Tauri's asset protocol

#### Scenario: Track with no thumbnail

- GIVEN `Track.thumbnail` is `None`
- WHEN the frontend renders the track
- THEN a placeholder image is shown (no broken image)