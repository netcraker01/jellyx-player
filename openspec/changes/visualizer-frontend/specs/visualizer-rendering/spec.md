# Visualizer Rendering Specification

## Purpose

Canvas 2D spectrum bar visualization and two-mode display (Ambient Blur and Modo Cine) for real-time frequency data from the Rust FFT engine.

## Requirements

### Requirement: VF-001 — Canvas Element and Spectrum Bars

The Visualizer component MUST create a `<canvas>` element and render frequency bins as vertical bars using `CanvasRenderingContext2D`.

#### Scenario: Normal rendering with frequency data

- GIVEN the component is mounted and receiving `FrequencyData` from the store
- WHEN the `requestAnimationFrame` callback fires
- THEN the canvas MUST render vertical bars, one per frequency bin (or grouped bins), using `--color-accent` as the bar color
- AND bars MUST be drawn with smooth interpolation between frames

#### Scenario: No frequency data available

- GIVEN the component is mounted but no `FrequencyData` has been received
- WHEN the `requestAnimationFrame` callback fires
- THEN the canvas MUST render empty (clear or minimal idle animation)
- AND the component MUST NOT throw errors

### Requirement: VF-002 — Bar Styling with Design Tokens

Spectrum bars MUST use `--color-accent` CSS custom property for fill color and respond to frequency bin magnitude with proportional height.

#### Scenario: Bars reflect amplitude

- GIVEN frequency data with bins of varying magnitudes
- WHEN bars are rendered
- THEN each bar's height MUST be proportional to its bin's magnitude relative to `peak`
- AND bars MUST use `--color-accent` as the base fill color

### Requirement: VF-003 — Ambient Blur Mode

During normal navigation, the visualizer MUST display an Ambient Blur background effect using CSS `backdrop-filter: blur()` with a color overlay derived from `peak` and dominant frequency bin.

#### Scenario: Ambient blur active during playback

- GIVEN the player is actively playing audio and emitting frequency data
- WHEN the user is navigating the app (not in Modo Cine)
- THEN a background overlay MUST apply `backdrop-filter: blur()` with a configurable radius
- AND the overlay background color MUST derive from the `peak` value mapped to a color gradient

#### Scenario: Feature detection fallback

- GIVEN the browser does not support `backdrop-filter`
- WHEN the visualizer initializes
- THEN the ambient blur overlay MUST fall back to a semi-transparent colored background without blur
- AND a console warning SHOULD be logged

### Requirement: VF-004 — Modo Cine Toggle

The visualizer MUST provide a fullscreen immersive mode (Modo Cine) that expands the canvas to fill the viewport, hiding sidebar and controls.

#### Scenario: Enter Modo Cine

- GIVEN the visualizer is in Ambient mode
- WHEN the user activates the Modo Cine toggle (button in BottomBar)
- THEN the canvas MUST expand to fullscreen with a smooth CSS transition
- AND the sidebar and bottom bar MUST be hidden
- AND the spectrum bars MUST render at full viewport dimensions

#### Scenario: Exit Modo Cine

- GIVEN the visualizer is in Modo Cine
- WHEN the user presses Escape or clicks the close button
- THEN the visualizer MUST return to Ambient mode with a smooth transition
- AND the sidebar and bottom bar MUST be restored

### Requirement: VF-005 — requestAnimationFrame Rendering Loop

The visualizer MUST use a `requestAnimationFrame` loop to render frames, decoupled from the Tauri event rate.

#### Scenario: Decoupled rendering from events

- GIVEN `frequency-data` events arrive at a variable rate
- WHEN the rAF loop fires at display refresh rate
- THEN the visualizer MUST read the latest `FrequencyData` from the store
- AND render one frame per rAF callback
- AND MUST NOT render on every incoming event

#### Scenario: Cleanup on component destroy

- GIVEN the visualizer component has an active rAF loop and event listener
- WHEN the component is destroyed (Svelte `onDestroy`)
- THEN the rAF loop MUST be cancelled via `cancelAnimationFrame`
- AND the Tauri event listener MUST be unsubscribed via the returned `UnlistenFn`