<script lang="ts">
  import { t } from '@i18n';
  import { currentTrack, isPlaying } from '@features/player/stores/player';
  import { albumArtUrl } from '@shared/utils/assetUrl';
  import NowPlayingInfo from '@features/player/components/NowPlayingInfo.svelte';
  import Controls from '@features/player/components/Controls.svelte';
  import ProgressBar from '@features/player/components/ProgressBar.svelte';
  import Queue from '@features/player/components/Queue.svelte';
  import Visualizer from '@features/player/components/Visualizer.svelte';

  $: backgroundArtUrl = $currentTrack ? albumArtUrl($currentTrack.thumbnail) : undefined;
  $: description = $currentTrack?.metadata?.description?.trim() || null;
</script>

<div class="page-now-playing">
  {#if backgroundArtUrl}
    <div
      class="artwork-background"
      style="background-image: url({backgroundArtUrl})"
      aria-hidden="true"
    ></div>
  {/if}

  {#if $currentTrack}
    <div class="now-playing-layout">
      <div class="main-section">
        <NowPlayingInfo track={$currentTrack} />
        <div class="controls-section">
          <ProgressBar />
          <Controls disabled={!$currentTrack} />
        </div>
        <div class="description-panel">
          {#if description}
            <p class="track-description">{description}</p>
          {:else}
            <p class="track-description empty">{$t('common.no_data') || 'No description available'}</p>
          {/if}
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
    position: relative;
    padding: 1.5rem;
    height: 100%;
    overflow: hidden;
    background:
      radial-gradient(ellipse 80% 60% at 50% -20%, rgba(138, 92, 255, 0.08), transparent),
      radial-gradient(ellipse 60% 50% at 80% 100%, rgba(0, 229, 255, 0.05), transparent);
  }

  .artwork-background {
    position: absolute;
    top: -10%;
    left: -10%;
    width: 120%;
    height: 120%;
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
    filter: blur(48px) brightness(0.55);
    opacity: 0.35;
    z-index: 0;
    pointer-events: none;
  }

  /* Fallback for browsers without filter: blur() support */
  @supports not (filter: blur(1px)) {
    .artwork-background {
      opacity: 0.15;
    }
  }

  .now-playing-layout {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 1.5rem;
    height: 100%;
  }

  .main-section {
    position: relative;
    z-index: 1;
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

  .description-panel {
    width: min(100%, 720px);
    padding: 1rem 1.1rem;
    border-radius: 16px;
    background: rgba(17, 24, 39, 0.52);
    border: 1px solid rgba(255, 255, 255, 0.06);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
  }

  .track-description {
    margin: 0;
    color: var(--text-secondary, #cbd5e1);
    font-size: 0.92rem;
    line-height: 1.55;
    text-align: left;
    white-space: pre-wrap;
    max-height: 10.5rem;
    overflow: auto;
  }

  .track-description.empty {
    color: var(--text-secondary, #94a3b8);
    font-style: italic;
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
    position: relative;
    z-index: 1;
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
    background: var(--jellyx-gradient-primary);
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
