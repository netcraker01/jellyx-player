# Design: Frontend Restructure

## Technical Approach

Greenfield scaffold per ARCHITECTURE.md §5.2: configure build pipeline first (tsconfig → svelte.config → vite.config → vitest), then create the full directory tree, relocate i18n, wire main.ts as entry point, rewrite App.svelte as a layout shell (Sidebar + BottomBar + Router outlet), and stub all features/services/types. The approach follows the proposal's "Minimal Scaffold + Feature Stubs" strategy, establishing the exact structure future features will build on.

## Architecture Decisions

| Decision | Choice | Alternatives Rejected | Rationale |
|----------|--------|----------------------|-----------|
| Router library | svelte-routing | routify, svelte-navigator, tanstack | Proposal confirmed; lightweight, Svelte 4 compatible, path-based, no SvelteKit dependency |
| Path alias source of truth | vite.config.ts defines, tsconfig extends | tsconfig as source, separate config | Single definition avoids drift; tsconfig references vite aliases |
| CSS layout strategy | CSS Grid for app shell | Flexbox-only, absolute positioning | Grid gives explicit row/column control for Sidebar + content + BottomBar; matches §5.2 layout |
| Theme token system | CSS custom properties | Svelte stores, CSS-in-JS | Tokens in CSS cascade naturally; no runtime cost; matches UI_DESIGN.md dark-mode-only spec |
| SPA routing in Tauri | Rewrite all paths to index.html | Hash-based routing | Path-based URLs are cleaner; Tauri v2 supports SPA handler natively via `withGlobalTauri` config |
| i18n relocation | Move as-is, expand JSON | Rewrite i18n module | Existing i18n is production-quality; expanding JSONs is lower risk than rewriting |
| Vitest config location | Inline in vite.config.ts | Separate vitest.config.ts | One file for build+test config; aliases stay in sync automatically |

## Data Flow

