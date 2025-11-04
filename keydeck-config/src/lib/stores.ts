import { writable } from 'svelte/store';

export const hasUnsavedChanges = writable<boolean>(false);
export const saveConfigCallback = writable<(() => Promise<void>) | null>(null);

// Event store for triggering icon list refresh across all components
export const iconRefreshTrigger = writable<number>(0);
