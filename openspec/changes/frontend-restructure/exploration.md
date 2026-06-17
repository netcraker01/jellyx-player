## Exploration: Frontend Restructure

### Current State

The frontend (`ui/`) is a **throwaway prototype** — a single `App.svelte` file containing minimal inline search+play logic with hardcoded dark-mode CSS. There is no routing, no component decomposition, no services layer, no TypeScript types, no test infrastructure, and no build configuration for Svelte's modern SFC patterns. The only real production-quality code is the `i18n/` module, which implements a solid reactive translation system with locale detection, caching, and dynamic imports.

**What exists:**
| File | Status | Quality |
|------|--------|---------|
| `ui/src/App.svelte` | Throwaway prototype | Monolithic: search bar + results list + play invoke. No layout, no sidebar, no bottom bar, no player controls |
| `ui/src/i18n/index.ts` | Real, reusable | Well-structured: writable/derived stores, locale detection, localStorage persistence, dynamic imports with cache |
| `ui/src/i18n/locales/en.json` | Real, reusable | Good coverage of future UI strings (player, library, visualizer, settings, errors) |
| `ui/src/i18n/locales/es.json` | Real, reusable | Complete Spanish translations matching en.json |
| `ui/src/components/.gitkeep` | Empty placeholder | — |
| `ui/src/stores/.gitkeep` | Empty placeholder | — |
| `ui/src/themes/.gitkeep` | Empty placeholder | — |
| `ui/package.json` | Minimal | Only svelte 4, @tauri-apps/api 2, @sveltejs/vite-plugin-svelte 3, vite 5, typescript 5 |
| `ui/vite.config.js` | Minimal | Bare Svelte plugin, no path aliases, no TS, no router |
| `ui/index.html` | Minimal | Loads App.svelte directly as module entry, imports Inter font |

**What's missing from ARCHITECTURE.md §5.2:**
| Target Dir/File | Status |
|----------------|--------|
| `src/app/App.svelte` | Needs move from `src/App.svelte` |
| `src/app/layout/` | Missing — needs Sidebar, BottomBar layout components |
| `src/routes/Home/+page.svelte` | Missing |
| `src/routes/Search/+page.svelte` | Missing |
| `src/routes/Favorites/+page.svelte` | Missing |
| `src/routes/NowPlaying/+page.svelte` | Missing |
| `src/features/player/components/Controls.svelte` | Missing |
| `src/features/player/components/ProgressBar.svelte` | Missing |
| `src/features/player/components/Queue.svelte` | Missing |
| `src/features/player/components/NowPlayingInfo.svelte` | Missing |
| `src/features/player/components/Visualizer.svelte` | Missing |
| `src/features/player/stores/player.ts` | Missing |
| `src/features/player/types/` | Missing |
| `src/features/search/components/` | Missing |
| `src/features/search/stores/` | Missing |
| `src/features/search/types/` | Missing |
| `src/features/favorites/components/` | Missing |
| `src/features/favorites/stores/` | Missing |
| `src/features/favorites/types/` | Missing |
| `src/features/library/components/` | Missing |
| `src/features/library/stores/` | Missing |
| `src/features/library/types/` | Missing |
| `src/shared/components/TrackList.svelte` | Missing |
| `src/shared/components/AlbumCard.svelte` | Missing |
| `src/shared/components/ArtistCard.svelte` | Missing |
| `src/shared/components/Sidebar.svelte` | Missing |
| `src/shared/stores/theme.ts` | Missing |
| `src/shared/types/models.ts` | Missing — TypeScript mirrors of Rust models |
| `src/shared/utils/` | Missing |
| `src/shared/constants/` | Missing |
| `src/shared/icons/` | Missing |
| `src/services/tauri.ts` | Missing — Tauri bridge |
| `src/services/events.ts` | Missing — Rust event subscriptions |
| `src/services/commands.ts` | Missing — Typed invoke commands |
| `src/styles/global.css` | Missing |
| `src/styles/tokens.css` | Missing |
| `src/styles/animations.css` | Missing |
| `src/main.ts` | Missing — currently App.svelte is loaded directly from index.html |
| `svelte.config.js` | Missing |
| `tsconfig.json` | Missing |
| `vite.config.ts` | Needs upgrade from .js, add path aliases, router |

