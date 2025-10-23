<script lang="ts">
  import ActionEditor from './ActionEditor.svelte';
  import ColorPicker from './ColorPicker.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { ask } from '@tauri-apps/plugin-dialog';

  interface Props {
    config: any;
    currentPage: string;
    currentTemplate?: string;
    buttonIndex: number;
    deviceSerial: string;
    isTemplate?: boolean;
    customTitle?: string;
    onNavigateToTemplate?: (templateName: string, keepButtonSelection?: boolean) => void;
  }

  let { config, currentPage, currentTemplate, buttonIndex, deviceSerial, isTemplate = false, customTitle, onNavigateToTemplate }: Props = $props();

  // Compute the display name for the button
  let buttonDisplayName = $derived(customTitle || `Button ${buttonIndex}`);

  let availableIcons = $state<string[]>([]);
  let showIconDropdown = $state(false);
  let iconSearchFilter = $state("");
  let openActionIndex = $state<number>(-1);

  // Load available icons from image directory
  async function loadIcons() {
    try {
      const icons = await invoke<string[]>('list_icons', { imageDir: config?.image_dir });
      availableIcons = icons || [];
    } catch (e) {
      console.error('Failed to load icons:', e);
      availableIcons = [];
    }
  }

  // Reload icons when config or image_dir changes
  $effect(() => {
    if (config) {
      loadIcons();
    }
  });

  function getIconUrl(filename: string): string {
    if (!config?.image_dir) return '';
    const fullPath = `${config.image_dir}/${filename}`;
    return convertFileSrc(fullPath);
  }

  function selectIcon(iconFile: string) {
    updateIcon(iconFile);
    showIconDropdown = false;
    iconSearchFilter = "";
  }

  let filteredIcons = $derived(
    iconSearchFilter.trim()
      ? availableIcons.filter(icon => icon.toLowerCase().includes(iconSearchFilter.toLowerCase()))
      : availableIcons
  );

  // Close dropdown when clicking outside
  $effect(() => {
    if (showIconDropdown) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.icon-dropdown-container')) {
          showIconDropdown = false;
          iconSearchFilter = "";
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  let buttonKey = $derived(`button${buttonIndex}`);
  let pageGroup = $derived(config.page_groups?.[deviceSerial] || config.page_groups?.default);
  // Pages are flattened, so currentPage is directly under pageGroup
  let page = $derived(pageGroup?.[currentPage]);
  let template = $derived(isTemplate ? config.templates?.[currentTemplate] : null);

  // Get button config including inherited configurations
  let buttonConfig = $derived.by(() => {
    // If viewing a template
    if (isTemplate && template) {
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

    // Otherwise viewing a page
    if (!page) return null;

    // First check if button is defined directly on the page
    if (page[buttonKey]) return page[buttonKey];

    // If not, check inherited templates
    if (page.inherits && config.templates) {
      const inherits = Array.isArray(page.inherits) ? page.inherits : [page.inherits];
      for (const templateName of inherits) {
        const result = getButtonFromTemplate(templateName, buttonKey);
        if (result) return result;
      }
    }

    return null;
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

  // Find which template the button is inherited from (recursively finds the deepest source)
  function getInheritedSource(): string | null {
    if (isTemplate && template) {
      // Check if button is directly on this template
      if (template[buttonKey]) return null;

      // Check inherited templates recursively
      if (template.inherits) {
        const inherits = Array.isArray(template.inherits) ? template.inherits : [template.inherits];
        for (const templateName of inherits) {
          const source = findButtonSource(templateName, buttonKey);
          if (source) return source;
        }
      }
    } else if (page) {
      // Check if button is directly on this page
      if (page[buttonKey]) return null;

      // Check inherited templates recursively
      if (page.inherits) {
        const inherits = Array.isArray(page.inherits) ? page.inherits : [page.inherits];
        for (const templateName of inherits) {
          const source = findButtonSource(templateName, buttonKey);
          if (source) return source;
        }
      }
    }

    return null;
  }

  // Find the first template that directly defines this button (stops at first match, doesn't recurse deeper)
  function findButtonSource(templateName: string, buttonKey: string, visited = new Set<string>()): string | null {
    if (visited.has(templateName)) return null;
    visited.add(templateName);

    const tmpl = config.templates?.[templateName];
    if (!tmpl) return null;

    // If this template has the button directly defined (not inherited), return it
    if (tmpl.hasOwnProperty(buttonKey)) {
      return templateName;
    }

    // This template doesn't have the button directly, check parents
    if (tmpl.inherits) {
      const inherits = Array.isArray(tmpl.inherits) ? tmpl.inherits : [tmpl.inherits];
      for (const parent of inherits) {
        const source = findButtonSource(parent, buttonKey, visited);
        if (source) return source;
      }
    }

    return null;
  }

  // Check if template has button (recursively)
  function checkTemplateHasButton(templateName: string, buttonKey: string, visited = new Set<string>()): boolean {
    if (visited.has(templateName)) return false;
    visited.add(templateName);

    const tmpl = config.templates?.[templateName];
    if (!tmpl) return false;

    if (tmpl[buttonKey]) return true;

    if (tmpl.inherits) {
      const inherits = Array.isArray(tmpl.inherits) ? tmpl.inherits : [tmpl.inherits];
      for (const parent of inherits) {
        if (checkTemplateHasButton(parent, buttonKey, visited)) {
          return true;
        }
      }
    }

    return false;
  }

  let inheritedSource = $derived(getInheritedSource());

  // Initialize button config if it doesn't exist
  function ensureButtonConfig() {
    if (isTemplate && template) {
      // For templates, ensure button exists in template
      if (!template[buttonKey]) {
        // Copy inherited config if exists, otherwise create empty
        const inherited = getDetailedConfig();
        template[buttonKey] = inherited ? JSON.parse(JSON.stringify(inherited)) : {
          text: "",
          actions: []
        };
        config.templates[currentTemplate][buttonKey] = template[buttonKey];
      }
    } else if (page) {
      // For pages, ensure button exists on page
      if (!page[buttonKey]) {
        // Copy inherited config if exists, otherwise create empty
        const inherited = getDetailedConfig();
        page[buttonKey] = inherited ? JSON.parse(JSON.stringify(inherited)) : {
          text: "",
          actions: []
        };
        // Also update root level since it's flattened
        const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
        config[groupKey][currentPage][buttonKey] = page[buttonKey];
      }
    }
  }

  // Handle template references vs detailed config
  function getDetailedConfig() {
    if (typeof buttonConfig === 'string') {
      // It's a template reference, convert to detailed
      return { text: `[Template: ${buttonConfig}]`, actions: [] };
    }
    return buttonConfig;
  }

  function updateButton(updates: any) {
    ensureButtonConfig();
    const detailed = getDetailedConfig();
    const newConfig = { ...detailed, ...updates };

    if (isTemplate && template) {
      // Update template
      template[buttonKey] = newConfig;
      config.templates[currentTemplate][buttonKey] = newConfig;
    } else if (page) {
      // Update page
      const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
      page[buttonKey] = newConfig;
      config[groupKey][currentPage][buttonKey] = newConfig;
    }
  }

  function updateText(value: string) {
    updateButton({ text: value });
  }

  function updateIcon(value: string) {
    updateButton({ icon: value || undefined });
  }

  function updateBackground(value: string) {
    updateButton({ background: value || undefined });
  }

  function updateTextColor(value: string) {
    updateButton({ text_color: value || undefined });
  }

  function updateOutline(value: string) {
    updateButton({ outline: value || undefined });
  }

  function addAction() {
    ensureButtonConfig();
    const detailed = getDetailedConfig();
    if (!detailed.actions) {
      detailed.actions = [];
    }
    detailed.actions.push({ jump: "" });
    openActionIndex = detailed.actions.length - 1;
    updateButton(detailed);
  }

  function updateAction(index: number, newAction: any) {
    const detailed = getDetailedConfig();
    if (detailed.actions) {
      detailed.actions[index] = newAction;
      updateButton(detailed);
    }
  }

  function removeAction(index: number) {
    const detailed = getDetailedConfig();
    if (detailed.actions) {
      detailed.actions.splice(index, 1);
      updateButton(detailed);
    }
  }

  function getTextValue(): string {
    const detailed = getDetailedConfig();
    if (!detailed?.text) return '';
    if (typeof detailed.text === 'string') return detailed.text;
    if (detailed.text.value) return detailed.text.value;
    return '';
  }

  async function clearButton() {
    const confirmed = await ask(
      `Clear all properties for ${buttonDisplayName}?\n\nThis will delete the button configuration from this ${isTemplate ? 'template' : 'page'}.`,
      {
        title: 'Confirm Clear',
        kind: 'warning'
      }
    );

    if (confirmed) {
      if (isTemplate && template) {
        // Delete from template
        delete template[buttonKey];
        delete config.templates[currentTemplate][buttonKey];
      } else if (page) {
        // Delete from page
        const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
        delete page[buttonKey];
        delete config[groupKey][currentPage][buttonKey];
      }
    }
  }
</script>

<div class="button-editor">
  <div class="button-header">
    <h3>{buttonDisplayName}</h3>
    {#if inheritedSource && onNavigateToTemplate}
      <button class="inherited-link" onclick={() => onNavigateToTemplate(inheritedSource, true)}>
        from {inheritedSource} →
      </button>
    {/if}
  </div>

  <div class="form-group">
    <label>Text</label>
    <input
      type="text"
      value={getTextValue()}
      oninput={(e) => updateText(e.currentTarget.value)}
      placeholder="Button label"
    />
  </div>

  <div class="form-group">
    <label>Icon</label>
    {#if availableIcons.length > 0}
      <div class="icon-dropdown-container">
        <button
          class="icon-dropdown-trigger"
          onclick={() => showIconDropdown = !showIconDropdown}
        >
          <div class="selected-icon">
            {#if getDetailedConfig()?.icon}
              <img src={getIconUrl(getDetailedConfig().icon)} alt="" class="icon-thumb" />
              <span>{getDetailedConfig().icon}</span>
            {:else}
              <span class="no-icon-text">No icon</span>
            {/if}
          </div>
          <span class="dropdown-arrow">▼</span>
        </button>

        {#if showIconDropdown}
          <div class="icon-dropdown-menu">
            <input
              type="text"
              class="icon-search"
              placeholder="Search icons..."
              bind:value={iconSearchFilter}
              onclick={(e) => e.stopPropagation()}
            />
            <div class="icon-options">
              <button
                class="icon-option"
                class:selected={!getDetailedConfig()?.icon}
                onclick={() => selectIcon("")}
              >
                <span class="no-icon-text">No icon</span>
              </button>
              {#each filteredIcons as iconFile}
                <button
                  class="icon-option"
                  class:selected={getDetailedConfig()?.icon === iconFile}
                  onclick={() => selectIcon(iconFile)}
                >
                  <img src={getIconUrl(iconFile)} alt="" class="icon-thumb" />
                  <span>{iconFile}</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <input
        type="text"
        value={getDetailedConfig()?.icon || ""}
        oninput={(e) => updateIcon(e.currentTarget.value)}
        placeholder="icon.png or path to image"
      />
    {/if}
  </div>

  <div class="form-group">
    <label>Background</label>
    <div class="color-item">
      <div class="color-info">
        <ColorPicker
          value={getDetailedConfig()?.background || ""}
          placeholder="0xRRGGBB"
          onUpdate={updateBackground}
        />
      </div>
    </div>
  </div>

  <div class="form-group">
    <label>Text Color</label>
    <div class="color-item">
      <div class="color-info">
        <ColorPicker
          value={getDetailedConfig()?.text_color || ""}
          placeholder="0xRRGGBB"
          onUpdate={updateTextColor}
        />
      </div>
    </div>
  </div>

  <div class="form-group">
    <label>Outline</label>
    <div class="color-item">
      <div class="color-info">
        <ColorPicker
          value={getDetailedConfig()?.outline || ""}
          placeholder="0xRRGGBB"
          onUpdate={updateOutline}
        />
      </div>
    </div>
  </div>

  <div class="form-group">
    <label>Actions</label>
    <div class="actions-list">
      {#if getDetailedConfig()?.actions && getDetailedConfig().actions.length > 0}
        {#each getDetailedConfig().actions as action, i (buttonIndex + '-' + i)}
          <ActionEditor
            {action}
            index={i}
            {config}
            {deviceSerial}
            initiallyOpen={i === openActionIndex}
            onToggle={() => openActionIndex = i}
            onUpdate={(newAction) => updateAction(i, newAction)}
            onDelete={() => removeAction(i)}
          />
        {/each}
      {:else}
        <p class="empty">No actions configured</p>
      {/if}
    </div>
    <button onclick={addAction}>+ Add Action</button>
  </div>

  <button class="clear-button" onclick={clearButton}>🗑️ Clear Button</button>
</div>

<style>
  .button-editor {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .button-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 12px;
    border-bottom: 1px solid #3e3e42;
  }

  h3 {
    margin: 0;
    font-size: 16px;
    color: #cccccc;
  }

  .inherited-link {
    padding: 4px 8px;
    background-color: #d7a964;
    color: #1e1e1e;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
    transition: background-color 0.2s;
  }

  .inherited-link:hover {
    background-color: #e5b876;
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

  .icon-dropdown-container {
    position: relative;
  }

  .icon-dropdown-trigger {
    width: 100%;
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-align: left;
  }

  .icon-dropdown-trigger:hover {
    background-color: #4a4a4a;
  }

  .selected-icon {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }

  .dropdown-arrow {
    font-size: 10px;
    color: #888;
  }

  .icon-thumb {
    width: 24px;
    height: 24px;
    object-fit: contain;
    background-color: #2a2a2a;
    border-radius: 2px;
    padding: 2px;
  }

  .no-icon-text {
    color: #888;
    font-style: italic;
  }

  .icon-dropdown-menu {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    background-color: #2d2d30;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 1000;
    max-height: 300px;
    display: flex;
    flex-direction: column;
  }

  .icon-search {
    margin: 8px;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .icon-search:focus {
    outline: none;
    border-color: #0e639c;
  }

  .icon-options {
    overflow-y: auto;
    max-height: 240px;
  }

  .icon-option {
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: #cccccc;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    font-size: 13px;
  }

  .icon-option:hover {
    background-color: #3c3c3c;
  }

  .icon-option.selected {
    background-color: #0e639c;
  }

  .color-item {
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

  .actions-list {
    display: flex;
    flex-direction: column;
    margin-bottom: 8px;
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

  .clear-button {
    background-color: #7a2d2d;
    color: white;
    padding: 8px;
    font-size: 13px;
    font-weight: 600;
    margin-top: 8px;
  }

  .clear-button:hover {
    background-color: #9a3d3d;
  }
</style>
