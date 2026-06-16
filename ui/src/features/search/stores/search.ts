/**
 * Search store stub.
 * Will be implemented during search feature development.
 */
import { writable } from 'svelte/store';

export const searchQuery = writable('');
export const searchResults = writable([]);