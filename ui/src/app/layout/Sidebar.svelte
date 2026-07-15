<script lang="ts">
  import BrandLockup from './BrandLockup.svelte';
  import { sidebarCollapsed } from './sidebar';
  import { Home, Search, ListMusic, Music, Library, Settings, PanelLeftClose, PanelLeftOpen } from 'lucide-svelte';
  import { t } from '@i18n';
  import { currentPath, navigate } from '../router/navigation';

  $: iconSize = $sidebarCollapsed ? 20 : 20;
</script>

<aside class="sidebar">
  <BrandLockup collapsed={$sidebarCollapsed} />
  <nav>
    <button type="button" class="nav-link{$currentPath === '/' ? ' active' : ''}" on:click={() => navigate('/')}>
      <Home size={iconSize} />
      <span>{$t('routes.home')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/search' ? ' active' : ''}" on:click={() => navigate('/search')}>
      <Search size={iconSize} />
      <span>{$t('routes.search')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/playlists' ? ' active' : ''}" on:click={() => navigate('/playlists')}>
      <ListMusic size={iconSize} />
      <span>{$t('routes.playlists')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/now-playing' ? ' active' : ''}" on:click={() => navigate('/now-playing')}>
      <Music size={iconSize} />
      <span>{$t('routes.now_playing')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/library' ? ' active' : ''}" on:click={() => navigate('/library')}>
      <Library size={iconSize} />
      <span>{$t('routes.library')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/settings' ? ' active' : ''}" on:click={() => navigate('/settings')}>
      <Settings size={iconSize} />
      <span>{$t('routes.settings')}</span>
    </button>
  </nav>
  <div class="sidebar-footer">
    <button type="button" class="collapse-toggle" on:click={() => sidebarCollapsed.update(v => !v)} aria-label={$sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
      {#if $sidebarCollapsed}
        <PanelLeftOpen size={20} />
      {:else}
        <PanelLeftClose size={20} />
      {/if}
    </button>
  </div>
</aside>

<style>
  .sidebar {
    grid-area: sidebar;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface, #111827);
    border-right: 1px solid var(--border-color, #1f2937);
    padding: 1rem 0;
    position: relative;
    z-index: 1;
    width: 240px;
    transition: width 0.25s ease;
    overflow: hidden;
  }

  :global(.sidebar-collapsed) .sidebar {
    width: 80px;
  }

  :global(.sidebar-collapsed) :global(.nav-link span) {
    opacity: 0;
    width: 0;
    overflow: hidden;
    transition: opacity 0.15s ease;
  }

  :global(.sidebar-collapsed) :global(.nav-link) {
    justify-content: center;
    padding: 0.6rem;
  }

  :global(.sidebar-collapsed) :global(.brand-lockup) {
    border-bottom: none;
    padding: 0 0 1rem;
  }

  .sidebar-footer {
    margin-top: auto;
    padding: 0.5rem 0.75rem;
    border-top: 1px solid var(--border-color, #1f2937);
  }

  .collapse-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    padding: 0.6rem;
    border-radius: 8px;
    border: 0;
    background: transparent;
    color: var(--text-secondary, #9ca3af);
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }

  .collapse-toggle:hover {
    background: var(--bg-elevated, #1f2937);
    color: var(--text-primary, #e0e0e0);
  }

  :global(.app-shell.cinematic-active) .sidebar {
    background: rgba(17, 24, 39, 0.58);
    -webkit-backdrop-filter: blur(10px);
    backdrop-filter: blur(10px);
    border-right-color: rgba(255, 255, 255, 0.08);
  }

  /* Modo cine: fully transparent — the visualizer shows through with no
     delimiting box. Only the nav text/icons paint. */
  :global(.app-shell.cine-background) .sidebar {
    background: transparent;
    -webkit-backdrop-filter: none;
    backdrop-filter: none;
    border-right-color: rgba(255, 255, 255, 0.06);
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.5rem 0.75rem;
    margin-top: 0.5rem;
  }

  :global(.nav-link) {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    border: 0;
    background: transparent;
    color: var(--text-secondary, #9ca3af);
    text-decoration: none;
    font-size: 0.9rem;
    text-align: left;
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }

  :global(.nav-link:hover) {
    background: var(--bg-elevated, #1f2937);
    color: var(--text-primary, #e0e0e0);
  }

  :global(.nav-link.active) {
    background: var(--bg-elevated, #1f2937);
    color: var(--color-accent, #6366f1);
  }
</style>
