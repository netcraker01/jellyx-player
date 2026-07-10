import { getMigratedItem, setMigratedItem } from '@shared/utils/storage';
import { writable } from 'svelte/store';

export type MiniPlayerControl = 'previous' | 'playPause' | 'next';

export interface MiniPlayerSkin {
  id: string;
  name: string;
  description: string;
  author: string;
  kind: 'ipod' | 'classic';
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
  minWidth: 1,
  minHeight: 1,
  maxWidth: 640,
  maxHeight: 720,
} as const;

export const MINI_PLAYER_SCALE_BOUNDS = {
  min: 0.3,
  max: 1,
  step: 0.05,
  default: 1,
} as const;

export const MINI_PLAYER_SKINS: readonly MiniPlayerSkin[] = [
  {
    id: 'ipod-classic',
    name: 'iPod Classic',
    description: 'A compact click-wheel inspired skin for the first Jellyx mini player.',
    author: 'Jellyx',
    kind: 'ipod',
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
    author: 'Jellyx',
    kind: 'ipod',
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

const MINI_PLAYER_SKIN_SUFFIX = 'mini-player-skin';
const MINI_PLAYER_SCALE_SUFFIX = 'mini-player-scale';

export function resolveMiniPlayerSkin(id: string | null | undefined): MiniPlayerSkin {
  return MINI_PLAYER_SKINS.find((skin) => skin.id === id) ?? MINI_PLAYER_SKINS[0];
}

function resolveBaseSkinDimensions(skin: MiniPlayerSkin): MiniPlayerWindowSize {
  const fallback = resolveMiniPlayerSkin(DEFAULT_MINI_PLAYER_SKIN).window;
  if (
    !Number.isFinite(skin.window.width) ||
    skin.window.width <= 0 ||
    !Number.isFinite(skin.window.height) ||
    skin.window.height <= 0
  ) {
    return { width: fallback.width, height: fallback.height };
  }

  return {
    width: skin.window.width,
    height: skin.window.height,
  };
}

export function clampMiniPlayerScale(value: number): number {
  if (!Number.isFinite(value)) return MINI_PLAYER_SCALE_BOUNDS.default;
  const stepped = Math.round(value / MINI_PLAYER_SCALE_BOUNDS.step) * MINI_PLAYER_SCALE_BOUNDS.step;
  return Math.min(MINI_PLAYER_SCALE_BOUNDS.max, Math.max(MINI_PLAYER_SCALE_BOUNDS.min, Number(stepped.toFixed(2))));
}

export function resolveMiniPlayerWindowSize(skin: MiniPlayerSkin, scale: number = MINI_PLAYER_SCALE_BOUNDS.default): MiniPlayerWindowSize {
  const base = resolveBaseSkinDimensions(skin);
  const clampedScale = clampMiniPlayerScale(scale);
  const boundsScale = Math.min(
    MINI_PLAYER_WINDOW_BOUNDS.maxWidth / base.width,
    MINI_PLAYER_WINDOW_BOUNDS.maxHeight / base.height,
  );
  const proportionalScale = Math.min(clampedScale, boundsScale);
  const scaledWidth = base.width * proportionalScale;
  const scaledHeight = base.height * proportionalScale;

  if (scaledWidth < MINI_PLAYER_WINDOW_BOUNDS.minWidth || scaledHeight < MINI_PLAYER_WINDOW_BOUNDS.minHeight) {
    return {
      width: Math.max(MINI_PLAYER_WINDOW_BOUNDS.minWidth, Math.round(scaledWidth)),
      height: Math.max(MINI_PLAYER_WINDOW_BOUNDS.minHeight, Math.round(scaledHeight)),
    };
  }

  const height = Math.round(scaledHeight);

  return {
    width: Math.round(height * (base.width / base.height)),
    height,
  };
}

export function resolveMiniPlayerSkinScale(skin: MiniPlayerSkin, scale: number = MINI_PLAYER_SCALE_BOUNDS.default): number {
  return resolveMiniPlayerWindowSize(skin, scale).width / resolveBaseSkinDimensions(skin).width;
}

function readPersistedSkin(): MiniPlayerSkinId {
  const raw = getMigratedItem(MINI_PLAYER_SKIN_SUFFIX);
  return resolveMiniPlayerSkin(raw).id as MiniPlayerSkinId;
}

function readPersistedScale(): number {
  const raw = getMigratedItem(MINI_PLAYER_SCALE_SUFFIX);
  return raw === null ? MINI_PLAYER_SCALE_BOUNDS.default : clampMiniPlayerScale(Number(raw));
}

export const selectedMiniPlayerSkinId = writable<MiniPlayerSkinId>(readPersistedSkin());
export const miniPlayerScale = writable<number>(readPersistedScale());

selectedMiniPlayerSkinId.subscribe((id) => {
  setMigratedItem(MINI_PLAYER_SKIN_SUFFIX, id);
});

miniPlayerScale.subscribe((scale) => {
  setMigratedItem(MINI_PLAYER_SCALE_SUFFIX, String(clampMiniPlayerScale(scale)));
});

export function activateMiniPlayerSkin(id: string): void {
  selectedMiniPlayerSkinId.set(resolveMiniPlayerSkin(id).id as MiniPlayerSkinId);
}

export function setMiniPlayerScale(scale: number): void {
  miniPlayerScale.set(clampMiniPlayerScale(scale));
}
