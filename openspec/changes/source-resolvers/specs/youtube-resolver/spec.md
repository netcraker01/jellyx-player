# youtube-resolver Specification

## Requirements

### Requirement: YouTube Search

The system MUST search YouTube via yt-dlp using `ytsearch<N>:<query>` format and return results as `Vec<Track>`.

#### Scenario: Successful YouTube search
- GIVEN yt-dlp is installed and available on PATH
- WHEN user searches with query "bohemian rhapsody"
- THEN system invokes yt-dlp `ytsearch5:bohemian rhapsody --dump-json --no-download --no-playlist`
- AND returns Vec<Track> with up to 5 tracks, each with source=YouTube

#### Scenario: YouTube search with no results
- GIVEN yt-dlp is installed
- WHEN user searches with query that returns no results
- THEN system returns empty Vec<Track>

#### Scenario: YouTube search when yt-dlp is not installed
- GIVEN yt-dlp is NOT available on PATH
- WHEN user searches YouTube
- THEN system returns SourceError::DependencyMissing

### Requirement: YouTube Stream Resolution

The system MUST resolve a YouTube stream URL from a video ID via yt-dlp.

#### Scenario: Resolve YouTube video to stream URL
- GIVEN yt-dlp is installed and a valid YouTube video ID
- WHEN system resolves ID "dQw4w9WgXcQ"
- THEN returns Track with stream_url populated

#### Scenario: Resolve fails for invalid video ID
- GIVEN yt-dlp is installed
- WHEN system resolves an invalid or deleted video ID
- THEN system returns SourceError::ResolveError