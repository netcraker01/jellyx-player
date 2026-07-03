# Helix Player — Launch & Promotion Plan

> **Version:** v0.2.1 (2026-07-02) · **Status:** Alpha
> **Promotion-ready platforms ✅** — Linux (AppImage, .deb, .rpm), Windows (MSI, NSIS, portable), macOS Apple Silicon
> **Intel macOS** is intentionally deferred for this alpha
> **Repo:** https://github.com/netcraker01/helix
> **Downloads:** https://github.com/netcraker01/helix/releases

---

## Messaging & Taglines

### Primary tagline (use everywhere)
> **Privacy-first desktop music player. Stream, visualize, discover. No accounts, no ads, no tracking.**

### Elevator pitch (1 sentence)
> Helix is a native open-source music player built with Rust + Tauri that streams from YouTube and SoundCloud, plays local files, and features 7 real-time audio visualizers — no sign-up required.

### Key differentiators
1. **Native desktop app** — Not a web wrapper. Built with Rust + Tauri (WebKitGTK on Linux, WebView2 on Windows, WKWebView on macOS).
2. **Real-time audio visualizers** — 7 modes (Bars, Wave, Mirror, Radial, Aurora, Grid, Tunnel) powered by FFT audio data from Symphonia.
3. **Stream from YouTube & SoundCloud** — No separate accounts, no browser tabs, no ads.
4. **Privacy-first** — No accounts, no cookies, no tracking, no telemetry. Not even for feature analytics.
5. **Bilingual UI** — English and Spanish built in.

### AI transparency note
Use this when people ask whether the project is AI-built:

> Helix was designed and directed by me using a strict software engineering workflow. I use AI to write code, but not through loose vibe coding. The project is guided with continuous human-in-the-loop review, rigid harnesses, deterministic validation, and requirement-driven iteration. The implementation is AI-assisted, the engineering decisions, constraints, and acceptance criteria are human-led.

Short version:

> This is AI-assisted code under strict human engineering control, not autonomous vibe-coded slop.

---

## Assets Ready

| Asset | Location | Usage |
|-------|----------|-------|
| Logo wide | `assets/brand/logo-wide.png` | Banner for posts |
| App icon | `assets/brand/app-icon.png` | Avatar, thumbnails |
| Icon SVG | `assets/brand/icon.svg` | Vector for scaling |
| Brand sheet | `assets/brand/brand-sheet.png` | Brand overview |
| Home screenshot | `docs/screenshots/home.png` | Posts, README |
| Search screenshot | `docs/screenshots/search-results.png` | Posts |
| Now Playing screenshot | `docs/screenshots/now-playing.png` | Posts, video poster |
| Playlists screenshot | `docs/screenshots/playlists.png` | Posts |
| Demo video (50s) | `docs/videos/demo.mp4` | README, YouTube |
| Demo GIF (animated) | `docs/videos/demo.gif` | Reddit, HN, Mastodon |
| Social preview | `.github/social-preview.png` | Open Graph, link sharing |
| Design tokens | `assets/brand/design-tokens.json` | Contributors |

---

## Distribution Channels

### Ready now (v0.2.1)
| Platform | Format | Download |
|----------|--------|----------|
| Linux | AppImage, .deb, .rpm | GitHub Releases ✅ |
| macOS | DMG (Apple Silicon) | GitHub Releases ✅ |
| Windows | MSI, NSIS setup.exe, portable exe | GitHub Releases ✅ |

### Pending submission
| Channel | What's needed | Effort |
|---------|---------------|--------|
| **Homebrew cask** | Create `netcraker01/homebrew-helix` repo + push cask | 15 min |
| **winget** | Open PR to microsoft/winget-pkgs | 20 min |
| **Flathub** | Blocked (see notes) | — |

---

## Post Drafts

### 1. Reddit — r/linux

**Title:** Helix — a privacy-first native music player for Linux (AI-assisted, human-led engineering)

**Body:**

