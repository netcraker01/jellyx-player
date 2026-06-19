<script lang="ts">
  import { t } from '@i18n';
  import { currentTrack, isPlaying } from '@features/player/stores/player';
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
      <div class="empty-state-graphic" aria-hidden="true"></div>
      <p class="empty-state-heading">{$t('now_playing.no_track')}</p>
      <p class="empty-state-hint">Open a track to start the experience</p>
    </div>
  {/if}
</div>

<style>
  .page-now-playing {
    padding: 1.5rem;
    height: 100%;
    background:
      radial-gradient(ellipse 80% 60% at 50% -20%, rgba(138, 92, 255, 0.08), transparent),
      radial-gradient(ellipse 60% 50% at 80% 100%, rgba(0, 229, 255, 0.05), transparent);
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
    border-radius: 16px;
    overflow: hidden;
    background: var(--bg-base, #0a0a0f);
    border: 1px solid var(--border-color, #1f2937);
  }

  .queue-section {
    border-left: 1px solid var(--border-color, #1f2937);
    overflow-y: auto;
    max-height: calc(100vh - 120px);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 60%;
    text-align: center;
  }

  .empty-state-graphic {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    margin-bottom: 1rem;
    background: var(--helix-gradient-primary);
    opacity: 0.25;
    flex-shrink: 0;
  }

  .empty-state-heading {
    color: var(--text-secondary, #9ca3af);
    font-size: 1.1rem;
    font-weight: 500;
    margin: 0;
  }

  .empty-state-hint {
    color: var(--text-secondary, #9ca3af);
    font-size: 0.85rem;
    opacity: 0.7;
    margin: 0.25rem 0 0;
  }
</style>
