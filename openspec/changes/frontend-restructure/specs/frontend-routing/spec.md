# Frontend Routing Specification

## Purpose

Path-based SPA routing via svelte-routing with Tauri SPA handler, enabling navigation between Home, Search, Favorites, and NowPlaying pages without full page reloads.

## Requirements

### Requirement: FR-008 — Router Setup with Four Routes

The application MUST configure `svelte-routing` with path-based routes mapping `/` to Home, `/search` to Search, `/favorites` to Favorites, and `/now-playing` to NowPlaying. The router MUST be initialized in `main.ts` before mounting the App component.

#### Scenario: Navigation renders correct page

- GIVEN the app is running
- WHEN the user navigates to `/search`
- THEN the Search route component renders in the router outlet
- AND the URL bar shows `/search`

#### Scenario: Default route renders Home

- GIVEN the app is running
- WHEN the user navigates to `/` or opens the app
- THEN the Home route component renders
- AND no 404 or blank screen appears

### Requirement: FR-009 — Tauri SPA Handler Configuration

The project MUST configure the Tauri SPA handler in `tauri.conf.json` so that all path-based routes resolve to `index.html`, enabling client-side routing to work inside the Tauri webview without 404 errors on direct navigation or refresh.

#### Scenario: Deep link refresh works

- GIVEN the app is running inside Tauri
- WHEN the user refreshes the page while on `/favorites`
- THEN the page loads correctly (not a 404)
- AND the Favorites route renders

#### Scenario: Direct URL entry resolves

- GIVEN the app is running inside Tauri
- WHEN the user enters `tauri://localhost/now-playing` directly
- THEN the NowPlaying route renders without error

### Requirement: FR-010 — Route Component Stubs

Each route MUST have a stub page component at `routes/{Name}/+page.svelte` that renders a heading with the route name and placeholder content. Stubs MUST be functional Svelte components that compile and render without errors.

#### Scenario: Home stub renders

- GIVEN `routes/Home/+page.svelte`
- WHEN the Home route is navigated to
- THEN the component renders a heading containing "Home"

#### Scenario: All four stubs exist and compile

- GIVEN the four route directories
- WHEN `npm run build` runs
- THEN all four `+page.svelte` files compile without errors