### Dependency Gap Analysis

**Current `package.json` dependencies:**
| Package | Version | Needed? |
|---------|---------|---------|
| svelte | ^4.0.0 | Yes (base) |
| @tauri-apps/api | ^2.0.0 | Yes (IPC) |
| @sveltejs/vite-plugin-svelte | ^3.0.0 | Yes (build) |
| vite | ^5.0.0 | Yes (build) |
| typescript | ^5.0.0 | Yes (types) |

**Missing dependencies for target architecture:**
| Package | Purpose | Priority |
|---------|---------|----------|
| @tauri-apps/api | Already present, but need event API | — |
| svelte-routing or svelte-spa-router | Client-side routing for /Home, /Search, /Favorites, /NowPlaying | **Critical** |
| vitest | Test runner | High |
| @testing-library/svelte | Component testing | High |
| jsdom | DOM env for vitest | High |
| @sveltejs/adapter-static | If building as SPA for Tauri | Medium |
| lucide-svelte | Icon library (per UI_DESIGN.md) | Medium |
| svelte-preprocess | For TypeScript in Svelte components | Medium |
| postcss + postcss-preset-env | CSS tooling if needed | Low |

**Missing configuration:**
| File | What's needed |
|------|---------------|
| `tsconfig.json` | TS config extending Svelte's defaults, path aliases for `@/` |
| `svelte.config.js` | Svelte preprocess config (TypeScript, PostCSS) |
| `vite.config.ts` | Upgrade from .js, add resolve aliases (`@app`, `@features`, `@shared`, `@services`), router plugin |
| `.svelte-kit/` or equivalent | Build output config for Tauri |

### Affected Areas

- `ui/src/App.svelte` — Must be decomposed; its search/play logic moves to features/player and features/search
- `ui/src/i18n/` — Keep as-is; relocate under `src/i18n/` path (module is production quality)
- `ui/src/i18n/locales/en.json` — Needs expansion for route labels, feature-specific strings
- `ui/src/i18n/locales/es.json` — Same expansion needed
- `ui/package.json` — Add router, vitest, testing library, icons, preprocess
- `ui/vite.config.js` → `ui/vite.config.ts` — Path aliases, router plugin, test config
- `ui/index.html` — Update entry point from App.svelte to main.ts
- NEW `ui/tsconfig.json` — TypeScript project references for Svelte
- NEW `ui/svelte.config.js` — Svelte preprocess config

### Approaches

