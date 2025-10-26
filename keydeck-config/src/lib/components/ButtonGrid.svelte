<script lang="ts">
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import DeviceSelector from './DeviceSelector.svelte';
  import { processEscapeSequences } from '$lib/utils/escapeChars';

  interface DeviceInfo {
    device_id: string;
    serial: string;
    model: string;
    button_layout: {
      rows: number;
      columns: number;
      total: number;
    };
    button_image: {
      width: number;
      height: number;
      format: string;
    };
    encoders?: number;
    touchpoints?: number;
    lcd_strip?: {
      width: number;
      height: number;
    };
    is_visual: boolean;
  }

  interface Props {
    device: DeviceInfo;
    config: any;
    currentPage: string;
    selectedButton: number | null;
    onButtonSelected: (index: number) => void;
    isTemplate?: boolean;
    pageName?: string;
    onPageTitleClicked?: () => void;
    onDeviceSelected?: (device: DeviceInfo) => void;
    onRefresh?: () => void;
  }

  let { device, config, currentPage, selectedButton, onButtonSelected, isTemplate = false, pageName, onPageTitleClicked, onDeviceSelected, onRefresh }: Props = $props();

  // Get icon directory from backend
  let iconDir = $state<string>('');

  async function getIconPath(): Promise<string> {
    try {
      const dir = await invoke<string>('ensure_default_icon_dir');
      return dir;
    } catch (e) {
      console.error('Failed to get icon directory:', e);
      return '';
    }
  }

  // Load icon directory on mount
  $effect(() => {
    getIconPath().then(dir => iconDir = dir);
  });

  // Get button configuration from template inheritance chain
  function getButtonFromTemplate(templateName: string, buttonKey: string, visited = new Set<string>()): any {
    if (visited.has(templateName)) return null;
    visited.add(templateName);

    const template = config.templates?.[templateName];
    if (!template) return null;

    // Check if this template has the button
    if (template[buttonKey]) return template[buttonKey];

    // Recursively check inherited templates
    if (template.inherits) {
      const inherits = Array.isArray(template.inherits) ? template.inherits : [template.inherits];
      for (const parentTemplate of inherits) {
        const result = getButtonFromTemplate(parentTemplate, buttonKey, visited);
        if (result) return result;
      }
    }

    return null;
  }

  // Get button configuration for the current page or template (including inherited)
  function getButtonConfig(index: number) {
    const buttonKey = `button${index}`;

    // If viewing a template, get button from template directly
    if (isTemplate) {
      const template = config.templates?.[currentPage];
      if (!template) return null;

      // First check if button is defined directly on the template
      if (template[buttonKey]) return template[buttonKey];

      // If not, check inherited templates
      if (template.inherits) {
        const inherits = Array.isArray(template.inherits) ? template.inherits : [template.inherits];
        for (const templateName of inherits) {
          const result = getButtonFromTemplate(templateName, buttonKey);
          if (result) return result;
        }
      }

      return null;
    }

    // Otherwise get from page
    const pageGroup = config.page_groups?.[device.serial] || config.page_groups?.default;
    if (!pageGroup) return null;

    // Pages are flattened, so currentPage is directly under pageGroup
    const page = pageGroup[currentPage];
    if (!page) return null;

    // First check if button is defined directly on the page
    if (page[buttonKey]) return page[buttonKey];

    // If not, check inherited templates
    if (page.inherits) {
      const inherits = Array.isArray(page.inherits) ? page.inherits : [page.inherits];
      for (const templateName of inherits) {
        const result = getButtonFromTemplate(templateName, buttonKey);
        if (result) return result;
      }
    }

    return null;
  }

  // Recursively check if button exists in template inheritance chain
  function checkTemplateForButton(templateName: string, buttonKey: string, visited = new Set<string>()): boolean {
    // Prevent infinite loops
    if (visited.has(templateName)) return false;
    visited.add(templateName);

    const template = config.templates?.[templateName];
    if (!template) return false;

    // Check if this template has the button
    if (template[buttonKey]) return true;

    // Recursively check inherited templates
    if (template.inherits) {
      const inherits = Array.isArray(template.inherits) ? template.inherits : [template.inherits];
      for (const parentTemplate of inherits) {
        if (checkTemplateForButton(parentTemplate, buttonKey, visited)) {
          return true;
        }
      }
    }

    return false;
  }

  // Check if button comes from inheritance
  function isInherited(index: number): boolean {
    const buttonKey = `button${index}`;

    // If viewing a template
    if (isTemplate) {
      const template = config.templates?.[currentPage];
      if (!template || !template.inherits) return false;

      // If button is defined directly on the template, it's not inherited (it overrides)
      if (template.hasOwnProperty(buttonKey)) return false;

      // Check if button exists in any of the inherited templates (recursively)
      const inherits = Array.isArray(template.inherits) ? template.inherits : [template.inherits];
      for (const templateName of inherits) {
        if (checkTemplateForButton(templateName, buttonKey)) {
          return true;
        }
      }

      return false;
    }

    // Otherwise viewing a page
    const pageGroup = config.page_groups?.[device.serial] || config.page_groups?.default;
    if (!pageGroup) return false;

    const page = pageGroup[currentPage];
    if (!page || !page.inherits) return false;

    // If button is defined directly on the page, it's not inherited (it overrides)
    if (page.hasOwnProperty(buttonKey)) return false;

    // Check if button exists in any of the inherited templates (recursively)
    const inherits = Array.isArray(page.inherits) ? page.inherits : [page.inherits];
    for (const templateName of inherits) {
      if (checkTemplateForButton(templateName, buttonKey)) {
        return true;
      }
    }

    return false;
  }

  // Check if button references a button definition (string reference)
  // This checks the DIRECT definition on the page/template, not inherited
  function isButtonDefReference(index: number): boolean {
    const buttonKey = `button${index}`;

    // If viewing a template
    if (isTemplate) {
      const template = config.templates?.[currentPage];
      if (!template) return false;

      // Check if directly defined as a string reference on this template
      return template.hasOwnProperty(buttonKey) && typeof template[buttonKey] === 'string';
    }

    // Otherwise viewing a page
    const pageGroup = config.page_groups?.[device.serial] || config.page_groups?.default;
    if (!pageGroup) return false;

    const page = pageGroup[currentPage];
    if (!page) return false;

    // Check if directly defined as a string reference on this page
    return page.hasOwnProperty(buttonKey) && typeof page[buttonKey] === 'string';
  }

  function getButtonLabel(index: number): string {
    let buttonConfig = getButtonConfig(index);

    if (!buttonConfig) return "";

    // Handle button definition reference (string) - resolve it
    if (typeof buttonConfig === 'string') {
      const buttonDefName = buttonConfig;
      const buttonDef = config.buttons?.[buttonDefName];
      if (buttonDef) {
        buttonConfig = buttonDef;
      } else {
        // If button definition doesn't exist, show the reference name
        return buttonDefName;
      }
    }

    // Handle detailed button config
    let text = "";
    if (buttonConfig.text) {
      if (typeof buttonConfig.text === 'string') {
        text = buttonConfig.text;
      } else if (buttonConfig.text.value) {
        text = buttonConfig.text.value;
      }
    }

    // Process escape sequences
    if (text) {
      text = processEscapeSequences(text);
    }

    // Replace ${...} dynamic variables with ⏱ for preview
    if (text) {
      text = text.replace(/\$\{[^}]+\}/g, '⏱');
    }

    return text;
  }

  function getButtonFontSize(index: number): number {
    let buttonConfig = getButtonConfig(index);
    if (!buttonConfig) return 32;

    // Resolve button definition reference
    if (typeof buttonConfig === 'string') {
      const buttonDef = config.buttons?.[buttonConfig];
      if (buttonDef) {
        buttonConfig = buttonDef;
      } else {
        return 32;
      }
    }

    // Get font size from config, default to 32
    let fontSize = 32;
    if (buttonConfig.text && typeof buttonConfig.text === 'object' && buttonConfig.text.font_size) {
      fontSize = buttonConfig.text.font_size;
    }

    // Scale based on button image size (default StreamDeck button is 72x72)
    const actualButtonSize = device.button_image.width || 72;
    const scale = actualButtonSize / 72;
    return fontSize * scale;
  }

  function hasConfig(index: number): boolean {
    return getButtonConfig(index) !== null && getButtonConfig(index) !== undefined;
  }

  function getButtonIcon(index: number): string | null {
    let buttonConfig = getButtonConfig(index);
    if (!buttonConfig) return null;

    // Resolve button definition reference
    if (typeof buttonConfig === 'string') {
      const buttonDef = config.buttons?.[buttonConfig];
      if (buttonDef) {
        buttonConfig = buttonDef;
      } else {
        return null;
      }
    }

    if (!buttonConfig.icon) return null;

    // Use hard-coded icon directory
    if (!iconDir) return null;

    // Build full path and convert to Tauri asset URL
    const fullPath = `${iconDir}/${buttonConfig.icon}`;
    return convertFileSrc(fullPath);
  }

  function getButtonOutline(index: number): string | null {
    let buttonConfig = getButtonConfig(index);
    if (!buttonConfig) return null;

    // Resolve button definition reference
    if (typeof buttonConfig === 'string') {
      const buttonDef = config.buttons?.[buttonConfig];
      if (buttonDef) {
        buttonConfig = buttonDef;
      } else {
        return null;
      }
    }

    return buttonConfig.outline || null;
  }

  function getButtonTextColor(index: number): string | null {
    let buttonConfig = getButtonConfig(index);
    if (!buttonConfig) return null;

    // Resolve button definition reference
    if (typeof buttonConfig === 'string') {
      const buttonDef = config.buttons?.[buttonConfig];
      if (buttonDef) {
        buttonConfig = buttonDef;
      } else {
        return null;
      }
    }

    return buttonConfig.text_color || null;
  }
