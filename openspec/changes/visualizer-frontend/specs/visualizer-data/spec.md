# Visualizer Data Specification

## Purpose

TypeScript types, event subscription, and Svelte store for receiving and distributing frequency data from the Rust FFT engine to the visualizer component.

## Requirements

### Requirement: VF-006 — FrequencyData TypeScript Interface

The frontend MUST define a `FrequencyData` interface matching the Rust `serde(rename_all = "camelCase")` serialization format.

#### Scenario: Type matches Rust JSON payload

- GIVEN the Rust backend emits `FrequencyData` with `bins`, `sampleRate`, and `peak` fields
- WHEN the frontend receives the JSON payload via Tauri event
- THEN the TypeScript interface MUST have `bins: number[]`, `sampleRate: number`, and `peak: number`
- AND the field names MUST use camelCase matching the Rust serialization

### Requirement: VF-007 — Event Subscription for Frequency Data

The `events.ts` service MUST expose an `onFrequencyData` function that subscribes to the `frequency-data` Tauri event with typed payload.

#### Scenario: Subscribe to frequency data events

- GIVEN the Tauri runtime is available
- WHEN `onFrequencyData(callback)` is called
- THEN the function MUST subscribe to the `frequency-data` event via `subscribeEvent<FrequencyData>`
- AND return a `Promise<UnlistenFn>` for cleanup

#### Scenario: Browser fallback (no Tauri)

- GIVEN the app runs in browser without Tauri runtime
- WHEN `onFrequencyData(callback)` is called
- THEN it MUST return a no-op `UnlistenFn` (consistent with existing `subscribeEvent` behavior)

### Requirement: VF-008 — FrequencyData Svelte Store

The `player.ts` store MUST provide a `frequencyData` writable store that holds the latest `FrequencyData` received from events.

#### Scenario: Store updates on each event

- GIVEN the `onFrequencyData` subscription is active
- WHEN a new `FrequencyData` event is received
- THEN `frequencyData` store MUST be updated with the latest payload
- AND downstream subscribers MUST receive the updated value reactively

#### Scenario: Initial store state

- GIVEN no `frequency-data` event has been received yet
- WHEN a component reads `$frequencyData`
- THEN the value MUST be `null`
- AND the type MUST be `Writable<FrequencyData | null>`

### Requirement: VF-009 — Lifecycle Cleanup

The visualizer component MUST properly clean up both the rAF loop and the Tauri event listener on destroy.

#### Scenario: Full cleanup on component unmount

- GIVEN the Visualizer component has subscribed to `frequency-data` events and started a rAF loop
- WHEN the component is destroyed (Svelte `onDestroy`)
- THEN the rAF frame ID MUST be cancelled via `cancelAnimationFrame`
- AND the Tauri event listener MUST be unsubscribed via the returned `UnlistenFn`
- AND the `frequencyData` store SHOULD NOT continue receiving updates (unlisten prevents this)

#### Scenario: Cleanup on mode switch

- GIVEN the visualizer switches from Ambient to Modo Cine (or vice versa)
- WHEN the rendering mode changes
- THEN the rAF loop MUST continue running (only canvas dimensions change)
- AND the event subscription MUST remain active
- AND no duplicate subscriptions or loops MUST be created