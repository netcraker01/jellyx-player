<script lang="ts">
  import Sidebar from './layout/Sidebar.svelte';
  import BottomBar from './layout/BottomBar.svelte';
  import ToastContainer from '@shared/components/ToastContainer.svelte';
  import { currentPath } from './router/navigation';
  import Home from '../routes/Home/Page.svelte';
  import Search from '../routes/Search/Page.svelte';
  import Favorites from '../routes/Favorites/Page.svelte';
  import NowPlaying from '../routes/NowPlaying/Page.svelte';
  import Library from '../routes/Library/Page.svelte';
  import Settings from '../routes/Settings/Page.svelte';
  import ArtistPage from '../routes/Artist/Page.svelte';
  import AlbumPage from '../routes/Album/Page.svelte';
  import { frequencyData, modoCineActive } from '@features/player/stores/player';

  type RouteMatch =
    | { name: 'home' }
    | { name: 'search' }
    | { name: 'favorites' }
    | { name: 'now-playing' }
    | { name: 'library' }
    | { name: 'settings' }
    | { name: 'artist'; id: string }
    | { name: 'album'; id: string };

  function resolveRoute(path: string): RouteMatch {
    if (path === '/search') return { name: 'search' };
    if (path === '/favorites') return { name: 'favorites' };
    if (path === '/now-playing') return { name: 'now-playing' };
    if (path === '/library') return { name: 'library' };
    if (path === '/settings') return { name: 'settings' };
    if (path.startsWith('/artist/')) return { name: 'artist', id: decodeURIComponent(path.slice('/artist/'.length)) };
    if (path.startsWith('/album/')) return { name: 'album', id: decodeURIComponent(path.slice('/album/'.length)) };
    return { name: 'home' };
  }

  $: route = resolveRoute($currentPath);
</script>

<div class="app-shell" class:modo-cine-active={$modoCineActive}>
  <!-- Ambient Blur overlay: shows during navigation when frequency data exists and not in Modo Cine -->
  {#if $frequencyData && !$modoCineActive}
    <div
      class="ambient-blur-overlay"
      style="background-color: hsl({240 + $frequencyData.peak * 120}, 70%, 25%)"
    ></div>
  {/if}
  <Sidebar />
  <main class="content">
    {#if route.name === 'search'}
      <Search />
    {:else if route.name === 'favorites'}
      <Favorites />
    {:else if route.name === 'now-playing'}
      <NowPlaying />
    {:else if route.name === 'library'}
      <Library />
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
  }

  .app-shell.modo-cine-active .content,
  .app-shell.modo-cine-active :global(.sidebar),
  .app-shell.modo-cine-active :global(.bottom-bar) {
    display: none;
  }

  .content {
    grid-area: content;
    overflow-y: auto;
    padding: 1.5rem;
  }

  .ambient-blur-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    z-index: 0;
    pointer-events: none;
    opacity: var(--viz-overlay-opacity, 0.3);
    -webkit-backdrop-filter: blur(var(--viz-blur-radius, 20px));
    backdrop-filter: blur(var(--viz-blur-radius, 20px));
    transition: background-color 0.1s ease;
  }

  /* Fallback for browsers without backdrop-filter support */
  @supports not (backdrop-filter: blur(1px)) {
    .ambient-blur-overlay {
      opacity: 0.5;
    }
  }
</style>
