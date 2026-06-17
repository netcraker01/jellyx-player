<script lang="ts">
  import { t } from '@i18n';
  import {
    currentTrack,
    isPlaying,
  } from '@features/player/stores/player';
  import NowPlayingInfo from '@features/player/components/NowPlayingInfo.svelte';
  import Controls from '@features/player/components/Controls.svelte';
  import ProgressBar from '@features/player/components/ProgressBar.svelte';
  import Queue from '@features/player/components/Queue.svelte';
  import Visualizer from '@features/player/components/Visualizer.svelte';
</script>

<div class="page-now-playing">
  {#if $currentTrack}
    <div class="now-playing-layout">
      <div class="main-section">
        <NowPlayingInfo track={$currentTrack} />
        <div class="controls-section">
          <ProgressBar />
          <Controls disabled={!$currentTrack} />
        </div>
        <div class="visualizer-section">
          <Visualizer />
        </div>
      </div>
      <aside class="queue-section">
        <Queue />
      </aside>
    </div>
  {:else}
    <div class="empty-state">
      <p>{$t('now_playing.no_track')}</p>
    </div>
  {/if}
</div>

<style>
  .page-now-playing {
    padding: 1rem;
    height: 100%;
  }

  .now-playing-layout {
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 1.5rem;
    height: 100%;
  }

  .main-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
  }

  .controls-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    width: 100%;
  }

  .visualizer-section {
    width: 100%;
    min-height: 120px;
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-base, #0a0a0f);
  }

  .queue-section {
    border-left: 1px solid var(--border-color, #1f2937);
    overflow-y: auto;
    max-height: calc(100vh - 120px);
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 60%;
    color: var(--text-secondary, #9ca3af);
    font-size: 1.1rem;
  }
</style>