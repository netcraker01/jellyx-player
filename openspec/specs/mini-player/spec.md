# Mini Player Specification

## Purpose

Define the behavior, layout, and visualizer contracts for the mini player,
including the compact horizontal Classic skin and its always-rendering
visualizer strip.

## Requirements

### Requirement: Classic Skin Compact Horizontal Layout

The system MUST render the Classic skin as a compact horizontal panel whose
shell height tracks its content (target ~90px at scale 1.0). The shell MUST
NOT use a fixed inner screen height inherited from other skins; it SHALL size
the screen area from its content. The shell width SHALL follow the skin
contract (`window.width`, 400px at scale 1.0) and the height SHALL auto-size,
clamped to the skin contract window height when content overflows.

#### Scenario: Classic skin default size

- GIVEN the Classic skin is selected at scale 1.0
- WHEN the mini player is rendered
- THEN the shell `--skin-card-width` is 400px and `--skin-card-height` is 100px
- AND the shell element renders with `data-kind="classic"`

#### Scenario: Classic shell auto-sizes to content

- GIVEN the Classic skin is rendered
- WHEN the shell layout resolves
- THEN the shell uses a flex column container with `height: auto`
- AND the inner screen area height is driven by content, not a fixed 172px rule

#### Scenario: Classic skin scales below 100%

- GIVEN the Classic skin is selected at scale 0.3
- WHEN the mini player is rendered
- THEN the window width is 120px and height is 30px proportionally
- AND the shell scale transform is 0.3

#### Scenario: Compact mode hides Full-app label

- GIVEN the Classic skin is rendered at a scale below 0.5
- WHEN the restore button renders
- THEN the "Full app" text is visually hidden but still accessible

### Requirement: Classic Visualizer Strip

The Classic skin MUST render the visualizer as a thin integrated strip inside
the screen panel, spanning the full panel width. The strip MUST NOT use a
heavy bordered box; it SHALL be borderless with a transparent background. The
strip target height is 12-14px so the Classic shell stays compact.

#### Scenario: Visualizer strip renders in Classic screen

- GIVEN the Classic skin is rendered
- WHEN the screen panel lays out
- THEN a visualizer element exists inside the screen with `flex: 0 0 auto`
- AND its height is between 12px and 14px with no visible border

#### Scenario: Visualizer strip does not collapse to zero height

- GIVEN the Classic skin is rendered with minimal content
- WHEN the visualizer strip resolves its size
- THEN the strip height is at least 12px
- AND the strip width matches the screen panel content width

### Requirement: Visualizer Always Renders

The mini player visualizer MUST always render and paint, regardless of skin
or playback state. When frequency data is null or empty, the visualizer SHALL
draw an idle bar pattern. The canvas MUST never remain at 0x0 after mount.

#### Scenario: Visualizer paints with no track (idle)

- GIVEN no track is loaded and frequencyData is null
- WHEN the mini player visualizer mounts
- THEN the canvas element exists and its width and height are each >= 1
- AND the idle bar pattern is drawn without throwing

#### Scenario: Visualizer paints with frequency data

- GIVEN frequencyData contains bins
- WHEN the rAF loop renders a frame
- THEN the bars renderer is called with a non-zero canvas width and height

#### Scenario: Canvas recovers from zero-size parent

- GIVEN the visualizer parent reports clientWidth/Height of 0
- WHEN the initial render frame runs
- THEN the canvas sizing falls back to its own bounding rect
- AND if that is also 0, the canvas is set to a non-zero fallback (80x12)

#### Scenario: Canvas resizes on parent resize

- GIVEN the visualizer is mounted
- WHEN the parent element resizes
- THEN a ResizeObserver triggers a canvas resize
- AND the canvas width and height are updated to the new parent size (floored to >= 1)

### Requirement: Classic Skin Window Contract

The Classic skin contract in `skins.ts` MUST declare a window width of 400 and
a height of 100 at scale 1.0. The height declares the intended compact target;
the shell layout MUST honor it as the upper bound while allowing content-driven
auto-height below it.

#### Scenario: Classic skin contract values

- GIVEN the `winamp-classic` skin definition is read
- WHEN its window contract is inspected
- THEN `window.width` is 400, `window.height` is 100, and `resizable` is false
- AND `kind` is `classic`

#### Scenario: Malformed skin dimensions are floored

- GIVEN a skin has non-finite or non-positive width or height
- WHEN `resolveMiniPlayerWindowSize` computes dimensions
- THEN tiny positive dimensions are floored to at least 1x1
- AND the skin never produces NaN or negative dimensions