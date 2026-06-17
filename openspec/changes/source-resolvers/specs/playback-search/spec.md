# playback-search Delta Spec

## MODIFIED Requirements

### Requirement: Multi-Source Search

The system MUST query all registered source resolvers when performing a search, not just YouTube.

(Previously: PlaybackService.search() hardcoded YouTubeResolver)

#### Scenario: Search queries all sources
- GIVEN PlaybackService with YouTube and SoundCloud in registry
- WHEN search("test") is called
- THEN results include tracks from both YouTube and SoundCloud

#### Scenario: Empty query validation
- GIVEN any state
- WHEN search("") is called
- THEN returns ValidationError::EmptyQuery