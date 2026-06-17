# Tasks: Frontend Restructure

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 550–700 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (config + dirs + i18n) → PR 2 (layout + stubs + tests) |
| Delivery strategy | auto-forecast |
| Chain strategy | feature-branch-chain |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Config, directory tree, i18n expansion | PR 1 | Base: feature/frontend-restructure; ~200-250 lines |
| 2 | Layout shell, route pages, services, types, styles, tests, cleanup | PR 2 | Base: PR 1 branch; ~300-450 lines |

## Phase 1: Config + Dependencies + Directory Structure

- [x] 1.1 Create `ui/tsconfig.json` with Svelte TS config and path aliases (`@/*`, `@features/*`, `@shared/*`, `@services/*`)
- [x] 1.2 Create `ui/svelte.config.js` with `svelte-preprocess` and TypeScript support
- [x] 1.3 Create `ui/vite.config.ts` replacing `vite.config.js` — Svelte plugin, path aliases, Vitest config inline with jsdom environment
- [x] 1.4 Update `ui/package.json` — add `svelte-routing`, `vitest`, `@testing-library/svelte`, `jsdom`, `lucide-svelte`, `svelte-preprocess` (dev vs prod separation); add `test` script
- [x] 1.5 Run `npm install` in `ui/` and verify no peer dependency errors
- [x] 1.6 Create full directory tree under `ui/src/` with `.gitkeep` files: `app/layout/`, `routes/Home/`, `routes/Search/`, `routes/Favorites/`, `routes/NowPlaying/`, `features/player/components/`, `features/player/stores/`, `features/player/types/`, `features/search/components/` (+stores+types), `features/favorites/components/` (+stores+types), `features/library/components/` (+stores+types), `shared/components/`, `shared/stores/`, `shared/types/`, `shared/utils/`, `shared/constants/`, `shared/icons/`, `services/`, `styles/`
- [x] 1.7 Delete legacy directories: `ui/src/components/`, `ui/src/stores/`, `ui/src/themes/`
- [x] 1.8 Expand `ui/src/i18n/locales/en.json` with `routes.home`, `routes.search`, `routes.favorites`, `routes.now_playing`, `routes.settings`
- [x] 1.9 Expand `ui/src/i18n/locales/es.json` with matching Spanish route keys
- [x] 1.10 Update `ui/index.html` — change script `src` from `/src/App.svelte` to `/src/main.ts`

## Phase 2: Entry Point + Layout Shell

- [x] 2.1 Create `ui/src/main.ts` — init i18n, import styles, mount App with svelte-routing Router
- [x] 2.2 Create `ui/src/app/App.svelte` — CSS Grid layout shell: Sidebar (left) + Router outlet (center) + BottomBar (bottom)
- [x] 2.3 Create `ui/src/app/layout/Sidebar.svelte` — nav links using svelte-routing `Link`, lucide-svelte icons, active route highlight using `$location` store
- [x] 2.4 Create `ui/src/app/layout/BottomBar.svelte` — stub layout: track info left, play/pause/skip center, volume right

## Phase 3: Route Pages + Feature Stubs

- [x] 3.1 Create `ui/src/routes/Home/Page.svelte` — stub page with `<h1>Home</h1>` heading
- [x] 3.2 Create `ui/src/routes/Search/Page.svelte` — stub page with `<h1>Search</h1>`
- [x] 3.3 Create `ui/src/routes/Favorites/Page.svelte` — stub page with `<h1>Favorites</h1>`
- [x] 3.4 Create `ui/src/routes/NowPlaying/Page.svelte` — stub page with `<h1>Now Playing</h1>`
- [x] 3.5 Create 5 empty feature stubs in `ui/src/features/player/components/`: `Controls.svelte`, `ProgressBar.svelte`, `Queue.svelte`, `NowPlayingInfo.svelte`, `Visualizer.svelte`
- [x] 3.6 Create `ui/src/features/player/stores/player.ts` — empty store stub
- [x] 3.7 Create `ui/src/features/player/types/index.ts` — empty types index
- [x] 3.8 Create 4 empty shared component stubs in `ui/src/shared/components/`: `TrackList.svelte`, `AlbumCard.svelte`, `ArtistCard.svelte`, `Sidebar.svelte`

## Phase 4: Services + Types + Styles

- [x] 4.1 Create `ui/src/shared/types/models.ts` — TypeScript interfaces `Track`, `Artist`, `Album`, `Source` enum mirroring ARCHITECTURE.md §4
- [x] 4.2 Create `ui/src/shared/stores/theme.ts` — dark-mode token system, sets CSS custom properties on `:root`
- [x] 4.3 Create `ui/src/services/tauri.ts` — `invokeCommand<T>` and `subscribeEvent<T>` with Tauri fallback for browser mode
- [x] 4.4 Create `ui/src/services/commands.ts` — typed command stubs: `search`, `play`, `pause`, `next`, `previous`, `setVolume`, `toggleFavorite`
- [x] 4.5 Create `ui/src/services/events.ts` — typed event stubs: `onTrackChanged`, `onStateChanged`, `onQueueUpdated`, `onProgressTick`
- [x] 4.6 Create `ui/src/styles/tokens.css` — CSS custom properties: `--bg-base`, `--bg-surface`, `--bg-elevated`, `--text-primary`, `--text-secondary`, `--color-accent` (#6366f1)
- [x] 4.7 Create `ui/src/styles/global.css` — imports `tokens.css`, applies baseline dark-mode styles (background, text, font)
- [x] 4.8 Create `ui/src/styles/animations.css` — empty animation stub with comment placeholder

## Phase 5: Tauri Config + Testing + Cleanup

- [x] 5.1 Update `src-tauri/tauri.conf.json` — add SPA handler config for svelte-routing path resolution
- [x] 5.2 Create `ui/src/tests/App.test.ts` — Vitest smoke test: App renders Sidebar + BottomBar + outlet (spec FR-011 scenario)
- [x] 5.3 Create `ui/src/tests/aliases.test.ts` — test path alias `@shared/types/models` resolves correctly (spec FR-001)
- [x] 5.4 Create `ui/src/tests/i18n.test.ts` — test new route keys resolve in both locales (spec FR-007)
- [x] 5.5 Delete `ui/src/App.svelte` (replaced by `ui/src/app/App.svelte`)
- [x] 5.6 Delete `ui/vite.config.js` (replaced by `ui/vite.config.ts`)
- [x] 5.7 Run `npm run dev` — verify app starts with Sidebar + BottomBar layout visible (success criterion)
- [x] 5.8 Run `npm run build` — verify bundle produces without errors (success criterion)
- [x] 5.9 Run `npm run test` — verify Vitest runs and all smoke tests pass (success criterion)