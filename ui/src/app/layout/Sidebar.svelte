<script>
  import { Link, useLocation } from 'svelte-routing';
  import { Home, Search, Heart, Music } from 'lucide-svelte';
  import { t } from '../../i18n';

  const locationStore = useLocation();
</script>

<aside class="sidebar">
  <h1 class="logo">Helix</h1>
  <nav>
    <Link to="/" class="nav-link{$locationStore.pathname === '/' ? ' active' : ''}">
      <Home size={20} />
      <span>{$t('routes.home')}</span>
    </Link>
    <Link to="/search" class="nav-link{$locationStore.pathname === '/search' ? ' active' : ''}">
      <Search size={20} />
      <span>{$t('routes.search')}</span>
    </Link>
    <Link to="/favorites" class="nav-link{$locationStore.pathname === '/favorites' ? ' active' : ''}">
      <Heart size={20} />
      <span>{$t('routes.favorites')}</span>
    </Link>
    <Link to="/now-playing" class="nav-link{$locationStore.pathname === '/now-playing' ? ' active' : ''}">
      <Music size={20} />
      <span>{$t('routes.now_playing')}</span>
    </Link>
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

  .logo {
    color: var(--color-accent, #6366f1);
    font-size: 1.5rem;
    margin: 0;
    padding: 0 1.5rem 1rem;
    border-bottom: 1px solid var(--border-color, #1f2937);
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
    padding: 0.6rem 0.75rem;
    border-radius: 8px;
    color: var(--text-secondary, #9ca3af);
    text-decoration: none;
    font-size: 0.9rem;
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