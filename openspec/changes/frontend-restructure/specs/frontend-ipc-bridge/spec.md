# Frontend IPC Bridge Specification

## Purpose

Typed service stubs that wrap Tauri's `invoke` and `listen` APIs, providing TypeScript type safety for command calls and event subscriptions between Svelte and Rust.

## Requirements

### Requirement: FR-015 — Tauri Invoke/Listen Wrapper

`services/tauri.ts` MUST export typed `invokeCommand<T>` and `subscribeEvent<T>` wrappers around `@tauri-apps/api/core invoke` and `@tauri-apps/api/event listen`. These wrappers MUST handle the case where Tauri is unavailable (dev/browser mode) by returning safe fallback values instead of crashing.

#### Scenario: Typed command invocation in Tauri context

- GIVEN the app runs inside Tauri
- WHEN `invokeCommand<Track>('play', { trackId: 'abc' })` is called
- THEN the command is forwarded to Rust via `invoke('play', { trackId: 'abc' })`
- AND the result is typed as `Track`

#### Scenario: Graceful fallback outside Tauri

- GIVEN the app runs in a browser without Tauri (dev mode)
- WHEN `invokeCommand<Track>('play', { trackId: 'abc' })` is called
- THEN it MUST NOT throw an unhandled error
- AND it SHOULD return a mock/fallback value or a controlled rejection

### Requirement: FR-016 — Event Subscription Stubs

`services/events.ts` MUST export typed event subscription functions for each Rust→Svelte event: `onTrackChanged`, `onStateChanged`, `onQueueUpdated`, `onProgressTick`. Each function MUST accept a callback typed to the event payload and return an `UnlistenFn` for cleanup.

#### Scenario: Subscribe to track change events

- GIVEN `onTrackChanged` is imported from `services/events.ts`
- WHEN called with a callback `(track: Track) => void`
- THEN it registers a Tauri event listener for `track_changed`
- AND returns an `UnlistenFn` that removes the listener when called

#### Scenario: Event callback receives typed payload

- GIVEN a listener registered via `onTrackChanged`
- WHEN Rust emits a `track_changed` event
- THEN the callback receives a payload typed as `Track`
- AND TypeScript enforces the callback signature at compile time

### Requirement: FR-017 — Typed Command Stubs

`services/commands.ts` MUST export typed command functions: `search(query: string): Promise<Track[]>`, `play(trackId: string): Promise<void>`, `pause(): Promise<void>`, `next(): Promise<void>`, `previous(): Promise<void>`, `setVolume(volume: number): Promise<void>`, `toggleFavorite(trackId: string): Promise<void>`. Each MUST delegate to `invokeCommand` from `services/tauri.ts`.

#### Scenario: Search command returns typed results

- GIVEN `search` is imported from `services/commands.ts`
- WHEN `search('ambient mix')` is called inside Tauri
- THEN it invokes the `search` Rust command
- AND the return type is `Promise<Track[]>`

#### Scenario: Play command sends correct payload

- GIVEN `play` is imported from `services/commands.ts`
- WHEN `play('track-uuid-123')` is called
- THEN it invokes `invokeCommand<void>('play', { trackId: 'track-uuid-123' })`

### Requirement: FR-018 — TypeScript Model Mirrors

`shared/types/models.ts` MUST export TypeScript interfaces that mirror the Rust data models: `Track`, `Artist`, `Album`, and `Source` (enum: `YouTube | SoundCloud | Local`). The `Track` interface MUST include all fields from ARCHITECTURE.md §4.1: `id`, `source`, `sourceId`, `title`, `artist`, `album?`, `duration?`, `thumbnail?`, `streamUrl?`, `localPath?`, and `metadata: Record<string, string>`.

#### Scenario: Track interface matches Rust model

- GIVEN `Track` imported from `models.ts`
- WHEN a developer creates a `Track` object
- THEN TypeScript enforces all required fields (`id`, `source`, `sourceId`, `title`, `artist`)
- AND optional fields (`album`, `duration`, `thumbnail`, `streamUrl`, `localPath`) are allowed to be undefined

#### Scenario: Source enum restricts values

- GIVEN `Source` imported from `models.ts`
- WHEN a developer assigns `Source.YouTube` to a variable
- THEN TypeScript accepts it
- AND assigning `Source.Spotify` causes a compile error (not in the union)