```
index.html ──→ main.ts ──→ App.svelte (mount)
                  │              │
                  │              ├─ Sidebar ──→ Link ──→ svelte-routing
                  │              ├─ Router Outlet ──→ Route pages (stubs)
                  │              └─ BottomBar (player stub)
                  │
                  ├─ initI18n() ──→ i18n/index.ts ──→ locales/*.json
                  │
                  └─ (future) services/ ──→ tauri.ts ──→ invoke ──→ Rust
                                              events.ts ──→ listen  ──→ Rust
                                              commands.ts ──→ typed wrappers
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `ui/tsconfig.json` | Create | Svelte TS config with path aliases @/*, @features/*, @shared/*, @services/* |
| `ui/svelte.config.js` | Create | svelte-preprocess with TypeScript |
| `ui/vite.config.ts` | Create (replaces .js) | Svelte plugin, path aliases, Vitest config with jsdom; delete vite.config.js |
| `ui/package.json` | Modify | Add svelte-routing, vitest, @testing-library/svelte, jsdom, lucide-svelte, svelte-preprocess |
| `ui/index.html` | Modify | Change script src from `/src/App.svelte` to `/src/main.ts` |
| `ui/src/main.ts` | Create | Entry: init i18n, mount App with router |
| `ui/src/app/App.svelte` | Create | Layout shell: CSS Grid → Sidebar + Router outlet + BottomBar |
| `ui/src/app/layout/Sidebar.svelte` | Create | Nav links with lucide-svelte icons, active route highlight |
| `ui/src/app/layout/BottomBar.svelte` | Create | Stub: track info, play/pause/skip, volume placeholders |
| `ui/src/routes/Home/+page.svelte` | Create | Stub page with heading |
| `ui/src/routes/Search/+page.svelte` | Create | Stub page with heading |
| `ui/src/routes/Favorites/+page.svelte` | Create | Stub page with heading |
| `ui/src/routes/NowPlaying/+page.svelte` | Create | Stub page with heading |
| `ui/src/features/player/components/*.svelte` | Create | 5 empty stubs (Controls, ProgressBar, Queue, NowPlayingInfo, Visualizer) |
| `ui/src/features/player/stores/player.ts` | Create | Empty store stub |
| `ui/src/features/player/types/index.ts` | Create | Empty types index |
| `ui/src/features/search/components/` | Create | .gitkeep |
| `ui/src/features/search/stores/` | Create | .gitkeep |
| `ui/src/features/search/types/` | Create | .gitkeep |
| `ui/src/features/favorites/components/` | Create | .gitkeep |
| `ui/src/features/favorites/stores/` | Create | .gitkeep |
| `ui/src/features/favorites/types/` | Create | .gitkeep |
| `ui/src/features/library/components/` | Create | .gitkeep |
| `ui/src/features/library/stores/` | Create | .gitkeep |
| `ui/src/features/library/types/` | Create | .gitkeep |
| `ui/src/shared/components/TrackList.svelte` | Create | Empty stub |
| `ui/src/shared/components/AlbumCard.svelte` | Create | Empty stub |
| `ui/src/shared/components/ArtistCard.svelte` | Create | Empty stub |
| `ui/src/shared/components/Sidebar.svelte` | Create | Empty stub (re-export from app/layout later) |
| `ui/src/shared/stores/theme.ts` | Create | Dark-mode token system using CSS custom properties |
| `ui/src/shared/types/models.ts` | Create | Track, Artist, Album, Source mirroring Rust models |
| `ui/src/shared/utils/` | Create | .gitkeep |
| `ui/src/shared/constants/` | Create | .gitkeep |
| `ui/src/shared/icons/` | Create | .gitkeep |
| `ui/src/services/tauri.ts` | Create | invokeCommand\<T\> and subscribeEvent\<T\> with Tauri fallback |
| `ui/src/services/events.ts` | Create | onTrackChanged, onStateChanged, onQueueUpdated, onProgressTick stubs |
| `ui/src/services/commands.ts` | Create | search, play, pause, next, previous, setVolume, toggleFavorite stubs |
| `ui/src/styles/tokens.css` | Create | --bg-base, --bg-surface, --bg-elevated, --text-primary, --text-secondary, --color-accent |
| `ui/src/styles/global.css` | Create | Imports tokens.css, applies baseline dark-mode styles |
| `ui/src/styles/animations.css` | Create | Empty animation stubs |
| `ui/src/i18n/locales/en.json` | Modify | Add routes.home, routes.search, routes.favorites, routes.now_playing, routes.settings |
| `ui/src/i18n/locales/es.json` | Modify | Add route keys in Spanish |
| `ui/src/App.svelte` | Delete | Replaced by ui/src/app/App.svelte |
| `ui/src/components/` | Delete | Replaced by new structure |
| `ui/src/stores/` | Delete | Replaced by new structure |
| `ui/src/themes/` | Delete | Replaced by new structure |
| `ui/vite.config.js` | Delete | Replaced by vite.config.ts |
| `src-tauri/tauri.conf.json` | Modify | Add SPA handler for client-side routing |

## Interfaces / Contracts

```typescript
// services/tauri.ts
export async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
export async function subscribeEvent<T>(event: string, cb: (payload: T) => void): Promise<() => void>;

// services/commands.ts
export function search(query: string): Promise<Track[]>;
export function play(trackId: string): Promise<void>;
export function pause(): Promise<void>;
export function next(): Promise<void>;
export function previous(): Promise<void>;
export function setVolume(volume: number): Promise<void>;
export function toggleFavorite(trackId: string): Promise<void>;

// services/events.ts
export function onTrackChanged(cb: (track: Track) => void): UnlistenFn;
export function onStateChanged(cb: (state: string) => void): UnlistenFn;
export function onQueueUpdated(cb: (queue: Track[]) => void): UnlistenFn;
export function onProgressTick(cb: (progress: number) => void): UnlistenFn;

// shared/types/models.ts
export enum Source { YouTube = 'YouTube', SoundCloud = 'SoundCloud', Local = 'Local' }
export interface Track { id: string; source: Source; sourceId: string; title: string; artist: string; album?: string; duration?: number; thumbnail?: string; streamUrl?: string; localPath?: string; metadata: Record<string, string> }
export interface Artist { id: string; name: string; thumbnail?: string; source: Source; sourceId: string }
export interface Album { id: string; title: string; artist: string; cover?: string; year?: number; source: Source; sourceId: string; tracks: string[] }
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | App.svelte renders Sidebar + BottomBar + outlet | Vitest + @testing-library/svelte smoke test |
| Unit | Path aliases resolve | Import from @shared/types/models in test |
| Unit | i18n key expansion | Test new route keys resolve in both locales |
| Unit | invokeCommand fallback | Test returns safe value when Tauri unavailable |
| Integration | Router navigation | Test each route renders correct page stub |
| E2E | None yet | Deferred to feature implementation phases |

## Migration / Rollout

**Phase 1 — Config + Structure + i18n** (chained PR #1):
1. Create tsconfig.json, svelte.config.js, vite.config.ts
2. Update package.json with new deps, run npm install
3. Create full directory tree with .gitkeep files
4. Relocate i18n/ (stays at same path, just expand locale JSONs)
5. Delete legacy dirs (components/, stores/, themes/)
6. Update index.html entry point

**Phase 2 — Layout + Stubs + Tests** (chained PR #2):
1. Create main.ts with router setup
2. Create App.svelte layout shell
3. Create Sidebar.svelte and BottomBar.svelte
4. Create route page stubs
5. Create shared/types/models.ts and services stubs
6. Create styles/tokens.css and global.css
7. Create theme.ts store
8. Add Vitest config and smoke test
9. Update tauri.conf.json SPA handler
10. Delete old App.svelte and vite.config.js

## Open Questions

- [ ] Exact accent color value for `--color-accent` (UI_DESIGN.md mentions "verde cyan, morado neón o azul eléctrico" — needs decision)
- [ ] Whether `+page.svelte` naming convention should match SvelteKit convention or use simpler `Page.svelte` (current spec uses `+page.svelte`)