/**
 * Player store stub.
 * Will be implemented during player feature development.
 */
import { writable } from 'svelte/store';
import type { FrequencyData } from '@shared/types/models';

export const currentTrack = writable(null);
export const isPlaying = writable(false);
export const progress = writable(0);
export const queue = writable([]);

/** Latest frequency data from the Rust FFT engine (null until first event). */
export const frequencyData = writable<FrequencyData | null>(null);

/** Whether Modo Cine (immersive fullscreen visualizer) is active. */
export const modoCineActive = writable<boolean>(false);