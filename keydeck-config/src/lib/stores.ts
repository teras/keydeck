// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

import { writable } from 'svelte/store';

export const hasUnsavedChanges = writable<boolean>(false);
export const saveConfigCallback = writable<(() => Promise<void>) | null>(null);

// Event store for triggering icon list refresh across all components
export const iconRefreshTrigger = writable<number>(0);
