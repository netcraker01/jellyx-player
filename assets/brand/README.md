# Helix brand assets

This folder contains the Helix brand kit: logo, app icon, design tokens, theme CSS, and a reusable Svelte icon component.

## Files

| File | Purpose |
|---|---|
| `logo-wide.png` | Horizontal logo for READMEs, landing pages, and social headers. |
| `app-icon.png` | Square app icon for stores and listings. |
| `icon.svg` | Vector icon (transparent) for web/app embedding. |
| `icon-white.png` | White icon variant. |
| `brand-sheet.png` | Full brand overview with colors, typography, and icon variations. |
| `brand-guide.pdf` | Brand identity PDF/template. |
| `design-tokens.json` | Colors, gradients, typography, and radii tokens. |
| `theme.css` | CSS custom properties matching the design tokens. |
| `HelixIcon.svelte` | Ready-to-use Svelte component of the Helix icon. |

## Using the Svelte icon component

Copy `HelixIcon.svelte` into your project (for example, `ui/src/lib/components/HelixIcon.svelte`) and import it:

```svelte
<script lang="ts">
  import HelixIcon from '$lib/components/HelixIcon.svelte';
</script>

<HelixIcon size={64} />
```

You can also pass a custom class:

```svelte
<HelixIcon size={96} className="app-logo" />
```

```css
.app-logo {
  filter: drop-shadow(0 12px 24px rgb(11 15 43 / 14%));
}
```

## Using the static SVG icon

Place `icon.svg` in your static assets folder and reference it:

```svelte
<img src="/brand/icon.svg" alt="Helix" width="64" height="64" />
```

## Theme CSS

Import `theme.css` in your global styles:

```ts
// src/app.css or +layout.svelte
import '../styles/brand/theme.css';
```

Example primary button:

```css
.primary-button {
  background: var(--helix-gradient-primary);
  color: white;
  border: 0;
  border-radius: var(--helix-radius-md);
  box-shadow: var(--helix-shadow-soft);
}
```

## Suggested icon sizes

- Sidebar / top bar: `32px` to `40px`
- Splash screen: `96px` to `160px`
- Application icon: export from SVG to `512x512`, `256x256`, `128x128`, `64x64`, `32x32`

## Note

The SVG is self-contained: gradients and shadows are defined inline, so it works as a static asset or as an embedded component without external dependencies.
