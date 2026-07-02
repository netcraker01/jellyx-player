<script lang="ts">
  import BrandLockup from './BrandLockup.svelte';
  import { Home, Search, ListMusic, Music, Library, Settings } from 'lucide-svelte';
  import { t } from '@i18n';
  import { currentPath, navigate } from '../router/navigation';
</script>

<aside class="sidebar">
  <BrandLockup />
  <nav>
    <button type="button" class="nav-link{$currentPath === '/' ? ' active' : ''}" on:click={() => navigate('/')}>
      <Home size={20} />
      <span>{$t('routes.home')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/search' ? ' active' : ''}" on:click={() => navigate('/search')}>
      <Search size={20} />
      <span>{$t('routes.search')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/playlists' ? ' active' : ''}" on:click={() => navigate('/playlists')}>
      <ListMusic size={20} />
      <span>{$t('routes.playlists')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/now-playing' ? ' active' : ''}" on:click={() => navigate('/now-playing')}>
      <Music size={20} />
      <span>{$t('routes.now_playing')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/library' ? ' active' : ''}" on:click={() => navigate('/library')}>
      <Library size={20} />
      <span>{$t('routes.library')}</span>
    </button>
    <button type="button" class="nav-link{$currentPath === '/settings' ? ' active' : ''}" on:click={() => navigate('/settings')}>
      <Settings size={20} />
      <span>{$t('routes.settings')}</span>
    </button>
  </nav>
</aside>

<style>
  .sidebar {
    grid-area: sidebar;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface, #111827);
    border-right: 1px solid var(--border-color, #1f2937);
    padding: 1rem 0;
  }

  :global(.app-shell.cinematic-active) .sidebar {
    background: rgba(17, 24, 39, 0.58);
    -webkit-backdrop-filter: blur(10px);
    backdrop-filter: blur(10px);
    border-right-color: rgba(255, 255, 255, 0.08);
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
