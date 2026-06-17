# Proposal: Frontend Restructure

## Intent

The current frontend is a throwaway 157-line prototype (`App.svelte`) with no routing, layout, component decomposition, services layer, TypeScript types, or test infrastructure. The only production-quality code is the `i18n/` module. This change restructures the frontend to match **ARCHITECTURE.md §5.2**, establishing the hybrid `routes/` + `features/` + `shared/` architecture so that subsequent feature implementations have a correct foundation.

## Scope

### In Scope
- Build config: `tsconfig.json`, `svelte.config.js`, `vite.config.ts` with path aliases (`@/`, `@features/`, `@shared/`, `@services/`)
- Dependency additions: `svelte-routing`, `vitest`, `@testing-library/svelte`, `jsdom`, `lucide-svelte`, `svelte-preprocess`
- Full directory structure per ARCHITECTURE.md §5.2 (`app/`, `routes/`, `features/`, `shared/`, `services/`, `styles/`)
- `main.ts` entry point with router setup (`svelte-routing`, path-based routes)
- `App.svelte` rewrite as layout shell (Sidebar + BottomBar + router outlet)
- Stub route pages: `Home/+page.svelte`, `Search/+page.svelte`, `Favorites/+page.svelte`, `NowPlaying/+page.svelte`
- Stub feature components (empty SFCs): all components listed in §5.2 `features/player/`, `features/search/`, `features/favorites/`, `features/library/`
- Stub feature stores and types directories
- `services/tauri.ts`, `services/events.ts`, `services/commands.ts` typed stubs
- `shared/types/models.ts` — TypeScript mirrors of Rust models (Track, Artist, Album, Source)
- `shared/stores/theme.ts` — dark-mode token system
- `shared/components/` stubs (TrackList, AlbumCard, ArtistCard, Sidebar)
- `styles/global.css`, `styles/tokens.css`, `styles/animations.css` stubs
- Relocate `i18n/` module as-is, expand locale JSON with route/feature strings
- Update `index.html` to load `main.ts` instead of `App.svelte` directly
- Vitest + @testing-library/svelte setup with one example test
- Tauri SPA handler config for `svelte-routing`

### Out of Scope
- Feature logic implementation (player controls, search, favorites, library)
- Visualizer implementation (needs Rust FFT bridge)
- Real Tauri command handlers (backend concern)
- Full i18n expansion for all features
- Svelte 5 migration (locked to Svelte 4 for MVP)

## Capabilities

### New Capabilities
- `frontend-scaffold`: Directory structure, build configs, path aliases, dependency installation, and dev/prod build pipeline
- `frontend-routing`: Path-based SPA routing via svelte-routing with Tauri SPA handler, route pages as stubs
- `frontend-layout`: App shell with Sidebar + BottomBar + router outlet, dark-mode theme tokens
- `frontend-ipc-bridge`: Typed Tauri service stubs (invoke commands, event subscriptions, typed models)

### Modified Capabilities
- None (no existing capabilities to modify — this is a greenfield scaffold)

## Approach

**Approach 3: Minimal Scaffold + Feature Stubs** (confirmed by user).

1. Set up all build/dev configuration first (tsconfig, svelte.config, vite.config, vitest config).
2. Install missing dependencies (svelte-routing, vitest, testing-library, lucide-svelte, svelte-preprocess).
3. Create full directory tree per ARCHITECTURE.md §5.2 with `.gitkeep` for empty dirs.
4. Relocate `i18n/` module as-is; expand locale JSONs with route labels and feature strings.
5. Create `main.ts` entry point wiring router, i18n init, and App mount.
6. Rewrite `App.svelte` as layout shell — Sidebar (navigation), BottomBar (player controls area), router outlet (center).
7. Stub all feature components as empty SFCs with correct import paths.
8. Create typed `services/` stubs and `shared/types/models.ts` mirroring Rust models.
9. Create style stubs (tokens.css for dark theme, global.css, animations.css).
10. Add Tauri SPA handler configuration for svelte-routing path resolution.
11. Add vitest config + one smoke test verifying App renders.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `ui/src/App.svelte` | Modified → Rewritten | Throwaway prototype → layout shell with Sidebar, BottomBar, router outlet |
| `ui/src/main.ts` | New | Entry point: router setup, i18n init, mount App |
| `ui/src/app/` | New | Layout components (App.svelte, layout/) |
| `ui/src/routes/` | New | 4 route pages as stubs (Home, Search, Favorites, NowPlaying) |
| `ui/src/features/` | New | 4 feature directories with stub components/stores/types |
| `ui/src/shared/` | New | Shared components, stores (theme), types (models), utils, constants, icons |
| `ui/src/services/` | New | 3 typed service stubs (tauri, events, commands) |
| `ui/src/styles/` | New | 3 style files (global.css, tokens.css, animations.css) |
| `ui/src/i18n/` | Modified | Relocated (kept as-is), locale JSONs expanded |
| `ui/package.json` | Modified | Add svelte-routing, vitest, testing-library, lucide-svelte, svelte-preprocess |
| `ui/vite.config.js` | Modified → `vite.config.ts` | Path aliases, router, test config |
| `ui/tsconfig.json` | New | TypeScript project config with path aliases |
| `ui/svelte.config.js` | New | Svelte preprocess config |
| `ui/index.html` | Modified | Entry point changes from App.svelte to main.ts |
| `ui/vitest.config.ts` | New | Vitest configuration with jsdom and svelte testing |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| svelte-routing + Tauri CSP conflicts | Low | Configure Tauri SPA handler and CSP to allow navigation; test early |
| Svelte 4 vs 5 migration path | Low (decided) | Locked to Svelte 4 for MVP; document runes migration plan for future |
| Path alias sync between tsconfig and vite | Medium | Single source of truth: define aliases in vite.config.ts, extend in tsconfig |
| Large diff size (400+ lines likely) | High | Split into 2 chained PRs: (1) configs + directory structure + i18n, (2) layout shell + stubs + tests |

## Rollback Plan

All changes are additive (new files, config updates). Rollback = revert the commit(s). No data migration, no backend changes. Keep `App.svelte` backup as `App.prototype.svelte.bak` during transition if needed.

## Dependencies

- Change #1 (scaffold-restructure) — DONE and verified. Backend Rust scaffold is in place.
- `svelte-routing` npm package — must be added to package.json
- Tauri v2 SPA handler configuration — must be tested in dev mode

## Success Criteria

- [ ] `npm run dev` starts the app with Sidebar + BottomBar layout visible
- [ ] Navigation between /Home, /Search, /Favorites, /NowPlaying works via router
- [ ] i18n still functions (language switch, existing translations)
- [ ] `npm run build` produces a working bundle
- [ ] `npm run test` runs vitest with at least one passing smoke test
- [ ] Directory structure matches ARCHITECTURE.md §5.2 exactly
- [ ] All path aliases (`@/`, `@features/`, `@shared/`, `@services/`) resolve correctly