<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  interface Props {
    value: string;
    placeholder?: string;
    onUpdate: (value: string) => void;
    dataColorName?: string;
    disabled?: boolean;
  }

  let { value, placeholder = "0xRRGGBB", onUpdate, dataColorName, disabled = false }: Props = $props();

  function updateColorFromText(newValue: string) {
    if (newValue.startsWith('#')) {
      newValue = '0x' + newValue.slice(1);
    }
    onUpdate(newValue);
  }

  function updateColorFromPicker(pickerValue: string) {
    onUpdate('0x' + pickerValue.slice(1));
  }

  function isEmpty(color: string): boolean {
    return !color || !color.trim();
  }

  function isValidColor(color: string): boolean {
    if (isEmpty(color)) return true; // Empty is considered "valid" (not invalid)

    // Check for 0xRRGGBB format (6 hex digits)
    if (color.startsWith('0x')) {
      const hex = color.slice(2);
      return /^[0-9A-Fa-f]{6}$/.test(hex);
    }

    // Check for #RRGGBB format (6 hex digits)
    if (color.startsWith('#')) {
      const hex = color.slice(1);
      return /^[0-9A-Fa-f]{6}$/.test(hex);
    }

    // Check for plain RRGGBB format (6 hex digits)
    return /^[0-9A-Fa-f]{6}$/.test(color);
  }

  function colorToHex(color: string): string {
    if (!color) return '#888888';
    if (color.startsWith('0x')) {
      return '#' + color.slice(2);
    }
    return color.startsWith('#') ? color : '#' + color;
  }
</script>

<div class="color-value-container">
  <input
    type="text"
    value={value || ''}
    oninput={(e) => updateColorFromText(e.currentTarget.value)}
    class="color-text-input"
    placeholder={placeholder}
    data-color-name={dataColorName}
    disabled={disabled}
  />
  <div class="color-picker-wrapper">
    <input
      type="color"
      value={colorToHex(value)}
      oninput={(e) => updateColorFromPicker(e.currentTarget.value)}
      class="color-picker-input"
      class:empty={isEmpty(value)}
      class:invalid={!isValidColor(value)}
      title="Pick color"
      disabled={disabled}
    />
    {#if !isValidColor(value) && !isEmpty(value)}
      <span class="invalid-indicator" title="Invalid color format">⚠</span>
    {/if}
  </div>
</div>

<style>
  .color-value-container {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }

  .color-text-input {
    flex: 1 1 auto;
    min-width: 0;
    width: 60px;
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

  .color-picker-wrapper {
    position: relative;
    display: inline-block;
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

  /* Checkerboard pattern for empty values */
  .color-picker-input.empty::-webkit-color-swatch {
    background-image:
      linear-gradient(45deg, #555 25%, transparent 25%),
      linear-gradient(-45deg, #555 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #555 75%),
      linear-gradient(-45deg, transparent 75%, #555 75%);
    background-size: 8px 8px;
    background-position: 0 0, 0 4px, 4px -4px, -4px 0px;
    background-color: #3a3a3a;
  }

  /* Red border for invalid values */
  .color-picker-input.invalid {
    border-color: #f48771;
  }

  .invalid-indicator {
    position: absolute;
    top: 50%;
    right: 6px;
    transform: translateY(-50%);
    font-size: 16px;
    color: #f48771;
    pointer-events: none;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.8);
  }
</style>
