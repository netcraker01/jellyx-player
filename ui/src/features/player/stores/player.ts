/**
 * Player store stub.
 * Will be implemented during player feature development.
 */
import { writable } from 'svelte/store';

export const currentTrack = writable(null);
export const isPlaying = writable(false);
export const progress = writable(0);
export const queue = writable([]);