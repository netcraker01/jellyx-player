# Helix Screenshot Guide

Use this checklist when preparing screenshots for Flathub, Homebrew, winget pages, and GitHub releases.

## Goal

Create a small, stable set of screenshots that explain what Helix does without marketing fluff.

Helix is not a landing page product. The screenshots should show the real application clearly:

1. Search and discovery
2. Playback and queue
3. Playlists / organization
4. Visual identity / desktop app feel

## Minimum set for Flathub

Prepare **3 screenshots** first. Add 4-5 only if they add new information.

### Screenshot 1 — Search and results
- Route: `Search`
- Show: search box + results list
- Purpose: prove multi-source discovery exists

### Screenshot 2 — Playback / Now Playing
- Route: `Now Playing`
- Show: active track, queue, controls
- Purpose: prove this is a real desktop player, not just a search shell

### Screenshot 3 — Playlists / Library organization
- Route: `Playlists`
- Show: user playlists, cover thumbnails, recent organization flow
- Purpose: prove persistent organization features exist

## Recommended optional screenshots

### Screenshot 4 — Home screen
- Show: Discover + Recently Played + Recent Lists

### Screenshot 5 — Visualization mode
- Only include if it is visually stable and representative
- Avoid placeholder or half-finished modes

## Technical requirements

- Format: **PNG** or **WebP**
- Aspect ratio: **16:9** preferred
- Minimum size: **1280×720**
- Use a clean theme and real data
- Do not include debug windows, terminals, or dev overlays
- Do not include personal/private browsing history

## File names

Store them under `docs/screenshots/` with predictable names:

- `search-results.png`
- `now-playing.png`
- `playlists.png`
- `home.png`
- `visualizer.png`

## Hosting

For Flathub, the screenshot URLs must be reachable over HTTPS.

Simplest option:

1. Commit images into `docs/screenshots/`
2. Push to GitHub
3. Reference the raw GitHub URLs in `packaging/flatpak/com.helix.music.metainfo.xml`

Example URL pattern:

```text
https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/search-results.png
```

## XML snippet template

```xml
<screenshots>
  <screenshot type="default">
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

## Final check before Flathub review

- [ ] All screenshot URLs load in a browser without authentication
- [ ] Images are 16:9 and at least 1280×720
- [ ] Captions describe the feature, not the implementation
- [ ] Images match the current release behavior
- [ ] No personal data appears on screen
