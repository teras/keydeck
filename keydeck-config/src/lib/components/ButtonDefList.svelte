<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog';

  interface Props {
    config: any;
    currentButtonDef: string | null;
    onButtonDefSelected: (buttonName: string | null) => void;
  }

  let { config, currentButtonDef, onButtonDefSelected }: Props = $props();

  let buttonDefs = $derived(Object.keys(config.buttons || {}));
  let showAddButton = $state(false);
  let newButtonName = $state("");
  let showButtonMenu = $state<string | null>(null);
  let buttonNameInput = $state<HTMLInputElement | undefined>();
  let renameButtonName = $state("");
  let renamingButton = $state<string | null>(null);

  function toggleAddButton() {
    showAddButton = !showAddButton;
    if (showAddButton) {
      setTimeout(() => buttonNameInput?.focus(), 0);
    }
  }

  // Click-outside handler for menu
  $effect(() => {
    if (showButtonMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.button-menu') && !target.closest('.button-menu-btn')) {
          showButtonMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  function addButton() {
    if (!newButtonName.trim()) return;

    if (!config.buttons) {
      config.buttons = {};
    }

    const buttonName = newButtonName.trim();
    if (config.buttons[buttonName]) {
      alert(`Button definition "${buttonName}" already exists!`);
      return;
    }

    config.buttons[buttonName] = {
      actions: []
    };
    newButtonName = "";
    showAddButton = false;
    // Select the newly added button definition
    if (onButtonDefSelected) {
      onButtonDefSelected(buttonName);
    }
  }

  async function deleteButton(buttonName: string) {
    showButtonMenu = null;

    const confirmed = await ask(`Delete button definition "${buttonName}"?`, {
      title: 'Confirm Delete',
      kind: 'warning'
    });

    if (confirmed) {
      delete config.buttons[buttonName];
      if (currentButtonDef === buttonName) {
        onButtonDefSelected(null);
      }
    }
  }

  function duplicateButton(buttonName: string) {
    const original = config.buttons[buttonName];
    let newName = `${buttonName}_copy`;
    let counter = 1;
    while (config.buttons[newName]) {
      newName = `${buttonName}_copy${counter}`;
      counter++;
    }

    config.buttons[newName] = JSON.parse(JSON.stringify(original));
    onButtonDefSelected(newName);
    showButtonMenu = null;
  }

  function startRename(buttonName: string) {
    renamingButton = buttonName;
    renameButtonName = buttonName;
    showButtonMenu = null;
  }

  function renameButton(oldName: string) {
    if (!renameButtonName.trim() || renameButtonName === oldName) {
      renameButtonName = "";
      renamingButton = null;
      return;
    }

    if (config.buttons[renameButtonName]) {
      alert(`Button definition "${renameButtonName}" already exists!`);
      renameButtonName = "";
      renamingButton = null;
      return;
    }

    // Rebuild object preserving order
    const newButtons: any = {};
    for (const key of Object.keys(config.buttons)) {
      if (key === oldName) {
        newButtons[renameButtonName] = config.buttons[oldName];
      } else {
        newButtons[key] = config.buttons[key];
      }
    }
    config.buttons = newButtons;

    if (currentButtonDef === oldName) {
      onButtonDefSelected(renameButtonName);
    }

    renameButtonName = "";
    renamingButton = null;
  }
</script>

<div class="button-list">
  <div class="header">
    <h3>Button Definitions</h3>
    <button class="add-btn" onclick={toggleAddButton}>+</button>
  </div>

  {#if showAddButton}
    <div class="add-button">
      <input
        type="text"
        bind:this={buttonNameInput}
        bind:value={newButtonName}
        placeholder="Button name"
        onkeydown={(e) => e.key === 'Enter' && addButton()}
      />
      <button onclick={addButton} title="Add">✓</button>
      <button onclick={() => showAddButton = false} title="Cancel">✕</button>
    </div>
  {/if}

  <div class="separator"></div>

  <div class="buttons">
    {#each buttonDefs as button}
      <div class="button-row">
        {#if renamingButton === button}
          <input
            type="text"
            bind:value={renameButtonName}
            class="rename-input"
            onkeydown={(e) => {
              if (e.key === 'Enter') renameButton(button);
              if (e.key === 'Escape') { renameButtonName = ""; renamingButton = null; }
            }}
            onblur={() => renameButton(button)}
            onmousedown={(e) => e.stopPropagation()}
            autofocus
          />
        {:else}
          <button
            class="button-item"
            class:active={button === currentButtonDef}
            onclick={() => onButtonDefSelected(button)}
          >
            {button}
          </button>
          <button
            class="button-menu-btn"
            onclick={(e) => {
              e.stopPropagation();
              showButtonMenu = showButtonMenu === button ? null : button;
            }}
          >
            ⋮
          </button>
        {/if}

        {#if showButtonMenu === button}
          <div class="button-menu">
            <button onclick={(e) => { e.stopPropagation(); startRename(button); }}>✏️ Rename</button>
            <button onclick={(e) => { e.stopPropagation(); duplicateButton(button); }}>📋 Duplicate</button>
            <button class="danger" onclick={(e) => { e.stopPropagation(); deleteButton(button); }}>🗑️ Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .button-list {
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 12px;
  }

  h3 {
    margin: 0;
    font-size: 16px;
    color: #cccccc;
  }

  .add-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .add-btn:hover {
    background-color: #1177bb;
  }

  .add-button {
    display: flex;
    gap: 4px;
    margin-top: 12px;
    margin-bottom: 12px;
  }

  .separator {
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 16px;
  }

  .add-button input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .add-button button {
    padding: 6px 12px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-button button:first-of-type {
    background-color: #2d7d46;
  }

  .add-button button:first-of-type:hover {
    background-color: #3a9d5a;
  }

  .add-button button:last-child {
    background-color: #7a2d2d;
  }

  .add-button button:last-child:hover {
    background-color: #9a3d3d;
  }

  .buttons {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .button-row {
    position: relative;
    display: flex;
    gap: 4px;
  }

  .rename-input {
    flex: 1;
    padding: 8px 12px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #0e639c;
    border-radius: 4px;
    font-size: 13px;
  }

  .button-item {
    flex: 1;
    padding: 8px 12px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
  }

  .button-item:hover {
    background-color: #4a4a4a;
  }

  .button-item.active {
    background-color: #354a5f;
    border-color: #5b9bd5;
  }

  .button-menu-btn {
    width: 28px;
    padding: 4px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
  }

  .button-menu-btn:hover {
    background-color: #4a4a4a;
    color: #cccccc;
  }

  .button-menu {
    position: absolute;
    right: 0;
    top: 100%;
    margin-top: 2px;
    background-color: #2d2d30;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.4);
    z-index: 10;
    display: flex;
    flex-direction: column;
    min-width: 120px;
  }

  .button-menu button {
    padding: 8px 12px;
    background: none;
    color: #cccccc;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }

  .button-menu button:hover {
    background-color: #3c3c3c;
  }
</style>
