<script lang="ts">
  import Sidebar from './layout/Sidebar.svelte';
  import BottomBar from './layout/BottomBar.svelte';
  import ToastContainer from '@shared/components/ToastContainer.svelte';
  import { currentPath } from './router/navigation';
import Home from '../routes/Home/Page.svelte';
import Search from '../routes/Search/Page.svelte';
import PlaylistsPage from '../routes/Playlists/Page.svelte';
import PlaylistDetail from '../routes/Playlists/PlaylistDetail.svelte';
import NowPlaying from '../routes/NowPlaying/Page.svelte';
import Library from '../routes/Library/Page.svelte';
import FolderDetail from '../routes/Library/FolderDetail.svelte';
import Settings from '../routes/Settings/Page.svelte';
import ArtistPage from '../routes/Artist/Page.svelte';
import AlbumPage from '../routes/Album/Page.svelte';
import Visualizer from '@features/player/components/Visualizer.svelte';
import { frequencyData, cinematicMode, cinematicIntensity, modoCineActive } from '@features/player/stores/player';
import UpdateAvailableModal from '@features/updater/UpdateAvailableModal.svelte';

  // Cinematic ambient background is active when the Settings cinematic-mode
  // toggle is ON and there is frequency data available. This is INDEPENDENT
  // from the bottom-bar visualizer button (modoCineActive), which drives the
  // Winamp-style fullscreen overlay in Visualizer.svelte.
  $: cineOn = $cinematicMode && $frequencyData != null;

  // Derive a cheap bass/low-end pulse by averaging the first ~25% of bins,
  // and pass the peak to the CSS via custom properties so Svelte reactivity
  // updates the gradients at ~60fps without a JS animation loop.
  $: cinePulse = (() => {
    const fd = $frequencyData;
    if (fd == null) return { bass: 0, peak: 0 };
    const bins = fd.bins;
    if (!bins || bins.length === 0) return { bass: 0, peak: fd.peak ?? 0 };
    const n = Math.max(1, Math.floor(bins.length * 0.25));
    let sum = 0;
    for (let i = 0; i < n; i++) sum += bins[i];
    const bass = Math.min(1, sum / n);
    return { bass, peak: fd.peak ?? 0 };
  })();

  type RouteMatch =
    | { name: 'home' }
    | { name: 'search' }
    | { name: 'playlists' }
    | { name: 'playlist-detail'; id: string }
    | { name: 'now-playing' }
    | { name: 'library' }
    | { name: 'folder-detail'; folderPath: string }
    | { name: 'settings' }
    | { name: 'artist'; id: string }
    | { name: 'album'; id: string };

  function resolveRoute(path: string): RouteMatch {
    if (path === '/search') return { name: 'search' };
    if (path === '/playlists') return { name: 'playlists' };
    if (path === '/now-playing') return { name: 'now-playing' };
    if (path === '/library') return { name: 'library' };
    if (path === '/settings') return { name: 'settings' };
    if (path.startsWith('/library/folder/')) return { name: 'folder-detail', folderPath: decodeURIComponent(path.slice('/library/folder/'.length)) };
    if (path.startsWith('/playlists/')) return { name: 'playlist-detail', id: decodeURIComponent(path.slice('/playlists/'.length)) };
    if (path.startsWith('/artist/')) return { name: 'artist', id: decodeURIComponent(path.slice('/artist/'.length)) };
    if (path.startsWith('/album/')) return { name: 'album', id: decodeURIComponent(path.slice('/album/'.length)) };
    return { name: 'home' };
  }

  $: route = resolveRoute($currentPath);
</script>

