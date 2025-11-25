<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import ActionEditor from './ActionEditor.svelte';
  import TemplateSelector from './TemplateSelector.svelte';
  import TriStateCheckbox from './TriStateCheckbox.svelte';

  interface Props {
    config: any;
    pageName: string;
    deviceSerial: string;
  }

  let { config, pageName, deviceSerial }: Props = $props();

  let pageGroup = $derived(config.page_groups?.[deviceSerial] || config.page_groups?.default);
  let page = $derived(pageGroup?.[pageName]);
  let groupKey = $derived(config.page_groups?.[deviceSerial] ? deviceSerial : 'default');
  let openActionIndex = $state<number>(-1);

  function updateWindowName(value: string) {
    if (!page) return;
    if (value.trim()) {
      page.window_name = value;
      config[groupKey][pageName].window_name = value;
    } else {
      delete page.window_name;
      delete config[groupKey][pageName].window_name;
    }
  }

  function updateInherits(templates: string[]) {
    if (!page) return;
    if (templates.length > 0) {
      page.inherits = templates;
      config[groupKey][pageName].inherits = templates;
    } else {
      delete page.inherits;
      delete config[groupKey][pageName].inherits;
    }
  }

  function getSelectedTemplates(): string[] {
    if (!page?.inherits) return [];
    if (Array.isArray(page.inherits)) return page.inherits;
    return [page.inherits];
  }

  function handleLockChange(newValue: boolean | undefined) {
    if (!page) return;
    if (newValue === undefined) {
      delete page.lock;
      delete config[groupKey][pageName].lock;
    } else {
      page.lock = newValue;
      config[groupKey][pageName].lock = newValue;
    }
  }

  function addOnTickAction() {
    if (!page) return;
    if (!page.on_tick) {
      page.on_tick = [];
    }
    page.on_tick = [...page.on_tick, { refresh: 'dynamic' }];
    openActionIndex = page.on_tick.length - 1;
    config[groupKey][pageName].on_tick = page.on_tick;
  }

  function updateOnTickAction(index: number, newAction: any) {
    if (!page?.on_tick) return;
    page.on_tick[index] = newAction;
    config[groupKey][pageName].on_tick = page.on_tick;
  }

  function removeOnTickAction(index: number) {
    if (!page?.on_tick) return;
    page.on_tick.splice(index, 1);
    config[groupKey][pageName].on_tick = page.on_tick;
  }
  let windowClasses = $state<string[]>([]);
  let windowListLoading = $state(false);
  let windowListError = $state<string | null>(null);
  let lastWindowRefresh = $state<Date | null>(null);
  let windowDropdownOpen = $state(false);
  let windowNameInput: HTMLInputElement | null = null;

  async function refreshWindowClasses() {
    windowListLoading = true;
    windowListError = null;
    try {
      const result = await invoke<string[]>("list_window_classes");
      windowClasses = result || [];
      lastWindowRefresh = new Date();
    } catch (err) {
      if (err instanceof Error) {
        windowListError = err.message;
      } else if (typeof err === "string") {
        windowListError = err;
      } else {
        windowListError = "Failed to load window classes";
      }
      windowClasses = [];
    } finally {
      windowListLoading = false;
    }
  }

  async function toggleWindowDropdown() {
    if (windowDropdownOpen) {
      windowDropdownOpen = false;
      return;
    }

    windowDropdownOpen = true;
    await refreshWindowClasses();
  }

  function selectWindowClass(value: string) {
    if (windowNameInput) {
      windowNameInput.value = value;
    }
    updateWindowName(value);
    windowDropdownOpen = false;
  }

  $effect(() => {
    if (!windowDropdownOpen) return;

    const handleClick = (event: MouseEvent) => {
      const target = event.target as HTMLElement;
      if (!target.closest('.window-input-container')) {
        windowDropdownOpen = false;
      }
    };

    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  });
</script>

