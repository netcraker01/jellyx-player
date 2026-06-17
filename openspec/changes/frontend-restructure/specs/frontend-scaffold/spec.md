# Frontend Scaffold Specification

## Purpose

Build pipeline, TypeScript configuration, dependency installation, and directory structure that establish the foundation for the Helix Player frontend. Without this, no other capability can compile or run.

## Requirements

### Requirement: FR-001 — TypeScript Configuration with Path Aliases

The project MUST provide a `tsconfig.json` that enables Svelte TypeScript checking and declares path aliases `@/*`, `@features/*`, `@shared/*`, and `@services/*` mapping to `ui/src/*`, `ui/src/features/*`, `ui/src/shared/*`, and `ui/src/services/*` respectively.

#### Scenario: IDE resolves path aliases

- GIVEN a developer opens `ui/src` in a TypeScript-aware IDE
- WHEN they write `import { Track } from '@shared/types/models'`
- THEN the IDE resolves the import without errors
- AND no relative `../` chains are needed

#### Scenario: Build rejects misaligned aliases

- GIVEN `vite.config.ts` defines aliases that differ from `tsconfig.json` paths
- WHEN `npm run build` executes
- THEN the build MUST fail with a clear path-resolution error

### Requirement: FR-002 — Svelte Preprocess Configuration

The project MUST provide a `svelte.config.js` that configures `svelte-preprocess` with TypeScript support so that `<script lang="ts">` blocks compile correctly in Svelte components.

#### Scenario: TypeScript in Svelte components compiles

- GIVEN a Svelte component with `<script lang="ts">`
- WHEN `npm run build` executes
- THEN the component compiles without errors

#### Scenario: Missing preprocess config causes build failure

- GIVEN `svelte.config.js` has no preprocess plugin
- WHEN a component uses `<script lang="ts">`
- THEN `npm run build` MUST fail with a preprocess error

### Requirement: FR-003 — Vite Configuration with Aliases and Testing

The project MUST provide a `vite.config.ts` (replacing `vite.config.js`) that registers the Svelte plugin, path aliases matching `tsconfig.json`, and a Vitest configuration block with jsdom environment and Svelte testing-library setup.

#### Scenario: Dev server starts with aliases

- GIVEN `vite.config.ts` with all path aliases configured
- WHEN `npm run dev` starts
- THEN imports using `@/`, `@features/`, `@shared/`, `@services/` resolve correctly in the browser

#### Scenario: Vitest discovers Svelte tests

- GIVEN `vitest` configuration in `vite.config.ts` with jsdom environment
- WHEN `npm run test` runs
- THEN Vitest discovers and executes test files matching `**/*.test.ts` or `**/*.test.svelte`

### Requirement: FR-004 — Package Dependencies

The project MUST update `ui/package.json` to include `svelte-routing`, `vitest`, `@testing-library/svelte`, `jsdom`, `lucide-svelte`, `svelte-preprocess`, and `@sveltejs/vite-plugin-svelte` (already present). Dev dependencies MUST be separated from production dependencies.

#### Scenario: Clean install succeeds

- GIVEN `ui/package.json` with all required dependencies listed
- WHEN `npm install` runs in `ui/`
- THEN all packages install without peer dependency errors
- AND `npm run dev` starts without module-not-found errors

#### Scenario: Missing dependency detected at build

- GIVEN a dependency is removed from `package.json`
- WHEN `npm run build` executes
- THEN the build MUST fail with a module-not-found error referencing the missing package

### Requirement: FR-005 — Vitest Standalone Config

The project MUST provide a `vitest.config.ts` (or include Vitest config inline in `vite.config.ts`) that sets the test environment to jsdom, configures Svelte component transformation, and registers `@testing-library/svelte` helpers.

#### Scenario: Smoke test passes

- GIVEN a test file `App.test.ts` that renders the App component
- WHEN `npm run test` runs
- THEN the test passes and reports one successful test

#### Scenario: Test environment is jsdom

- GIVEN `vitest.config.ts` sets `environment: 'jsdom'`
- WHEN a test accesses `window` or `document`
- THEN the test MUST NOT throw a reference error

### Requirement: FR-006 — Directory Structure

The project MUST create the full directory tree per ARCHITECTURE.md §5.2: `app/`, `routes/Home/`, `routes/Search/`, `routes/Favorites/`, `routes/NowPlaying/`, `features/player/components/`, `features/search/components/`, `features/favorites/components/`, `features/library/components/`, `shared/components/`, `shared/stores/`, `shared/types/`, `shared/utils/`, `shared/constants/`, `shared/icons/`, `services/`, `styles/`. Empty directories MUST contain `.gitkeep`.

#### Scenario: Structure matches architecture spec

- GIVEN the project directory after setup
- WHEN listing `ui/src/` recursively
- THEN every directory from ARCHITECTURE.md §5.2 exists
- AND each empty directory contains a `.gitkeep` file

#### Scenario: No orphan directories from prototype

- GIVEN the old prototype structure (`ui/src/components/`, `ui/src/stores/`, `ui/src/themes/`)
- WHEN the restructure completes
- THEN those legacy directories MUST NOT exist alongside the new structure

### Requirement: FR-007 — i18n Relocation and Expansion

The project MUST relocate the existing `i18n/` module as-is (preserving all translation keys) and expand both `en.json` and `es.json` with keys for `routes.home`, `routes.search`, `routes.favorites`, `routes.now_playing`, `routes.settings`.

#### Scenario: i18n still works after relocation

- GIVEN the relocated `i18n/` module at `ui/src/i18n/`
- WHEN `initI18n()` is called and locale is set to `en`
- THEN `$t('app.title')` returns `"Helix"` as before the move

#### Scenario: New route keys resolve

- GIVEN expanded locale JSONs with route keys
- WHEN `$t('routes.home')` is called
- THEN it MUST return `"Home"` (en) or `"Inicio"` (es)