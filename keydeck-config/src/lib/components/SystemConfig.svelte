<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    config: any;
    selectedDevice?: any;
  }

  let { config, selectedDevice }: Props = $props();
  let imageDirExists = $state<boolean>(true);

  // Check if directory exists whenever image_dir changes
  $effect(() => {
    if (config.image_dir) {
      checkImageDirExists(config.image_dir);
    } else {
      imageDirExists = true; // No warning if not set
    }
  });

  async function checkImageDirExists(path: string) {
    try {
      const exists = await invoke<boolean>("check_directory_exists", { path });
      imageDirExists = exists;
    } catch (e) {
      console.error("Failed to check directory:", e);
      imageDirExists = false;
    }
  }

  function updateImageDir(value: string) {
    config.image_dir = value || undefined;
  }

  async function browseImageDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: config.image_dir || undefined
      });

      if (selected && typeof selected === 'string') {
        config.image_dir = selected;
        await checkImageDirExists(selected);
      }
    } catch (e) {
      console.error("Failed to open directory picker:", e);
    }
  }

  function updateTickTime(value: string) {
    const num = parseFloat(value);
    if (num >= 1 && num <= 60) {
      config.tick_time = num;
    }
  }

  function getDevicePageGroup() {
    if (!selectedDevice || !config.page_groups) return null;
    return config.page_groups[selectedDevice.serial] || config.page_groups.default;
  }

  function updateMainPage(value: string) {
    const pageGroup = getDevicePageGroup();
    if (!pageGroup) return;

    if (value.trim()) {
      pageGroup.main_page = value.trim();
    } else {
      delete pageGroup.main_page;
    }
  }

  function updateRestoreMode(value: string) {
    const pageGroup = getDevicePageGroup();
    if (!pageGroup) return;

    pageGroup.restore_mode = value;
  }

  function getAvailablePages(): string[] {
    const pageGroup = getDevicePageGroup();
    if (!pageGroup) return [];

    const knownFields = ['main_page', 'restore_mode', 'on_tick'];
    return Object.keys(pageGroup).filter(key => !knownFields.includes(key));
  }

  // Color management
  let newColorName = $state("");
  let newColorValue = $state("#000000");
  let isAddingColor = $state(false);

  function startAddingColor() {
    newColorName = "";
    newColorValue = "#000000";
    isAddingColor = true;
  }

  function addColor() {
    if (!newColorName.trim()) return;

    if (!config.colors) {
      config.colors = {};
    }

    config.colors[newColorName.trim()] = newColorValue;
    isAddingColor = false;
    newColorName = "";
    newColorValue = "#000000";
  }

  function updateColor(name: string, value: string) {
    if (config.colors) {
      config.colors[name] = value;
    }
  }

  function deleteColor(name: string) {
    if (config.colors) {
      delete config.colors[name];
      config.colors = config.colors; // Trigger reactivity
    }
  }

  function cancelAddingColor() {
    isAddingColor = false;
    newColorName = "";
    newColorValue = "#000000";
  }
</script>