> ![Helix](https://raw.githubusercontent.com/netcraker01/helix/main/assets/brand/logo-wide.png)
>
> I was tired of music apps that are either bloated, browser-based, or full of tracking, so I built my own.
>
> It’s called **Helix**, a privacy-first desktop music player built with **Rust + Tauri + Svelte**.
>
> I also want to be explicit about something up front: **this is an AI-assisted project, but it is not vibe-coded slop**. I designed and directed it using a strict software engineering workflow, with continuous human-in-the-loop review, requirement-driven iteration, rigid validation harnesses, and deterministic checks. AI wrote a significant part of the implementation, but the architecture, constraints, acceptance criteria, and review process were continuously controlled by me.
>
> **What it does:**
>
> - Stream music from **YouTube** and **SoundCloud** without opening a browser
> - Play **local files** (FLAC, MP3, OPUS, etc.)
> - **7 real-time visualizers** — Bars, Wave, Mirror, Radial, Aurora, Grid, Tunnel
> - **Cinematic mode** — full-app ambient background that reacts to the music
> - **Queue management, playlists, artist favorites, listening history**
> - **Bilingual UI** (English + Spanish)
>
> **What it does NOT do:**
>
> - no account required
> - no tracking
> - no ads
> - not built with Electron
>
> **Downloads currently available:**
>
> - Linux: AppImage, .deb, .rpm
> - Windows: NSIS installer, MSI, portable exe
> - macOS: Apple Silicon
>
> **GitHub:**
> https://github.com/netcraker01/helix
>
> **Release:**
> https://github.com/netcraker01/helix/releases/tag/v0.2.1
>
> This is still alpha, but it’s already usable and I’d really like feedback from Linux users.
>
> **What I’m most interested in:**
>
> - playback stability
> - distro-specific issues
> - whether the UI feels comfortable for daily use
> - whether the visualizers are actually useful or just noise
>
> Thanks!

---

### 2. Reddit — r/opensource

**Title:** I built an open-source Spotify alternative in Rust + Tauri (alpha release)

**Body:**

> ![](https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/now-playing.png)
>
> **Helix** is an open-source (AGPL-3.0) desktop music player that lets you stream from YouTube and SoundCloud, play local files, and visualize your music in real time — without signing up for anything.
>
> **Why I built it:**
>
> Every "Spotify alternative" falls into one trap or another: Electron-based and bloated, browser-only with no real audio access, terminal-only, or requires its own account infrastructure. Helix is none of those.
>
> **Tech stack:**
>
> - **Language:** Rust (backend), Svelte (frontend)
> - **Desktop framework:** Tauri v2 (WebKitGTK / WebView2 / WKWebView)
> - **Audio:** yt-dlp → Symphonia decoding → cpal playback → rustfft visualization
> - **Streaming sources:** YouTube (via yt-dlp), SoundCloud (API), local files
>
> **Current features:**
>
> - YouTube + SoundCloud streaming
> - Local file playback (FLAC, MP3, OPUS, etc.)
> - 7 real-time visualizer modes
> - Cinematic ambient mode
> - Queue, playlists, favorites, history
> - Automatic yt-dlp management
> - Bilingual UI (EN/ES)
>
> ![](https://raw.githubusercontent.com/netcraker01/helix/main/docs/videos/demo.gif)
>
> **Join the project:**
>
> https://github.com/netcraker01/helix
>
> Looking for contributors, testers, and feedback. Everything is AGPL-3.0, and there are Good First Issues tagged for people who want to help.

---

### 3. Reddit — r/privacy

**Title:** Helix — a music player that doesn't track you (open source, no accounts)

**Body:**

> **The problem:** Every mainstream music service builds a profile of what you listen to, when you listen, what you skip, and what you repeat. That data is worth money. Your listening habits are productized.
>
> **Helix is different:**
>
> - **No accounts.** Not optional accounts — there is no account system at all.
> - **No telemetry.** The app phones home to exactly zero servers. Streams go directly from YouTube/SoundCloud to your speakers.
> - **No cookies, no fingerprinting.** It's a native app, not a web app wrapped in a browser.
> - **No recommendation algorithms that track your behavior.** The home screen shows your recently played, your favorites, and curated moods — all computed locally.
>
> **What you get instead:**
>
> - Stream from YouTube and SoundCloud without opening a browser
> - Play your local FLAC/MP3/OPUS collection
> - Real-time audio visualizers (7 modes)
> - Queue, playlists, favorites, listening history (all stored locally)
>
> **Tech:**
>
> Built with Rust + Tauri + Svelte. Completely open source (AGPL-3.0).
>
> https://github.com/netcraker01/helix
>
> **Screenshots:**
>
> | Home | Search | Now Playing | Library |
> |------|--------|-------------|---------|
> | ![](https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/home.png) | ![](https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/search-results.png) | ![](https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/now-playing.png) | ![](https://raw.githubusercontent.com/netcraker01/helix/main/docs/screenshots/playlists.png) |
>
> **Why trust it?**
>
> 1. It's compiled from source — you can inspect every line of code
> 2. No network connections except the streams you explicitly request
> 3. yt-dlp runs locally, no third-party API proxies
> 4. All data (playlists, favorites) stays on your machine

---

### 4. Hacker News — Show HN

**Title:** Show HN: Helix – native desktop music player in Rust/Tauri with real-time visualizers

**Body:**

> https://github.com/netcraker01/helix
>
> I've been working on a privacy-first desktop music player. It's in alpha but already usable for daily listening.
>
> **What makes it different from the dozens of other "Spotify alternatives":**
>
> 1. **Native audio pipeline.** Most "desktop" players are Electron apps feeding audio through a browser's MSE API. Helix runs the entire audio stack natively: yt-dlp resolves streams, Symphonia decodes, cpal outputs to your audio device, rustfft powers the visualizers. You get real PCM data, not browser approximations.
>
> 2. **7 real-time visualizers.** Because we have access to raw FFT data from the decoding pipeline, the visualizers aren't canned animations — they respond to the actual frequency content of every track. Modes: Bars, Wave, Mirror, Radial, Aurora, Grid, Tunnel.
>
> 3. **Multisource streaming.** YouTube, SoundCloud, local files — all in one interface. No browser tabs, no switching between apps.
>
> 4. **No accounts, no tracking, no telemetry.** Not "we respect your privacy" — the app literally has no way to track you. No sign-up screen, no analytics SDK, no backend API.
>
> **Tech stack:**
>
> - **Backend:** Rust (Symphonia, cpal, rustfft, reqwest)
> - **Frontend:** Svelte + TypeScript
> - **Desktop framework:** Tauri v2
> - **Stream resolution:** yt-dlp
> - **UI i18n:** English + Spanish
>
> ![](https://raw.githubusercontent.com/netcraker01/helix/main/docs/videos/demo.gif)
>
> **Downloads:** https://github.com/netcraker01/helix/releases (AppImage, .deb, .rpm, DMG, MSI, NSIS)
>
> **Would love feedback on:**
> - The visualizer modes (too much? not enough?)
> - Audio latency / quality
> - The UI workflow (search → play → organize)
> - Any bugs you hit
>
> All feedback welcome. Open source under AGPL-3.0.

---

### 5. Mastodon / Fediverse

**Post 1 (intro + GIF):**
> 🎵 I've been building Helix — a privacy-first, open-source music player.
>
> Stream from YouTube & SoundCloud, play local files, 7 real-time visualizers. No accounts, no tracking.
>
> Built with Rust + Tauri + Svelte.
>
> [demo.gif]
>
> https://github.com/netcraker01/helix

**Post 2 (screenshots):**
> Native desktop. Not Electron, not a web wrapper.
>
> Real FFT-powered visualizers. Bilingual UI (EN/ES). Queue, playlists, favorites.
>
> Linux • macOS • Windows
>
> [screenshots grid]
>
> https://github.com/netcraker01/helix

**Post 3 (call for contributors):**
> Looking for:
> • UI/UX feedback
> • Bug reports (it's alpha)
> • Testers on different distros
> • Contributors who want to add streaming sources
>
> Good First Issues tagged in the repo.
>
> https://github.com/netcraker01/helix

---

## Timing & Scheduling

| Day | Action | Channel |
|-----|--------|---------|
| **Day 1 — 14:00 UTC** | Post Show HN | Hacker News |
| **Day 1 — 15:00 UTC** | Post with GIF + screenshots | r/linux |
| **Day 1 — 15:30 UTC** | Post | r/opensource |
| **Day 1 — 16:00 UTC** | Post | r/privacy |
| **Day 1 — 16:30 UTC** | Thread (3 posts) | Mastodon |
| **Day 2** | Monitor comments, respond | All |
| **Day 3** | Post to Lobste.rs, Tildes | Secondary |
| **Day 7** | Post update if significant traction | All |

**Tips:**
- Post Reddit/HN in **US morning time** (14:00–16:00 UTC = 10am–12pm ET) for max visibility
- Don't post all Reddit subs at the exact same second — space by 30-60 min
- Reply to EVERY comment in the first 6 hours (algorithm boosts engagement)
- For HN: the title is critical. Keep it factual and specific

---

## Quick Commands for Submission

### Homebrew tap
```bash
# Create the tap repo on GitHub first (one-time):
gh repo create homebrew-helix --public --description "Homebrew tap for Helix music player"

# Clone and push the cask:
git clone https://github.com/netcraker01/homebrew-helix.git
cd homebrew-helix
mkdir -p Casks
cp /path/to/packaging/homebrew/Casks/helix-player.rb Casks/
git add Casks/helix-player.rb
git commit -m "Add helix-player cask v0.2.1"
git push origin main

# Users install via:
# brew tap netcraker01/helix
# brew install --cask helix-player
```

### winget PR
```bash
# Fork microsoft/winget-pkgs, then:
# Create manifests/n/netcraker01/helix-player/0.2.1/ directory
# Update: InstallerUrl, InstallerSha256, ProductCode, UpgradeCode
# Open PR to microsoft/winget-pkgs
```

---

## Checklist: Before You Post

- [ ] Verify ALL download links work on the release page
- [ ] Test that screenshots render from raw.githubusercontent.com URLs
- [ ] Verify the demo GIF plays in a browser
- [ ] Check README renders correctly on GitHub
- [ ] Confirm GitHub Discussions is enabled (for community replies)
- [ ] Have browser tabs ready for each post to respond to comments
- [ ] Notify any existing users/stargazers (if any) before the public posts
