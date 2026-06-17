# source-registry Specification

## Requirements

### Requirement: Source Registry Management

The system MUST provide a SourceRegistry that manages multiple SourceResolver implementations and queries them for search and resolve operations.

#### Scenario: Register and search multiple sources
- GIVEN SourceRegistry with YouTube and SoundCloud resolvers registered
- WHEN search("bohemian rhapsody") is called
- THEN registry queries each resolver and merges results into single Vec<Track>

#### Scenario: Individual source failure does not block others
- GIVEN SourceRegistry with YouTube (working) and SoundCloud (fails) resolvers
- WHEN search("query") is called
- THEN registry returns YouTube results and silently logs SoundCloud failure

#### Scenario: Resolve by source type
- GIVEN SourceRegistry with YouTube and SoundCloud resolvers
- WHEN resolve(Source::YouTube, "video_id") is called
- THEN registry routes to YouTubeResolver.resolve()