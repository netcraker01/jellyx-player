import { writable } from 'svelte/store';

export type MiniPlayerControl = 'previous' | 'playPause' | 'next';

export interface MiniPlayerSkin {
  id: string;
  name: string;
  description: string;
  author: string;
  shape: 'rounded-rectangle';
  window: {
    width: number;
    height: number;
    resizable: boolean;
  };
  layout: {
    artwork: 'screen';
    controls: MiniPlayerControl[];
    progress: 'screen-bar';
  };
  theme: {
    shell: string;
    shellEdge: string;
    screen: string;
    screenText: string;
    accent: string;
    controlSurface: string;
    controlText: string;
  };
}

export interface MiniPlayerWindowSize {
  width: number;
  height: number;
}

export const MINI_PLAYER_WINDOW_BOUNDS = {
  minWidth: 240,
  minHeight: 240,
  maxWidth: 640,
  maxHeight: 720,
} as const;

export const MINI_PLAYER_SKINS: readonly MiniPlayerSkin[] = [
  {
    id: 'ipod-classic',
    name: 'iPod Classic',
    description: 'A compact click-wheel inspired skin for the first Helix mini player.',
    author: 'Helix',
    shape: 'rounded-rectangle',
    window: { width: 320, height: 480, resizable: false },
    layout: {
      artwork: 'screen',
      controls: ['previous', 'playPause', 'next'],
      progress: 'screen-bar',
    },
    theme: {
      shell: '#f1f2f4',
      shellEdge: '#cfd3da',
      screen: '#d9e2d4',
      screenText: '#1f2933',
      accent: '#6b7280',
      controlSurface: '#fbfbfc',
      controlText: '#1f2933',
    },
  },
  {
    id: 'graphite-pocket',
    name: 'Graphite Pocket',
    description: 'A smaller graphite skin for compact desktop placement.',
    author: 'Helix',
    shape: 'rounded-rectangle',
    window: { width: 300, height: 480, resizable: false },
    layout: {
      artwork: 'screen',
      controls: ['previous', 'playPause', 'next'],
      progress: 'screen-bar',
    },
    theme: {
      shell: '#2f343d',
      shellEdge: '#111827',
      screen: '#c8d4c0',
      screenText: '#111827',
      accent: '#93c5fd',
      controlSurface: '#f3f4f6',
      controlText: '#111827',
    },
  },
] as const;

export type MiniPlayerSkinId = (typeof MINI_PLAYER_SKINS)[number]['id'];

export const DEFAULT_MINI_PLAYER_SKIN: MiniPlayerSkinId = 'ipod-classic';

const MINI_PLAYER_SKIN_KEY = 'helix-mini-player-skin';

export function resolveMiniPlayerSkin(id: string | null | undefined): MiniPlayerSkin {
  return MINI_PLAYER_SKINS.find((skin) => skin.id === id) ?? MINI_PLAYER_SKINS[0];
}

function clampSkinDimension(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

export function resolveMiniPlayerWindowSize(skin: MiniPlayerSkin): MiniPlayerWindowSize {
  return {
    width: clampSkinDimension(skin.window.width, MINI_PLAYER_WINDOW_BOUNDS.minWidth, MINI_PLAYER_WINDOW_BOUNDS.maxWidth),
    height: clampSkinDimension(skin.window.height, MINI_PLAYER_WINDOW_BOUNDS.minHeight, MINI_PLAYER_WINDOW_BOUNDS.maxHeight),
  };
}

function readPersistedSkin(): MiniPlayerSkinId {
  try {
    const raw = localStorage.getItem(MINI_PLAYER_SKIN_KEY);
    return resolveMiniPlayerSkin(raw).id as MiniPlayerSkinId;
  } catch {
    return DEFAULT_MINI_PLAYER_SKIN;
  }
}

export const selectedMiniPlayerSkinId = writable<MiniPlayerSkinId>(readPersistedSkin());

selectedMiniPlayerSkinId.subscribe((id) => {
  try {
    localStorage.setItem(MINI_PLAYER_SKIN_KEY, id);
  } catch {
    // localStorage unavailable — keep the selected skin in memory for this session.
  }
});

export function activateMiniPlayerSkin(id: string): void {
  selectedMiniPlayerSkinId.set(resolveMiniPlayerSkin(id).id as MiniPlayerSkinId);
}
