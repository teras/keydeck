<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import ColorPicker from './ColorPicker.svelte';

  interface Props {
    config: any;
    selectedDevice: any;
  }

  let { config, selectedDevice }: Props = $props();

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

    const knownFields = ['main_page', 'restore_mode', 'press_effect'];
    return Object.keys(pageGroup).filter(key => !knownFields.includes(key));
  }

  let pageGroup = $derived(getDevicePageGroup());
  let availablePages = $derived(getAvailablePages());

  // Initialize brightness with default value if not set
  if (config.brightness === undefined) {
    config.brightness = 80;
  }

  function updateBrightness(value: number) {
    config.brightness = value;
  }

  // Press effect management
  const PRESS_EFFECT_DEFAULTS: Record<string, number> = {
    shrink: 2,
    shift: 4,
    emboss: 2,
  };

  function getPressEffect() {
    return pageGroup?.press_effect || { type: 'shift', pixels: 4 };
  }

  function updatePressEffectType(type: string) {
    if (!pageGroup) return;
    pageGroup.press_effect = {
      type,
      pixels: PRESS_EFFECT_DEFAULTS[type] || 4,
      ...(pageGroup.press_effect?.border_color ? { border_color: pageGroup.press_effect.border_color } : {}),
    };
  }

  function updatePressEffectPixels(value: number) {
    if (!pageGroup) return;
    const effect = getPressEffect();
    effect.pixels = value;
    pageGroup.press_effect = { ...effect };
  }

  function updatePressEffectBorderColor(value: string) {
    if (!pageGroup) return;
    const effect = getPressEffect();
    if (value.trim()) {
      effect.border_color = value;
    } else {
      delete effect.border_color;
    }
    pageGroup.press_effect = { ...effect };
  }

  let pressEffect = $derived(getPressEffect());
</script>

<div class="device-settings">
  <div class="header">
    <h3>{selectedDevice.model}</h3>
  </div>

  <div class="separator"></div>

  <div class="settings-content">
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

    <div class="form-group">
      <label>Brightness ({config.brightness}%)</label>
      <input
        type="range"
        min="0"
        max="100"
        value={config.brightness}
        oninput={(e) => updateBrightness(parseInt(e.currentTarget.value))}
        class="brightness-slider"
      />
      <p class="help">Global brightness level for all devices (0-100%)</p>
    </div>

    <div class="section">
      <h4>Press Effect</h4>
      <p class="help">Visual feedback when buttons are pressed (software-rendered devices only)</p>

      <div class="form-group" style="margin-top: 8px;">
        <label>Effect Type</label>
        <select
          value={pressEffect.type}
          onchange={(e) => updatePressEffectType(e.currentTarget.value)}
        >
          <option value="shift">Shift — content shifts down-right on press</option>
          <option value="shrink">Shrink — content scales down on press</option>
          <option value="emboss">Emboss — 3D bevel border effect</option>
        </select>
      </div>

      <div class="form-group">
        <label>Pixels ({pressEffect.pixels}px)</label>
        <input
          type="range"
          min="1"
          max="10"
          value={pressEffect.pixels}
          oninput={(e) => updatePressEffectPixels(parseInt(e.currentTarget.value))}
          class="pixels-slider"
        />
        <p class="help">
          {#if pressEffect.type === 'shrink'}
            Margin on each side when shrunk
          {:else if pressEffect.type === 'shift'}
            Shift distance in pixels
          {:else}
            Bevel border thickness
          {/if}
        </p>
      </div>

      <div class="form-group">
        <label>Border Color</label>
        <ColorPicker
          value={pressEffect.border_color || ''}
          placeholder="auto (from button)"
          onUpdate={updatePressEffectBorderColor}
        />
        <p class="help">Fill color for the press effect border area (leave empty for auto)</p>
      </div>
    </div>
  </div>
</div>

<style>
  .device-settings {
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

  .help {
    margin: 0;
    font-size: 11px;
    color: #666;
    font-style: italic;
  }

  .brightness-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 8px;
    background: linear-gradient(to right, #3c3c3c 0%, #0e639c 100%);
    border-radius: 4px;
    outline: none;
    transition: opacity 0.2s;
  }

  .brightness-slider:hover {
    opacity: 0.9;
  }

  .brightness-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 18px;
    height: 18px;
    background: #0e639c;
    border: 2px solid #ffffff;
    border-radius: 50%;
    cursor: pointer;
    transition: background-color 0.2s, transform 0.2s;
  }

  .brightness-slider::-webkit-slider-thumb:hover {
    background: #1177bb;
    transform: scale(1.1);
  }

  .brightness-slider::-moz-range-thumb {
    width: 18px;
    height: 18px;
    background: #0e639c;
    border: 2px solid #ffffff;
    border-radius: 50%;
    cursor: pointer;
    transition: background-color 0.2s, transform 0.2s;
  }

  .brightness-slider::-moz-range-thumb:hover {
    background: #1177bb;
    transform: scale(1.1);
  }

  .section {
    padding-top: 12px;
    border-top: 1px solid #3e3e42;
  }

  h4 {
    margin: 0 0 4px 0;
    font-size: 13px;
    color: #aaa;
  }

  .pixels-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 8px;
    background: linear-gradient(to right, #3c3c3c 0%, #0e639c 100%);
    border-radius: 4px;
    outline: none;
  }

  .pixels-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    background: #0e639c;
    border: 2px solid #ffffff;
    border-radius: 50%;
    cursor: pointer;
  }

  .pixels-slider::-moz-range-thumb {
    width: 16px;
    height: 16px;
    background: #0e639c;
    border: 2px solid #ffffff;
    border-radius: 50%;
    cursor: pointer;
  }
</style>
