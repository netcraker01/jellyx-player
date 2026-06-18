<script lang="ts">
  import { onMount } from 'svelte';
  import { Link, navigate } from 'svelte-routing';
  import { Search, AlertCircle, RotateCcw } from 'lucide-svelte';
  import { t } from '@i18n';
  import { homeStore, homeLoading, homeError } from '@features/home/stores/home';
  import TrackList from '@shared/components/TrackList.svelte';
  import ArtistCard from '@shared/components/ArtistCard.svelte';
  import AlbumCard from '@shared/components/AlbumCard.svelte';
  import type { HomeSnapshot, RecommendationItem } from '@shared/types/models';

  onMount(() => {
    homeStore.load();
  });

  function handleRetry() {
    homeStore.load();
  }

  function navigateToArtist(id: string) {
    navigate(`/artist/${id}`);
  }

  function navigateToAlbum(id: string) {
    navigate(`/album/${id}`);
  }

  $: snapshot = $homeStore;
  $: loading = $homeLoading;
  $: error = $homeError;
  $: hasRecentlyPlayed = Boolean(snapshot && snapshot.recentlyPlayed.length > 0);
  $: hasRecommendations = Boolean(snapshot && snapshot.recommendations.length > 0);

  function recentlyPlayedEntries(snapshot: HomeSnapshot) {
    return snapshot.recentlyPlayed.map((entry) => ({ track: entry.track }));
  }

  function recommendedTrackEntries(item: RecommendationItem & { type: 'Track' }) {
    return [{ track: item.track }];
  }
</script>

<div class="page-home">
  <h1>{$t('routes.home')}</h1>

  {#if loading && !snapshot}
    <p class="loading">{$t('common.loading')}</p>
  {:else if error}
    <div class="error-state" role="alert">
      <AlertCircle size={48} />
      <p>{error}</p>
      <button class="retry-btn" on:click={handleRetry}>
        <RotateCcw size={16} />
        {$t('common.retry')}
      </button>
    </div>
  {:else if !hasRecentlyPlayed && !hasRecommendations}
    <div class="empty-state">
      <Search size={48} />
      <p>{$t('home.empty_recommendations')}</p>
      <Link to="/search" class="search-link">
        <Search size={16} />
        {$t('home.go_search')}
      </Link>
    </div>
  {:else}
    {#if hasRecentlyPlayed}
      <section class="recently-played">
        <h2>{$t('home.recently_played')}</h2>
        <TrackList tracks={snapshot ? recentlyPlayedEntries(snapshot) : []} />
      </section>
    {/if}

    {#if hasRecommendations}
      <section class="recommendations">
        <h2>{$t('home.recommended')}</h2>
        <ul class="recommendation-list">
          {#each snapshot.recommendations as item (recommendationKey(item))}
            <li class="recommendation-item">
              <span class="reason-label">{item.reason}</span>
              {#if item.type === 'Track'}
                <TrackList tracks={recommendedTrackEntries(item)} showActions={false} showSource={false} />
              {:else if item.type === 'Artist'}
                <button
                  class="card-btn"
                  on:click={() => navigateToArtist(item.id)}
                  aria-label="Open artist {item.name}"
                >
                  <ArtistCard name={item.name} thumbnail={item.thumbnail} />
                </button>
              {:else if item.type === 'Album'}
                <button
                  class="card-btn"
                  on:click={() => navigateToAlbum(item.id)}
                  aria-label="Open album {item.title}"
                >
                  <AlbumCard title={item.title} artist={item.artist} thumbnail={item.cover} />
                </button>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/if}
</div>

<script lang="ts" context="module">
  import type { RecommendationItem as KeyItem } from '@shared/types/models';

  /** Stable key for recommendation list keyed each block. */
  export function recommendationKey(item: KeyItem): string {
    if (item.type === 'Track') return `track-${item.track.id}`;
    return `${item.type.toLowerCase()}-${item.id}`;
  }
</script>

<style>
  .page-home {
    padding: 1rem;
  }

  h1 {
    color: var(--text-primary, #e0e0e0);
    font-size: 1.5rem;
    margin-bottom: 1.5rem;
  }

  h2 {
    color: var(--text-secondary, #9ca3af);
    font-size: 1.1rem;
    margin-bottom: 0.75rem;
    font-weight: 500;
  }

  .loading {
    color: var(--text-secondary, #9ca3af);
  }

  .empty-state,
  .error-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    color: var(--text-secondary, #9ca3af);
    text-align: center;
  }

  .empty-state p,
  .error-state p {
    font-size: 1rem;
  }

  .error-state p {
    color: #fca5a5;
  }

  .retry-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1.25rem;
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 24px;
    background: transparent;
    color: var(--color-accent, #6366f1);
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.2s, color 0.2s;
  }

  .retry-btn:hover {
    background: var(--color-accent, #6366f1);
    color: white;
  }

  :global(.search-link) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 1.25rem;
    border: 1px solid var(--color-accent, #6366f1);
    border-radius: 24px;
    color: var(--color-accent, #6366f1);
    font-size: 0.9rem;
    transition: background 0.2s, color 0.2s;
  }

  :global(.search-link:hover) {
    background: var(--color-accent, #6366f1);
    color: white;
  }

  .recently-played,
  .recommendations {
    margin-bottom: 2rem;
  }

  .recommendation-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 1rem;
  }

  .recommendation-item {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .reason-label {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.75rem;
  }

  .card-btn {
    all: unset;
    display: block;
    cursor: pointer;
  }

  .card-btn :global(.artist-card),
  .card-btn :global(.album-card) {
    width: 100%;
  }
</style>