</script>

<div class="button-grid-container">
  <div class="device-info">
    {#if config}
      <DeviceSelector onDeviceSelected={onDeviceSelected} onRefresh={onRefresh} />
    {/if}
  </div>

  <div class="center-content">
    {#if pageName}
      <div class="page-info">
        <h2
          class="page-title"
          class:clickable={!!onPageTitleClicked}
          onclick={() => onPageTitleClicked?.()}
          role={onPageTitleClicked ? "button" : undefined}
          tabindex={onPageTitleClicked ? 0 : undefined}
        >
          <span class="page-icon">{isTemplate ? '🏗️' : '🗂️'}</span>
          {pageName}
        </h2>
      </div>
    {/if}

    <div
      class="button-grid"
      style="
        grid-template-columns: repeat({device.button_layout.columns}, 1fr);
        grid-template-rows: repeat({device.button_layout.rows}, 1fr);
      "
    >
    {#each Array(device.button_layout.total) as _, index}
      {@const buttonIndex = index + 1}
      {@const iconUrl = getButtonIcon(buttonIndex)}
      {@const outline = getButtonOutline(buttonIndex)}
      {@const textColor = getButtonTextColor(buttonIndex)}
      {@const label = getButtonLabel(buttonIndex)}
      {@const fontSize = getButtonFontSize(buttonIndex)}
      <button
        class="grid-button"
        class:selected={selectedButton === buttonIndex}
        class:configured={hasConfig(buttonIndex) && !isInherited(buttonIndex) && !isButtonDefReference(buttonIndex)}
        class:inherited={isInherited(buttonIndex)}
        class:button-def-reference={isButtonDefReference(buttonIndex)}
        class:has-icon={iconUrl !== null}
        onclick={() => onButtonSelected(buttonIndex)}
        title="Button {buttonIndex}"
      >
        {#if iconUrl}
          <img src={iconUrl} alt="Button {buttonIndex}" class="button-icon" />
          {#if label}
            <span class="button-text-overlay" style="font-size: {fontSize * 0.3}px;{textColor ? ` color: ${textColor};` : ''}{outline ? ` --outline-color: ${outline};` : ''}">{label}</span>
          {/if}
        {:else if label}
          <span class="button-label" style="font-size: {fontSize * 0.3}px;{textColor ? ` color: ${textColor};` : ''}{outline ? ` --outline-color: ${outline};` : ''}">{label}</span>
        {:else}
          <span class="button-number">{buttonIndex}</span>
        {/if}
      </button>
    {/each}
    </div>
  </div>

  {#if device.lcd_strip}
    <div class="lcd-strip">
      <p>LCD Strip: {device.lcd_strip.width} × {device.lcd_strip.height}</p>
    </div>
  {/if}
</div>

<style>
  .button-grid-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
    min-height: 100%;
  }

  .device-info {
    display: flex;
    justify-content: center;
    align-items: center;
    width: 100%;
    padding: 20px 0;
  }

  .center-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    flex: 1;
    justify-content: center;
  }

  .page-info {
    text-align: center;
  }

  .page-title {
    margin: 0;
    font-size: 18px;
    color: #cccccc;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 12px;
    transition: color 0.2s;
  }

  .page-title.clickable {
    cursor: pointer;
    user-select: none;
  }

  .page-title.clickable:hover {
    color: #5b9bd5;
  }

  .page-title.clickable:active {
    color: #4a8ac2;
  }

  .page-icon {
    font-size: 18px;
  }

  .button-grid {
    display: grid;
    gap: 8px;
    padding: 20px;
    background-color: #2d2d30;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .grid-button {
    width: 80px;
    height: 80px;
    background: linear-gradient(135deg, #3c3c3c 0%, #2a2a2a 100%);
    border: 2px solid #555;
    border-radius: 8px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    transition: all 0.2s;
    position: relative;
  }

  .grid-button:hover {
    background: linear-gradient(135deg, #4a4a4a 0%, #333 100%);
    border-color: #777;
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.4);
  }

  .grid-button.selected {
    border-color: #0e639c;
    background: linear-gradient(135deg, #1a4d6d 0%, #0e3a52 100%);
    box-shadow: 0 0 12px rgba(14, 99, 156, 0.5);
  }

  .grid-button.configured {
    border-color: #4ec9b0;
  }

  .grid-button.configured.selected {
    border-color: #0e639c;
  }

  .grid-button.inherited {
    border-color: #d7a964;
  }

  .grid-button.inherited.selected {
    border-color: #0e639c;
  }

  .grid-button.button-def-reference {
    border-color: #b57edc;
  }

  .grid-button.button-def-reference.selected {
    border-color: #0e639c;
  }

  .button-number {
    position: absolute;
    top: 4px;
    left: 6px;
    font-size: 10px;
    color: #888;
    font-weight: 600;
  }

  .button-icon {
    max-width: 60px;
    max-height: 60px;
    object-fit: contain;
    position: absolute;
  }

  .button-text-overlay {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: white;
    text-align: center;
    padding: 2px 4px;
    font-weight: bold;
    white-space: pre-line;
    z-index: 1;
    max-width: 90%;
  }

  .button-text-overlay[style*="--outline-color"] {
    text-shadow:
      -1px -1px 0 var(--outline-color),
      1px -1px 0 var(--outline-color),
      -1px 1px 0 var(--outline-color),
      1px 1px 0 var(--outline-color),
      0 0 3px var(--outline-color);
  }

  .button-label {
    font-size: 11px;
    color: #cccccc;
    text-align: center;
    padding: 0 4px;
    overflow: hidden;
    white-space: pre-line;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .button-label[style*="--outline-color"] {
    text-shadow:
      -1px -1px 0 var(--outline-color),
      1px -1px 0 var(--outline-color),
      -1px 1px 0 var(--outline-color),
      1px 1px 0 var(--outline-color),
      0 0 3px var(--outline-color);
  }

  .lcd-strip {
    width: 100%;
    padding: 20px;
    text-align: center;
    margin-top: auto;
  }

  .lcd-strip p {
    margin: 0;
    font-size: 13px;
    color: #888;
  }
</style>