<div class="app-shell" class:cinematic-active={cineOn}>
  <!-- Cinematic ambient background: layered reactive gradients/glow that paint
       BEHIND app content (z-index: -1 within an isolated stacking context on
       .app-shell). Only rendered when the user opted in and frequency data is
       available. Opacity is driven solely by the user intensity slider so
       quiet passages don't make the background flicker. -->
  {#if cineOn}
    <div
      class="cinematic-layer"
      style="--cine-peak: {cinePulse.peak}; --cine-bass: {cinePulse.bass}; --cine-intensity: {$cinematicIntensity};"
      aria-hidden="true"
    >
      <div class="cinematic-wash"></div>
      <div class="cinematic-glow"></div>
      <div class="cinematic-vignette"></div>
    </div>
  {/if}

  <Sidebar />
  <main class="content">
    {#if route.name === 'search'}
      <Search />
    {:else if route.name === 'playlists'}
      <PlaylistsPage />
    {:else if route.name === 'playlist-detail'}
      <PlaylistDetail id={route.id} />
    {:else if route.name === 'now-playing'}
      <NowPlaying />
    {:else if route.name === 'library'}
      <Library />
    {:else if route.name === 'folder-detail'}
      <FolderDetail folderPath={route.folderPath} />
    {:else if route.name === 'settings'}
      <Settings />
    {:else if route.name === 'artist'}
      <ArtistPage id={route.id} />
    {:else if route.name === 'album'}
      <AlbumPage id={route.id} />
    {:else}
      <Home />
    {/if}
  </main>
  <BottomBar />
  <ToastContainer />
  <UpdateAvailableModal />
  {#if $modoCineActive}
    <div class="visualizer-embed">
      <Visualizer />
    </div>
  {/if}
</div>

<style>
  .app-shell {
    display: grid;
    grid-template-columns: 240px 1fr;
    grid-template-rows: 1fr auto;
    grid-template-areas:
      "sidebar content"
      "sidebar bottombar";
    height: 100vh;
    background: var(--bg-base, #0a0a0f);
    color: var(--text-primary, #e0e0e0);
    font-family: 'Inter', sans-serif;
    position: relative;
    /* Create a new stacking context so the cinematic background (z-index: -1)
       stays scoped to the shell and paints behind the static grid children. */
    isolation: isolate;
  }

  .content {
    grid-area: content;
    overflow-y: auto;
    padding: 1.5rem;
    /* Position so it paints above the negative-z cinematic background. */
    position: relative;
    z-index: 1;
  }

  /* ── Cinematic ambient background ──────────────────────────────────
     Paints BEHIND app content. Opacity is driven solely by the user intensity
     slider (--cine-intensity, 0..1); reactivity comes from --cine-peak and
     --cine-bass custom properties (0..1) feeding gradient color stops, so the
     effect animates at display refresh without a JS rAF loop. Per-layer alphas
     are kept low (0.55/0.5/0.3) and blur radii large (48/72px) so the background
     stays soft and does not compete with text. */
  .cinematic-layer {
    position: fixed;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    opacity: var(--cine-intensity, 0.5);
    overflow: hidden;
  }

  .cinematic-wash {
    position: absolute;
    inset: -10%;
    background:
      radial-gradient(
        ellipse 60% 50% at 20% 30%,
        hsla(calc(240 + var(--cine-peak, 0) * 120), 70%, 25%, 0.55),
        transparent 70%
      ),
      radial-gradient(
        ellipse 50% 40% at 80% 70%,
        hsla(calc(200 + var(--cine-bass, 0) * 80), 65%, 22%, 0.55),
        transparent 70%
      );
    filter: blur(48px);
    transition: opacity 0.2s ease;
  }

  .cinematic-glow {
    position: absolute;
    inset: 20% 25%;
    background: radial-gradient(
      circle,
      hsla(calc(260 + var(--cine-peak, 0) * 60), 75%, 55%, calc(0.15 + var(--cine-bass, 0) * 0.35)),
      transparent 60%
    );
    filter: blur(72px);
    transition: opacity 0.15s ease;
  }

  .cinematic-vignette {
    position: absolute;
    inset: 0;
    background: radial-gradient(
      ellipse at center,
      transparent 55%,
      rgba(0, 0, 0, 0.3) 100%
    );
  }

  .visualizer-embed {
    position: fixed;
    inset: 0;
    z-index: 99;
    pointer-events: auto;
  }

</style>
