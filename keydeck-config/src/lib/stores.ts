import { writable } from 'svelte/store';

export const hasUnsavedChanges = writable<boolean>(false);
export const saveConfigCallback = writable<(() => Promise<void>) | null>(null);