1. **Clean Rewrite with Scaffolding** — Delete App.svelte, scaffold the full directory structure from ARCHITECTURE.md §5.2, set up all configs (tsconfig, svelte.config, vite.config), install all deps, create stub components with correct imports
   - Pros: Clean start, no legacy coupling, matches architecture exactly, sets up testing infra from day 1
   - Cons: Larger diff, throws away the working search+play prototype (but it's minimal and can be recreated)
   - Effort: Medium

2. **Incremental Migrate** — Keep App.svelte, gradually extract pieces into new structure while keeping the app functional at each step
   - Pros: Working app at every step, smaller diffs per change
   - Cons: App.svelte is so minimal (157 lines, no real component decomposition) that incremental migration adds unnecessary complexity; the "working" state is just a search bar that calls Tauri — not a real layout
   - Effort: Medium-High (more steps for less value)

3. **Minimal Scaffold + Feature Stubs** — Set up configs and directory structure, migrate i18n as-is, create App.svelte with layout shell (Sidebar + BottomBar + router outlet), stub all feature components, but don't implement any feature logic
   - Pros: Establishes architecture without overbuilding; each feature can be implemented in subsequent changes; test infra is ready from the start
   - Cons: Still a big initial change; requires follow-up changes for each feature
   - Effort: Medium

### Recommendation

**Approach 3: Minimal Scaffold + Feature Stubs** — This is the right balance. Here's why:

1. **App.svelte is throwaway**: The current 157-line file has no real component decomposition, no routing, no layout. It's a proof-of-concept, not production code. No point in migrating incrementally.

2. **i18n is keepable**: The i18n module is well-structured and can be relocated as-is. Only the locale JSON files need expansion (new route/feature strings).

3. **The real value is in the scaffolding**: Setting up tsconfig, svelte.config, vite.config with path aliases, vitest, @testing-library/svelte, and the full directory structure is the foundation everything else depends on. This must be done first and done correctly.

4. **Feature stubs over full implementation**: Each feature (player, search, favorites, library) should get its directory structure, a stub store, and stub components — but NOT full logic. That logic belongs in separate SDD changes per feature.

**Change scope for `frontend-restructure`:**
- Set up all build/dev configuration (tsconfig, svelte.config, vite.config, vitest config)
- Install all missing dependencies (router, vitest, testing-library, icons, preprocess)
- Create full directory structure per ARCHITECTURE.md §5.2
- Relocate i18n module (keep implementation, expand locale strings)
- Create `main.ts` entry point with router setup
- Create App.svelte with layout shell (Sidebar + BottomBar + router outlet)
- Create stub components for every component listed in §5.2 (empty or minimal)
- Create `services/tauri.ts`, `services/events.ts`, `services/commands.ts` with typed stubs
- Create `shared/types/models.ts` with TypeScript mirrors of Rust models
- Create `shared/stores/theme.ts` with dark-mode token system
- Create `styles/` with global.css, tokens.css, animations.css stubs
- Set up vitest + @testing-library/svelte with example test

**What this change does NOT cover** (deferred to subsequent changes):
- Implementing any feature logic (player, search, favorites, library)
- Implementing the Visualizer component (needs Rust FFT bridge)
- Implementing real Tauri command handlers (backend feature)
- Full i18n string expansion for all features

### Risks

1. **Svelte 4 vs Svelte 5**: The current package.json uses Svelte 4. ARCHITECTURE.md doesn't specify a version. Svelte 5 introduces runes ($state, $derived) which change the store pattern. Must decide now — recommend Svelte 4 for MVP stability, with a migration plan documented.

2. **Router choice**: ARCHITECTURE.md §5.2 uses `+page.svelte` convention (SvelteKit-style). In a Tauri app (SPA, not SSR), we need either: (a) a client-side router that supports this convention, or (b) adapt the structure to use a different routing pattern. SvelteKit is overkill for Tauri — recommend `svelte-spa-router` or `svelte-routing` with adapted directory structure.

3. **+page.svelte naming convention**: ARCHITECTURE.md uses SvelteKit-style `+page.svelte` inside route folders. In a plain Svelte + Vite setup without SvelteKit, this convention won't work automatically. We should use the same directory structure but with `Page.svelte` or `index.svelte` naming, OR set up a custom Vite plugin to resolve routes from the filesystem.

4. **Path alias configuration**: Both TypeScript and Vite need consistent path aliases. Must ensure tsconfig paths and vite resolve.alias are aligned.

5. **Tauri v2 integration**: The current vite.config.js doesn't configure Tauri-specific settings (like CSP, allowed commands). The restructure must ensure Tauri's dev server integration still works.

6. **Test infrastructure timing**: vitest + @testing-library/svelte setup is simple but must be done correctly. Better to include it in this change so that subsequent feature implementations can write tests immediately.

### Ready for Proposal

**Yes** — The exploration is complete. The gap is clear, the recommendation is defined, and the change scope is well-bounded. The orchestrator should tell the user:

> The frontend is a throwaway prototype with one real module (i18n). We recommend a **minimal scaffold + feature stubs** approach: set up all build configuration, create the full directory structure from ARCHITECTURE.md §5.2, relocate i18n, create layout shell with router, and stub all feature components — but don't implement any feature logic yet. Each feature will get its own SDD change. We need a decision on: (1) Svelte 4 vs 5, (2) routing approach for the +page.svelte convention in a non-SvelteKit SPA.