# Frontend Layout Specification

## Purpose

App shell layout with persistent Sidebar navigation, BottomBar player controls area, and router outlet in the center. Dark-mode theme token system for consistent styling.

## Requirements

### Requirement: FR-011 — App Shell Layout

`App.svelte` MUST be rewritten as a layout shell with three areas: Sidebar (fixed left), BottomBar (fixed bottom, full width), and router outlet (center, scrollable). The layout MUST use CSS Grid or Flexbox to partition the viewport.

#### Scenario: Layout renders three areas

- GIVEN the app is running
- WHEN `App.svelte` mounts
- THEN the Sidebar is visible on the left
- AND the BottomBar is visible at the bottom
- AND the router outlet fills the remaining center area

#### Scenario: Sidebar remains on route change

- GIVEN the user is on `/search`
- WHEN they navigate to `/favorites`
- THEN the Sidebar and BottomBar remain visible (not re-mounted)
- AND only the router outlet content changes

### Requirement: FR-012 — Sidebar Navigation Component

The Sidebar MUST contain navigation links for Home, Search, Favorites, and NowPlaying using `lucide-svelte` icons. The active route MUST be visually distinguished. Links MUST use `svelte-routing`'s `Link` component for client-side navigation.

#### Scenario: Active route is highlighted

- GIVEN the user navigates to `/search`
- WHEN the Sidebar renders
- THEN the Search link shows a visual indicator of the active state (e.g., accent color, bold)
- AND other links remain in default style

#### Scenario: Sidebar links trigger client-side navigation

- GIVEN the Sidebar with Link components
- WHEN the user clicks the Favorites link
- THEN the URL changes to `/favorites` without a full page reload
- AND the router outlet renders the Favorites page

### Requirement: FR-013 — BottomBar Player Controls Stub

The BottomBar MUST render as a fixed bar at the bottom of the viewport containing placeholder areas for: track info (left), play/pause/skip controls (center), and volume control (right). The BottomBar MUST be a stub — no functional playback logic, only layout and placeholder elements.

#### Scenario: BottomBar is always visible

- GIVEN the app is running on any route
- WHEN the viewport renders
- THEN the BottomBar is visible at the bottom with track info, controls, and volume areas

#### Scenario: BottomBar does not overlap content

- GIVEN the BottomBar has a fixed height
- WHEN the router outlet content scrolls
- THEN the content area MUST have bottom padding equal to the BottomBar height
- AND content is not obscured by the BottomBar

### Requirement: FR-014 — Dark Theme Token System

The project MUST provide `styles/tokens.css` with CSS custom properties defining the dark theme: background levels (`--bg-base`, `--bg-surface`, `--bg-elevated`), text levels (`--text-primary`, `--text-secondary`), accent color (`--color-accent`), and spacing/sizing tokens. `global.css` MUST import `tokens.css` and apply baseline dark-mode styles (background, text color, font).

#### Scenario: Theme tokens are applied globally

- GIVEN `tokens.css` is imported in `global.css`
- WHEN the app renders
- THEN `var(--bg-base)` resolves to a near-black color
- AND `var(--text-primary)` resolves to an off-white color
- AND `var(--color-accent)` resolves to a vibrant color

#### Scenario: Components consume tokens

- GIVEN a Svelte component using `var(--bg-surface)` as background
- WHEN the token value in `tokens.css` changes
- THEN the component background updates automatically without code changes