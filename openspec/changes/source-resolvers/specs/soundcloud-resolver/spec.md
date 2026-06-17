# soundcloud-resolver Specification

## Requirements

### Requirement: SoundCloud Search

The system MUST search SoundCloud via yt-dlp using `scsearch<N>:<query>` format and return results as `Vec<Track>`.

#### Scenario: Successful SoundCloud search
- GIVEN yt-dlp is installed
- WHEN user searches with query "ambient mix"
- THEN system invokes yt-dlp `scsearch5:ambient mix --dump-json --no-download --no-playlist`
- AND returns Vec<Track> with up to 5 tracks, each with source=SoundCloud

#### Scenario: SoundCloud search when yt-dlp not installed
- GIVEN yt-dlp is NOT available on PATH
- WHEN user searches SoundCloud
- THEN system returns SourceError::DependencyMissing

### Requirement: SoundCloud Stream Resolution

The system MUST resolve a SoundCloud stream URL from a track URL via yt-dlp.

#### Scenario: Resolve SoundCloud track to stream URL
- GIVEN yt-dlp is installed and valid SoundCloud track URL
- WHEN system resolves the URL
- THEN returns Track with stream_url populated