<div class="page-editor">
  <h3>Page: {pageName}</h3>

  <div class="section">
    <div class="form-group">
      <label>Window Name</label>
      <div class="window-input-container">
        <div class="window-input-row">
          <input
            type="text"
            bind:this={windowNameInput}
            value={page?.window_name || ""}
            oninput={(e) => updateWindowName(e.currentTarget.value)}
            placeholder="window class (or title substring)"
          />
          <button
            type="button"
            class="dropdown-window-btn"
            title={windowDropdownOpen ? "Hide window list" : "Show window list"}
            onclick={toggleWindowDropdown}
            aria-expanded={windowDropdownOpen}
          >
            ▾
          </button>
        </div>
        {#if windowDropdownOpen}
          <div class="window-dropdown">
            {#if windowListLoading}
              <p class="dropdown-help">Loading windows…</p>
            {:else if windowListError}
              <div class="dropdown-error">
                <p>Window list unavailable: {windowListError}</p>
                <button type="button" onclick={refreshWindowClasses}>Retry</button>
              </div>
            {:else if windowClasses.length === 0}
              <p class="dropdown-help">No matching windows detected.</p>
            {:else}
              <div class="window-options">
                {#each windowClasses as className}
                  <button type="button" class="window-option" onclick={() => selectWindowClass(className)}>
                    {className}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
      <p class="help">
        Page will be activated when a window matching this name is focused.
        {#if lastWindowRefresh}
          Last updated {lastWindowRefresh.toLocaleTimeString()}.
        {/if}
      </p>
    </div>

    <div class="form-group">
      <label>Inherits Templates</label>
      <TemplateSelector
        {config}
        selectedTemplates={getSelectedTemplates()}
        onUpdate={updateInherits}
      />
      <p class="help">Select one or more templates to inherit</p>
    </div>

    <div class="form-group">
      <TriStateCheckbox
        value={page?.lock}
        label="Lock Page"
        onToggle={handleLockChange}
        inheritLabel="Inherit from template"
        trueLabel="Locked"
        falseLabel="Unlocked"
      />
      <p class="help">
        {#if page?.lock === undefined}
          Will inherit lock state from template (if any)
        {:else if page?.lock === true}
          Prevents automatic page switching when window focus changes
        {:else}
          Allows automatic page switching when window focus changes
        {/if}
      </p>
    </div>
  </div>

  <div class="section">
    <div class="actions-header">
      <h4>On Tick Actions</h4>
      <button class="add-btn" onclick={addOnTickAction}>+</button>
    </div>
    <p class="help">Actions executed periodically based on global tick_time</p>
    <div class="actions-list">
      {#if page?.on_tick && page.on_tick.length > 0}
        {#each page.on_tick as action, i}
          <ActionEditor
            {action}
            index={i}
            {config}
            deviceSerial={deviceSerial}
            initiallyOpen={i === openActionIndex}
            onToggle={() => openActionIndex = i}
            onUpdate={(newAction) => updateOnTickAction(i, newAction)}
            onDelete={() => removeOnTickAction(i)}
          />
        {/each}
      {:else}
        <p class="empty">No on_tick actions configured</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .page-editor {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  h3 {
    margin: 0;
    font-size: 16px;
    color: #cccccc;
    padding-bottom: 12px;
    border-bottom: 1px solid #3e3e42;
  }

  h4 {
    margin: 0;
    font-size: 13px;
    color: #aaa;
  }

  .actions-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .add-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 15px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .add-btn:hover {
    background-color: #1177bb;
  }

  .section {
    padding-top: 12px;
    border-top: 1px solid #3e3e42;
  }

  .section:first-of-type {
    padding-top: 0;
    border-top: none;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 16px;
  }

  label {
    font-size: 12px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
  }

  input {
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
    min-height: 34px;
  }

  input:focus {
    outline: none;
    border-color: #0e639c;
  }

  .help {
    margin: 0;
    font-size: 11px;
    color: #666;
    font-style: italic;
  }

  .actions-list {
    display: flex;
    flex-direction: column;
    margin-bottom: 12px;
  }

  .empty {
    color: #666;
    font-size: 12px;
    font-style: italic;
    margin: 8px 0;
  }

  button {
    padding: 8px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  button:hover {
    background-color: #1177bb;
  }

  .window-input-container {
    position: relative;
    width: 100%;
  }

  .window-input-row {
    display: flex;
    align-items: center;
    width: 100%;
    height: 34px;
    background-color: #3c3c3c;
    border: 1px solid #555;
    border-radius: 4px;
    overflow: hidden;
  }

  .window-input-row:focus-within {
    border-color: #0e639c;
  }

  .window-input-row input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    padding: 0 10px;
    height: 100%;
    color: #cccccc;
  }

  .window-input-row input:focus {
    outline: none;
  }

  .dropdown-window-btn {
    flex: 0 0 32px;
    padding: 0;
    margin: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    border: none;
    border-left: 1px solid #4a4a4a;
    background-color: #3c3c3c;
    color: #bbbbbb;
    font-size: 12px;
    cursor: pointer;
  }

  .dropdown-window-btn:hover {
    background-color: #4a4a4a;
  }

  .window-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    border: 1px solid #555;
    border-radius: 4px;
    background-color: #2b2b2b;
    max-height: 220px;
    overflow: hidden;
    z-index: 20;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45);
  }

  .window-options {
    max-height: 180px;
    overflow-y: auto;
  }

  .window-option {
    width: 100%;
    padding: 6px 10px;
    background: none;
    color: #ccc;
    border: none;
    text-align: left;
    border-bottom: 1px solid #333;
    font-size: 12px;
  }

  .window-option:last-child {
    border-bottom: none;
  }

  .window-option:hover {
    background-color: #2b2b2b;
  }

  .dropdown-help,
  .dropdown-error {
    margin: 0;
    padding: 10px;
    font-size: 12px;
    color: #bbb;
  }

  .dropdown-error {
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: #ff9c9c;
  }
</style>
