<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog';
  import ColorPicker from './ColorPicker.svelte';
  import CleanUpIcons from './CleanUpIcons.svelte';

  interface Props {
    config: any;
  }

  let { config }: Props = $props();

  function updateTickTime(value: string) {
    const num = parseFloat(value);
    if (num >= 1 && num <= 60) {
      config.tick_time = num;
    }
  }

  // Color management
  let newColorName = $state("");
  let showAddColor = $state(false);
  let showColorMenu = $state<string | null>(null);
  let renamingColor = $state<string | null>(null);
  let renameColorName = $state("");
  let lastAddedColor = $state<string | null>(null);
  let colorNameInput = $state<HTMLInputElement | undefined>();

  // Protected icons management
  let showAddProtectedIcon = $state(false);
  let newProtectedIcon = $state("");

  function toggleAddColor() {
    showAddColor = !showAddColor;
    if (showAddColor) {
      setTimeout(() => colorNameInput?.focus(), 0);
    }
  }

  function addColor() {
    if (!newColorName.trim()) return;

    if (!config.colors) {
      config.colors = {};
    }

    const colorName = newColorName.trim();
    if (config.colors[colorName]) {
      alert(`Color "${colorName}" already exists!`);
      return;
    }

    config.colors[colorName] = "0x888888";
    lastAddedColor = colorName;
    newColorName = "";
    showAddColor = false;

    setTimeout(() => {
      const colorInput = document.querySelector(`input[data-color-name="${colorName}"]`) as HTMLInputElement;
      if (colorInput) {
        colorInput.focus();
        colorInput.setSelectionRange(colorInput.value.length, colorInput.value.length);
      }
      lastAddedColor = null;
    }, 50);
  }

  function updateColorFromText(name: string, value: string) {
    if (config.colors) {
      config.colors[name] = value;
    }
  }

  // Click-outside handler for menu
  $effect(() => {
    if (showColorMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.color-menu') && !target.closest('.color-menu-btn')) {
          showColorMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  async function deleteColor(name: string) {
    showColorMenu = null;

    const confirmed = await ask(`Delete color "${name}"?`, {
      title: 'Confirm Delete',
      kind: 'warning'
    });

    if (confirmed && config.colors) {
      delete config.colors[name];
      config.colors = config.colors;
    }
  }

  function duplicateColor(name: string) {
    if (!config.colors) return;

    const original = config.colors[name];
    let newName = `${name}_copy`;
    let counter = 1;
    while (config.colors[newName]) {
      newName = `${name}_copy${counter}`;
      counter++;
    }

    config.colors[newName] = original;
    config.colors = config.colors;
    lastAddedColor = newName;
    showColorMenu = null;

    // Focus on the duplicated color's text input
    setTimeout(() => {
      const colorInput = document.querySelector(`input[data-color-name="${newName}"]`) as HTMLInputElement;
      if (colorInput) {
        colorInput.focus();
        colorInput.setSelectionRange(colorInput.value.length, colorInput.value.length);
      }
      lastAddedColor = null;
    }, 50);
  }

  function startRenameColor(name: string) {
    renamingColor = name;
    renameColorName = name;
    showColorMenu = null;
  }

  function renameColor(oldName: string) {
    if (!renameColorName.trim() || renameColorName === oldName) {
      renameColorName = "";
      renamingColor = null;
      return;
    }

    if (config.colors && config.colors[renameColorName]) {
      alert(`Color "${renameColorName}" already exists!`);
      renameColorName = "";
      renamingColor = null;
      return;
    }

    if (config.colors) {
      const newColors: any = {};
      for (const key of Object.keys(config.colors)) {
        if (key === oldName) {
          newColors[renameColorName] = config.colors[oldName];
        } else {
          newColors[key] = config.colors[key];
        }
      }
      config.colors = newColors;
    }

    renameColorName = "";
    renamingColor = null;
  }

  function toggleAddProtectedIcon() {
    showAddProtectedIcon = !showAddProtectedIcon;
    if (showAddProtectedIcon) {
      setTimeout(() => {
        const input = document.querySelector('.protected-icon-input') as HTMLInputElement;
        input?.focus();
      }, 50);
    } else {
      newProtectedIcon = "";
    }
  }

  function addProtectedIcon() {
    const pattern = newProtectedIcon.trim();
    if (!pattern) {
      showAddProtectedIcon = false;
      return;
    }

    if (!config.protected_icons) {
      config.protected_icons = [];
    }

    config.protected_icons.push(pattern);
    config.protected_icons = config.protected_icons;
    newProtectedIcon = "";
    showAddProtectedIcon = false;
  }

  function removeProtectedIcon(index: number) {
    if (config.protected_icons) {
      config.protected_icons = config.protected_icons.filter((_: any, i: number) => i !== index);
    }
  }
</script>

<div class="global-settings">
  <div class="header">
    <h3>Global Configuration</h3>
  </div>

  <div class="separator"></div>

  <div class="settings-content">
    <div class="form-group">
      <label>Tick Time (seconds)</label>
      <input
        type="number"
        min="1"
        max="60"
        step="0.1"
        value={config.tick_time || 2.0}
        oninput={(e) => updateTickTime(e.currentTarget.value)}
      />
      <p class="help">Global tick interval (1-60 seconds)</p>
    </div>

    <div class="section">
    <div class="color-header">
      <h4>Colors</h4>
      <button class="add-btn" onclick={toggleAddColor}>+</button>
    </div>

    {#if showAddColor}
      <div class="add-color">
        <input
          type="text"
          bind:this={colorNameInput}
          bind:value={newColorName}
          placeholder="Color name"
          onkeydown={(e) => e.key === 'Enter' && addColor()}
        />
        <button onclick={addColor} title="Add">✓</button>
        <button onclick={() => showAddColor = false} title="Cancel">✕</button>
      </div>
    {/if}

    {#if config.colors && Object.keys(config.colors).length > 0}
      <div class="color-list">
        {#each Object.entries(config.colors) as [name, color]}
          <div class="color-row">
            <div class="color-item">
              <div class="color-info">
                {#if renamingColor === name}
                  <input
                    type="text"
                    bind:value={renameColorName}
                    class="rename-color-input"
                    onkeydown={(e) => {
                      if (e.key === 'Enter') renameColor(name);
                      if (e.key === 'Escape') { renameColorName = ""; renamingColor = null; }
                    }}
                    onblur={() => renameColor(name)}
                    onmousedown={(e) => e.stopPropagation()}
                    autofocus
                  />
                {:else}
                  <span class="color-name">{name}</span>
                {/if}
                <ColorPicker
                  value={color}
                  onUpdate={(value) => updateColorFromText(name, value)}
                  dataColorName={name}
                />
              </div>
            </div>
            {#if renamingColor !== name}
              <button
                class="color-menu-btn"
                onclick={(e) => {
                  e.stopPropagation();
                  showColorMenu = showColorMenu === name ? null : name;
                }}
              >
                ⋮
              </button>
            {/if}

            {#if showColorMenu === name}
              <div class="color-menu">
                <button onclick={(e) => { e.stopPropagation(); startRenameColor(name); }}>✏️ Rename</button>
                <button onclick={(e) => { e.stopPropagation(); duplicateColor(name); }}>📋 Duplicate</button>
                <button class="danger" onclick={(e) => { e.stopPropagation(); deleteColor(name); }}>🗑️ Delete</button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty">No colors defined</p>
    {/if}
    </div>

    <div class="section">
      <h4>Icon Management</h4>
      <div class="icon-cleanup-section">
        <p class="help">Remove unused icon files from the icons directory to free up space.</p>
        <CleanUpIcons {config} />
      </div>

      <div class="subsection">
        <div class="subsection-header">
          <span>Protected Icon Patterns</span>
          <button class="add-btn" onclick={toggleAddProtectedIcon}>+</button>
        </div>
        <p class="help">Glob patterns for icons protected from cleanup (e.g., *.dynamic.png)</p>

        {#if showAddProtectedIcon}
          <div class="add-input-row">
            <input
              type="text"
              class="protected-icon-input"
              bind:value={newProtectedIcon}
              placeholder="e.g., *.dynamic.png"
              onkeydown={(e) => e.key === 'Enter' && addProtectedIcon()}
            />
            <button onclick={addProtectedIcon} title="Add">✓</button>
            <button onclick={() => showAddProtectedIcon = false} title="Cancel">✕</button>
          </div>
        {/if}

        {#if config.protected_icons && config.protected_icons.length > 0}
          <div class="pattern-list">
            {#each config.protected_icons as pattern, i}
              <div class="pattern-item">
                <input
                  type="text"
                  bind:value={config.protected_icons[i]}
                  placeholder="e.g., *.dynamic.png"
                />
                <button class="remove-btn" onclick={() => removeProtectedIcon(i)} title="Remove">✕</button>
              </div>
            {/each}
          </div>
        {:else}
          <p class="empty">No protected patterns defined</p>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .global-settings {
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

  .separator {
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 16px;
  }

  .settings-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  h4 {
    margin: 0 0 8px 0;
    font-size: 13px;
    color: #aaa;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 12px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
  }

  input {
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
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

  .section {
    padding-top: 12px;
    border-top: 1px solid #3e3e42;
  }

  .color-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
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

  .add-color {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
  }

  .add-color input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .add-color button {
    padding: 6px 12px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-color button:first-of-type {
    background-color: #2d7d46;
  }

  .add-color button:first-of-type:hover {
    background-color: #3a9d5a;
  }

  .add-color button:last-child {
    background-color: #7a2d2d;
  }

  .add-color button:last-child:hover {
    background-color: #9a3d3d;
  }

  .color-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .color-row {
    position: relative;
    display: flex;
    gap: 4px;
  }

  .color-item {
    flex: 1;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    padding: 8px;
    background-color: #3c3c3c;
    border: 1px solid #555;
    border-radius: 4px;
    gap: 8px;
  }

  .color-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .color-name {
    font-size: 12px;
    font-weight: 600;
    color: #cccccc;
  }

  .rename-color-input {
    font-size: 12px;
    font-weight: 600;
    padding: 4px 6px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #0e639c;
    border-radius: 4px;
  }

  .rename-color-input:focus {
    outline: none;
    border-color: #1177bb;
  }

  .color-menu-btn {
    width: 28px;
    padding: 4px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
  }

  .color-menu-btn:hover {
    background-color: #4a4a4a;
    color: #cccccc;
  }

  .color-menu {
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

  .color-menu button {
    padding: 8px 12px;
    background: none;
    color: #cccccc;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }

  .color-menu button:hover {
    background-color: #3c3c3c;
  }

  .color-menu button.danger {
    color: #f48771;
  }

  .color-menu button.danger:hover {
    background-color: #5a1d1d;
  }

  .empty {
    color: #666;
    font-size: 12px;
    font-style: italic;
    margin: 8px 0;
  }

  .icon-cleanup-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 16px;
  }

  .subsection {
    margin-top: 8px;
  }

  .subsection-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .subsection-header span {
    font-size: 13px;
    color: #aaa;
    font-weight: 500;
  }

  .add-input-row {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
  }

  .add-input-row input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
    font-family: 'Courier New', monospace;
  }

  .add-input-row input:focus {
    outline: none;
    border-color: #0e639c;
  }

  .add-input-row button {
    padding: 6px 12px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-input-row button:first-of-type {
    background-color: #2d7d46;
  }

  .add-input-row button:first-of-type:hover {
    background-color: #3a9d5a;
  }

  .add-input-row button:last-child {
    background-color: #7a2d2d;
  }

  .add-input-row button:last-child:hover {
    background-color: #9a3d3d;
  }

  .pattern-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .pattern-item {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .pattern-item input {
    flex: 1;
    padding: 6px 8px;
    background: #2a2a2a;
    border: 1px solid #3e3e42;
    border-radius: 4px;
    color: #ccc;
    font-size: 12px;
    font-family: 'Courier New', monospace;
  }

  .pattern-item input:focus {
    outline: none;
    border-color: #007acc;
  }

  .pattern-item .remove-btn {
    padding: 4px 8px;
    background: #3e3e42;
    color: #aaa;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.2s;
  }

  .pattern-item .remove-btn:hover {
    background: #dc3545;
    color: white;
  }
</style>
