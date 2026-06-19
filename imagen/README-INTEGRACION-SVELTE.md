# Helix — assets para integración en Svelte

Este paquete contiene una versión SVG vectorial del icono de Helix con fondo blanco, más un componente Svelte listo para integrar en la interfaz.

## Archivos

- `helix-icon-white.svg`: icono vectorial independiente.
- `HelixIcon.svelte`: componente reutilizable para Svelte/SvelteKit.
- `helix-theme.css`: variables CSS de marca.

## Uso recomendado en Svelte

Copia `HelixIcon.svelte` en `src/lib/components/HelixIcon.svelte`.

```svelte
<script lang="ts">
  import HelixIcon from '$lib/components/HelixIcon.svelte';
</script>

<HelixIcon size={64} />
```

También puedes pasar una clase:

```svelte
<HelixIcon size={96} className="app-logo" />
```

```css
.app-logo {
  filter: drop-shadow(0 12px 24px rgb(11 15 43 / 14%));
}
```

## Uso como archivo estático

Coloca `helix-icon-white.svg` en:

```txt
static/brand/helix-icon-white.svg
```

Y úsalo así:

```svelte
<img src="/brand/helix-icon-white.svg" alt="Helix" width="64" height="64" />
```

## Tema CSS

Importa `helix-theme.css` en tu archivo global, por ejemplo:

```ts
// src/routes/+layout.svelte o src/app.css
import '$lib/styles/helix-theme.css';
```

Ejemplo de botón principal:

```css
.primary-button {
  background: var(--helix-gradient-primary);
  color: white;
  border: 0;
  border-radius: var(--helix-radius-md);
  box-shadow: var(--helix-shadow-soft);
}
```

## Tamaños sugeridos

- Sidebar / barra superior: `32px` a `40px`.
- Splash screen: `96px` a `160px`.
- Icono de aplicación: exportar desde SVG a `512x512`, `256x256`, `128x128`, `64x64`, `32x32`.

## Nota técnica

El SVG es editable y no depende de fuentes externas. Los degradados están definidos dentro del propio SVG, por lo que puede usarse directamente como asset estático o como componente embebido.
