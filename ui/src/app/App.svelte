<script>
  import { Router, Route } from 'svelte-routing';
  import Sidebar from './layout/Sidebar.svelte';
  import BottomBar from './layout/BottomBar.svelte';
  import Home from '../routes/Home/Page.svelte';
  import Search from '../routes/Search/Page.svelte';
  import Favorites from '../routes/Favorites/Page.svelte';
  import NowPlaying from '../routes/NowPlaying/Page.svelte';
</script>

<div class="app-shell">
  <Sidebar />
  <main class="content">
    <Router>
      <Route path="/" let:params>
        <Home />
      </Route>
      <Route path="/search" let:params>
        <Search />
      </Route>
      <Route path="/favorites" let:params>
        <Favorites />
      </Route>
      <Route path="/now-playing" let:params>
        <NowPlaying />
      </Route>
    </Router>
  </main>
  <BottomBar />
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
  }

  .content {
    grid-area: content;
    overflow-y: auto;
    padding: 1.5rem;
  }
</style>