<div class="system-config">
  <h3>System Configuration</h3>

  <!-- Global Settings -->
  <div class="section-header">Global Settings</div>

  <div class="form-group">
    <label>Image Directory</label>
    <div class="input-with-button">
      <input
        type="text"
        value={config.image_dir || ""}
        oninput={(e) => updateImageDir(e.currentTarget.value)}
        placeholder="/path/to/images"
        class:warning={!imageDirExists}
      />
      <button onclick={browseImageDir} class="browse-button" title="Browse for folder">
        📁
      </button>
      {#if !imageDirExists}
        <span class="warning-icon" title="Directory does not exist">⚠️</span>
      {/if}
    </div>
    <p class="help">Directory where button icons are stored</p>
  </div>

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
    <h4>Colors</h4>
    <p class="help">Define named colors for reuse (#RRGGBB format)</p>

    {#if config.colors && Object.keys(config.colors).length > 0}
      <div class="color-list">
        {#each Object.entries(config.colors) as [name, color]}
          <div class="color-item">
            <div class="color-info">
              <span class="color-name">{name}</span>
              <div class="color-value-container">
                <input
                  type="text"
                  value={color}
                  oninput={(e) => updateColor(name, e.currentTarget.value)}
                  class="color-text-input"
                  placeholder="#RRGGBB"
                />
                <input
                  type="color"
                  value={color}
                  oninput={(e) => updateColor(name, e.currentTarget.value)}
                  class="color-picker-input"
                  title="Pick color"
                />
              </div>
            </div>
            <button onclick={() => deleteColor(name)} class="delete-button" title="Delete color">
              ✕
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty">No colors defined</p>
    {/if}

    {#if isAddingColor}
      <div class="add-color-form">
        <input
          type="text"
          value={newColorName}
          oninput={(e) => newColorName = e.currentTarget.value}
          placeholder="Color name"
          class="color-name-input"
        />
        <div class="color-value-container">
          <input
            type="text"
            value={newColorValue}
            oninput={(e) => newColorValue = e.currentTarget.value}
            class="color-text-input"
            placeholder="#RRGGBB"
          />
          <input
            type="color"
            value={newColorValue}
            oninput={(e) => newColorValue = e.currentTarget.value}
            class="color-picker-input"
            title="Pick color"
          />
        </div>
        <div class="add-color-buttons">
          <button onclick={addColor} class="add-button">Add</button>
          <button onclick={cancelAddingColor} class="cancel-button">Cancel</button>
        </div>
      </div>
    {:else}
      <button onclick={startAddingColor} class="add-color-button">+ Add Color</button>
    {/if}
  </div>

  <!-- Device Settings (shown when device is selected) -->
  {#if selectedDevice}
    {@const pageGroup = getDevicePageGroup()}
    {@const availablePages = getAvailablePages()}

    <div class="section">
      <div class="section-header">Device Settings: {selectedDevice.model}</div>

      <div class="form-group">
        <label>Main Page</label>
        <select
          value={pageGroup?.main_page || ""}
          onchange={(e) => updateMainPage(e.currentTarget.value)}
        >
          <option value="">Auto (first page)</option>
          {#each availablePages as pageName}
            <option value={pageName}>{pageName}</option>
          {/each}
        </select>
        <p class="help">Default page to show when device starts</p>
      </div>

      <div class="form-group">
        <label>Restore Mode</label>
        <select
          value={pageGroup?.restore_mode || "main"}
          onchange={(e) => updateRestoreMode(e.currentTarget.value)}
        >
          <option value="keep">Keep - Stay on current page</option>
          <option value="last">Last - Return to last page</option>
          <option value="main">Main - Return to main page</option>
        </select>
        <p class="help">Page behavior on window focus change</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .system-config {
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
    margin: 0 0 8px 0;
    font-size: 13px;
    color: #aaa;
  }

  .section-header {
    font-size: 14px;
    font-weight: 600;
    color: #4ec9b0;
    margin: 20px 0 12px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid #3e3e42;
  }

  .section-header:first-child {
    margin-top: 0;
  }

  select {
    width: 100%;
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
  }

  select:focus {
    outline: none;
    border-color: #0e639c;
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

  input.warning {
    border-color: #f48771;
  }

  .input-with-button {
    position: relative;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .input-with-button input {
    flex: 1;
  }

  .browse-button {
    padding: 8px 12px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    min-width: 40px;
    height: 37px;
  }

  .browse-button:hover {
    background-color: #1177bb;
  }

  .warning-icon {
    position: absolute;
    right: 52px;
    font-size: 18px;
    pointer-events: none;
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

  .color-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
  }

  .color-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px;
    background-color: #3c3c3c;
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

  .color-value-container {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .color-text-input {
    flex: 1;
    padding: 6px 8px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
    font-family: monospace;
  }

  .color-text-input:focus {
    outline: none;
    border-color: #0e639c;
  }

  .color-picker-input {
    width: 40px;
    height: 32px;
    padding: 2px;
    background-color: #2a2a2a;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
  }

  .color-picker-input::-webkit-color-swatch-wrapper {
    padding: 2px;
  }

  .color-picker-input::-webkit-color-swatch {
    border: none;
    border-radius: 2px;
  }

  .delete-button {
    padding: 4px 8px;
    background-color: #5a1d1d;
    color: #f48771;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    min-width: 28px;
    height: 28px;
  }

  .delete-button:hover {
    background-color: #7a2d2d;
  }

  .add-color-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background-color: #2a2a2a;
    border-radius: 4px;
    margin-bottom: 12px;
  }

  .color-name-input {
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
  }

  .color-name-input:focus {
    outline: none;
    border-color: #0e639c;
  }

  .add-color-buttons {
    display: flex;
    gap: 8px;
  }

  .add-button {
    flex: 1;
    padding: 8px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .add-button:hover {
    background-color: #1177bb;
  }

  .cancel-button {
    flex: 1;
    padding: 8px;
    background-color: #555;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .cancel-button:hover {
    background-color: #666;
  }

  .add-color-button {
    padding: 8px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    width: 100%;
  }

  .add-color-button:hover {
    background-color: #1177bb;
  }

  .empty {
    color: #666;
    font-size: 12px;
    font-style: italic;
    margin: 8px 0;
  }
</style>
