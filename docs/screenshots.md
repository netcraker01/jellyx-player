# Helix Screenshot Guide

Use this checklist when preparing screenshots for the GitHub README, Flathub, Homebrew, winget pages, and GitHub releases.

## Goal

Create a small, stable set of screenshots that explain what Helix does at a glance. The screenshots should sell the user experience, not the implementation details.

Helix is a privacy-first desktop music player. The screenshots should show the real application clearly:

1. Home and discovery
2. Search and results
3. Playback and queue
4. Playlists / organization

## Minimum set

Prepare **3-4 screenshots** first. Add a fifth only if it adds new information.

### Screenshot 1 — Home
- File: `docs/screenshots/home.png`
- Show: Discover moods + Recently Played
- Purpose: show the entry point and personalized feel

### Screenshot 2 — Search and results
- File: `docs/screenshots/search-results.png`
- Show: search box + multi-source results list
- Purpose: prove multi-source discovery exists

### Screenshot 3 — Playback / Now Playing
- File: `docs/screenshots/now-playing.png`
- Show: active track, album art, queue, controls
- Purpose: prove this is a real desktop player, not just a search shell

### Screenshot 4 — Playlists / Library organization
- File: `docs/screenshots/playlists.png`
- Show: user playlists, artist favorites, recent organization flow
- Purpose: prove persistent organization features exist

## Technical requirements

- Format: **PNG** or **WebP**
- Aspect ratio: **16:9** preferred
- Minimum size: **1280×720**
- Use a clean theme and real data
- Do not include debug windows, terminals, or dev overlays
- Do not include personal/private browsing history

## Hosting

For Flathub, the screenshot URLs must be reachable over HTTPS.

Commit the images into `docs/screenshots/` and reference the raw GitHub URLs:

```text
https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/home.png
https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/search-results.png
https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/now-playing.png
https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/playlists.png
```

## README usage

The [README.md](../README.md) embeds these screenshots with relative paths so they render on GitHub and in any local markdown preview.

## XML snippet template

```xml
<screenshots>
  <screenshot type="default">
    <caption>Home screen with discovery and recently played tracks</caption>
    <image>https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/home.png</image>
  </screenshot>
  <screenshot>
    <caption>Search and stream music from multiple sources</caption>
    <image>https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/search-results.png</image>
  </screenshot>
  <screenshot>
    <caption>Native playback controls and queue</caption>
    <image>https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/now-playing.png</image>
  </screenshot>
  <screenshot>
    <caption>Create and manage playlists locally</caption>
    <image>https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/playlists.png</image>
  </screenshot>
</screenshots>
```

## Final check before publishing

- [ ] All screenshot URLs load in a browser without authentication
- [ ] Images are 16:9 and at least 1280×720
- [ ] Captions describe the feature, not the implementation
- [ ] Images match the current release behavior
- [ ] No personal data appears on screen
- [ ] README renders correctly on GitHub
