<script lang="ts">
  import ActionEditor from './ActionEditor.svelte';
  import ColorField from './ColorField.svelte';
  import TriStateCheckbox from './TriStateCheckbox.svelte';
  import DrawConfigEditor from './DrawConfigEditor.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { ask, message } from '@tauri-apps/plugin-dialog';

  interface Props {
    config: any;
    currentPage: string;
    currentTemplate?: string;
    buttonIndex: number;
    deviceSerial: string;
    isTemplate?: boolean;
    customTitle?: string;
    isButtonDef?: boolean;
    onNavigateToTemplate?: (templateName: string, keepButtonSelection?: boolean) => void;
    onNavigateToButtonDef?: (buttonDefName: string, keepButtonSelection?: boolean) => void;
    onOpenGlobalSettings?: () => void;
  }

  let { config, currentPage, currentTemplate, buttonIndex, deviceSerial, isTemplate = false, customTitle, isButtonDef = false, onNavigateToTemplate, onNavigateToButtonDef, onOpenGlobalSettings }: Props = $props();

  // Compute the display name for the button
  let buttonDisplayName = $derived(customTitle || `Button ${buttonIndex}`);

  let availableIcons = $state<string[]>([]);
  let showIconDropdown = $state(false);
  let iconSearchFilter = $state("");
  let openActionIndex = $state<number>(-1);

  let showButtonDefDropdown = $state(false);
  let buttonDefSearchFilter = $state("");

  // Application browser state
  let showAppBrowser = $state(false);
  let availableApps = $state<{name: string; icon_path: string}[]>([]);
  let appSearchFilter = $state("");
  let loadingApps = $state(false);

  // Load available icons from hard-coded image directory
  async function loadIcons() {
    try {
      const icons = await invoke<string[]>('list_icons');
      availableIcons = icons || [];
    } catch (e) {
      console.error('Failed to load icons:', e);
      availableIcons = [];
    }
  }

  // Reload icons when config changes
  $effect(() => {
    if (config) {
      loadIcons();
    }
  });

  // Get icon URL from hard-coded icon directory
  async function getIconPath(): Promise<string> {
    try {
      const iconDir = await invoke<string>('ensure_default_icon_dir');
      return iconDir;
    } catch (e) {
      console.error('Failed to get icon directory:', e);
      return '';
    }
  }

  let iconDir = $state<string>('');

  // Load icon directory on mount
  $effect(() => {
    getIconPath().then(dir => iconDir = dir);
  });

  function getIconUrl(filename: string): string {
    if (!iconDir) return '';
    const fullPath = `${iconDir}/${filename}`;
    return convertFileSrc(fullPath);
  }

  function selectIcon(iconFile: string) {
    updateIcon(iconFile);
    showIconDropdown = false;
    iconSearchFilter = "";
  }

  async function selectButtonDef(buttonDefName: string) {
    // Check if button definition exists
    if (!config.buttons || !config.buttons[buttonDefName]) {
      const createNew = await ask(
        `Button definition "${buttonDefName}" doesn't exist.\n\nDo you want to create it first?`,
        {
          title: 'Button Definition Not Found',
          kind: 'warning'
        }
      );

      if (!createNew) {
        showButtonDefDropdown = false;
        buttonDefSearchFilter = "";
        return;
      }

      // Create empty button definition
      if (!config.buttons) {
        config.buttons = {};
      }
      config.buttons[buttonDefName] = {
        text: "",
        actions: []
      };
    }

    // Set button to reference
    if (isTemplate && template) {
      template[buttonKey] = buttonDefName;
      config.templates[currentTemplate][buttonKey] = buttonDefName;
      config.templates = { ...config.templates };
    } else if (page) {
      const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
      page[buttonKey] = buttonDefName;
      config[groupKey][currentPage][buttonKey] = buttonDefName;
      config.page_groups[groupKey] = { ...config.page_groups[groupKey] };
    }

    showButtonDefDropdown = false;
    buttonDefSearchFilter = "";
  }

  let filteredIcons = $derived(
    iconSearchFilter.trim()
      ? availableIcons.filter(icon => icon.toLowerCase().includes(iconSearchFilter.toLowerCase()))
      : availableIcons
  );

  let availableButtonDefs = $derived(Object.keys(config.buttons || {}).sort());

  let filteredButtonDefs = $derived(
    buttonDefSearchFilter.trim()
      ? availableButtonDefs.filter(name => name.toLowerCase().includes(buttonDefSearchFilter.toLowerCase()))
      : availableButtonDefs
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

  // Close button def dropdown when clicking outside
  $effect(() => {
    if (showButtonDefDropdown) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.buttondef-dropdown-container')) {
          showButtonDefDropdown = false;
          buttonDefSearchFilter = "";
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

  // Get the button definition reference name if it is one
  function getButtonDefReference(): string | null {
    if (isTemplate && template && typeof template[buttonKey] === 'string') {
      return template[buttonKey];
    } else if (page && typeof page[buttonKey] === 'string') {
      return page[buttonKey];
    }
    return null;
  }

  let buttonDefReference = $derived(getButtonDefReference());

  // Check if button has local configuration (State 1 or 2)
  let hasLocalConfig = $derived.by(() => {
    if (isTemplate && currentTemplate && config.templates?.[currentTemplate]) {
      return config.templates[currentTemplate].hasOwnProperty(buttonKey);
    } else if (!isTemplate && currentPage) {
      const groupKey = config.page_groups?.[deviceSerial] ? deviceSerial : 'default';
      return config.page_groups?.[groupKey]?.[currentPage]?.hasOwnProperty(buttonKey) || false;
    }
    return false;
  });

  // Check if button is read-only (inherited or a reference)
  let isReadOnly = $derived(buttonDefReference !== null || inheritedSource !== null);

  // Initialize button config if it doesn't exist, or convert reference to object
  function ensureButtonConfig() {
    if (isTemplate && template) {
      // For templates, ensure button exists in template as an object
      if (!template[buttonKey] || typeof template[buttonKey] === 'string') {
        // Copy inherited/referenced config if exists, otherwise create empty
        const inherited = getDetailedConfig();
        template[buttonKey] = inherited ? JSON.parse(JSON.stringify(inherited)) : {
          text: "",
          actions: []
        };
        config.templates[currentTemplate][buttonKey] = template[buttonKey];
        // Trigger reactivity by reassigning the templates object
        config.templates = { ...config.templates };
      }
    } else if (page) {
      // For pages, ensure button exists on page as an object
      if (!page[buttonKey] || typeof page[buttonKey] === 'string') {
        // Copy inherited/referenced config if exists, otherwise create empty
        const inherited = getDetailedConfig();
        page[buttonKey] = inherited ? JSON.parse(JSON.stringify(inherited)) : {
          text: "",
          actions: []
        };
        // Also update root level since it's flattened
        const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
        config[groupKey][currentPage][buttonKey] = page[buttonKey];
        // Trigger reactivity by reassigning the page group
        config.page_groups[groupKey] = { ...config.page_groups[groupKey] };
      }
    }
  }

  // Handle button definition references vs detailed config
  function getDetailedConfig() {
    if (typeof buttonConfig === 'string') {
      // It's a button definition reference, resolve it
      const buttonDefName = buttonConfig;
      const buttonDef = config.buttons?.[buttonDefName];
      if (buttonDef) {
        // Return the actual button definition properties
        return buttonDef;
      }
      // Fallback if button definition doesn't exist
      return { text: `[Missing ButtonDef: ${buttonDefName}]`, actions: [] };
    }
    return buttonConfig;
  }

  function updateButton(updates: any) {
    ensureButtonConfig();
    const detailed = getDetailedConfig();
    const newConfig = { ...detailed, ...updates };

    // Remove properties that are undefined or empty (to clean up empty fields)
    Object.keys(newConfig).forEach(key => {
      const value = newConfig[key];
      // Remove if undefined
      if (value === undefined) {
        delete newConfig[key];
      }
      // Remove if empty string (except for actions which can be legitimately empty)
      else if (key !== 'actions' && value === '') {
        delete newConfig[key];
      }
      // Remove if empty array (actions with no items)
      else if (Array.isArray(value) && value.length === 0) {
        delete newConfig[key];
      }
    });

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
    const detailed = getDetailedConfig();
    const currentText = detailed?.text;

    // If there's no value and no font_size, clear the text field
    if (!value && (!currentText || typeof currentText === 'string' || !currentText.font_size)) {
      updateButton({ text: undefined });
      return;
    }

    // If there's a font_size already set, preserve it
    if (currentText && typeof currentText === 'object' && currentText.font_size) {
      updateButton({ text: { value, font_size: currentText.font_size } });
    } else {
      // Simple string form
      updateButton({ text: value || undefined });
    }
  }

  function updateFontSize(value: string) {
    const detailed = getDetailedConfig();
    const currentText = detailed?.text;
    const textValue = getTextValue();

    // Parse and validate font size
    const fontSize = value ? parseFloat(value) : undefined;

    // If no font size is specified, use simple string form (if there's text)
    if (!fontSize || fontSize <= 0 || !isFinite(fontSize)) {
      if (textValue) {
        updateButton({ text: textValue });
      } else {
        updateButton({ text: undefined });
      }
      return;
    }

    // Use detailed form with font_size
    updateButton({
      text: {
        value: textValue || "",
        font_size: fontSize
      }
    });
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

  function handleDynamicChange(newValue: boolean | undefined) {
    updateButton({ dynamic: newValue });
  }

  function updateDraw(index: number, drawConfig: any) {
    const detailed = getDetailedConfig();
    const currentDraw = detailed.draw || [];

    if (drawConfig === null || drawConfig === undefined) {
      // Remove draw config at index
      const newDraw = currentDraw.filter((_: any, i: number) => i !== index);
      if (newDraw.length === 0) {
        // Remove draw field entirely if empty - pass draw: undefined to signal deletion
        updateButton({ draw: undefined });
      } else {
        updateButton({ draw: newDraw });
      }
    } else {
      // Update draw config at index
      const newDraw = [...currentDraw];
      newDraw[index] = drawConfig;
      updateButton({ draw: newDraw });
    }
  }

  function addGraphic() {
    ensureButtonConfig();
    const detailed = getDetailedConfig();
    const currentDraw = detailed.draw || [];
    const newDraw = [...currentDraw, {
      type: 'gauge',
      value: '',
      range: [0, 100],
      color: '#00ff00'
    }];
    updateButton({ draw: newDraw });
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

  function getFontSize(): number | undefined {
    const detailed = getDetailedConfig();
    if (!detailed?.text) return undefined;
    if (typeof detailed.text === 'string') return undefined;
    return detailed.text.font_size;
  }

  async function openIconSearchDialog() {
    // Toggle the app browser
    showAppBrowser = !showAppBrowser;

    // Load apps if not already loaded and we're showing the browser
    if (showAppBrowser && availableApps.length === 0 && !loadingApps) {
      loadingApps = true;
      try {
        const apps = await invoke<{name: string; icon_path: string}[]>('list_applications');
        availableApps = apps || [];
      } catch (e) {
        console.error('Failed to load applications:', e);
        await message('Failed to load applications. This feature is Linux-only.', { title: 'Error', kind: 'error' });
        showAppBrowser = false;
      } finally {
        loadingApps = false;
      }
    }

    // Reset search filter when opening
    if (showAppBrowser) {
      appSearchFilter = "";
    }
  }

  async function selectApp(app: {name: string; icon_path: string}) {
    try {
      // Copy the icon and get the filename
      const iconFilename = await invoke<string>('select_app_icon', {
        appName: app.name,
        iconPath: app.icon_path
      });

      // Select this icon
      updateIcon(iconFilename);

      // Close the app browser
      showAppBrowser = false;
      appSearchFilter = "";

      // Reload icon list to include the new icon
      await loadIcons();
    } catch (e) {
      console.error('Failed to select app icon:', e);
      await message(`Failed to copy icon: ${e}`, { title: 'Error', kind: 'error' });
    }
  }

  let filteredApps = $derived(
    availableApps.filter(app =>
      app.name.toLowerCase().includes(appSearchFilter.toLowerCase())
    )
  );

  async function clearButton() {
    const confirmed = await ask(
      `Clear all properties for ${buttonDisplayName}?\n\nThis will delete the button configuration from this ${isTemplate ? 'template' : 'page'}.`,
      {
        title: 'Confirm Clear',
        kind: 'warning'
      }
    );

    if (confirmed) {
      if (isTemplate && currentTemplate) {
        // Delete from template
        delete config.templates[currentTemplate][buttonKey];
        // Trigger reactivity by creating a new object
        config.templates = {
          ...config.templates,
          [currentTemplate]: { ...config.templates[currentTemplate] }
        };
      } else if (currentPage) {
        // Delete from page
        const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
        delete config.page_groups[groupKey][currentPage][buttonKey];
        // Trigger reactivity by creating new nested objects
        config.page_groups = {
          ...config.page_groups,
          [groupKey]: {
            ...config.page_groups[groupKey],
            [currentPage]: { ...config.page_groups[groupKey][currentPage] }
          }
        };
      }
    }
  }

  function createOverride() {
    ensureButtonConfig();
    // Trigger reactivity
    if (isTemplate) {
      config.templates = { ...config.templates };
    } else {
      const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
      config.page_groups[groupKey] = { ...config.page_groups[groupKey] };
    }
  }

  async function setButtonReference() {
    // Get list of available button definitions
    const availableButtonDefs = Object.keys(config.buttons || {});

    // For now, use a simple prompt. We can enhance this later with a proper selection dialog
    const buttonDefName = prompt(
      `Enter button definition name:\n\nAvailable button definitions:\n${availableButtonDefs.join(', ') || 'None'}`,
      ''
    );

    if (buttonDefName && buttonDefName.trim()) {
      const trimmedName = buttonDefName.trim();

      // Check if button definition exists
      if (!config.buttons || !config.buttons[trimmedName]) {
        const createNew = await ask(
          `Button definition "${trimmedName}" doesn't exist.\n\nDo you want to create it first?`,
          {
            title: 'Button Definition Not Found',
            kind: 'warning'
          }
        );

        if (!createNew) return;

        // Create empty button definition
        if (!config.buttons) {
          config.buttons = {};
        }
        config.buttons[trimmedName] = {
          text: "",
          actions: []
        };
      }

      // Set button to reference
      if (isTemplate && template) {
        template[buttonKey] = trimmedName;
        config.templates[currentTemplate][buttonKey] = trimmedName;
        config.templates = { ...config.templates };
      } else if (page) {
        const groupKey = config.page_groups[deviceSerial] ? deviceSerial : 'default';
        page[buttonKey] = trimmedName;
        config[groupKey][currentPage][buttonKey] = trimmedName;
        config.page_groups[groupKey] = { ...config.page_groups[groupKey] };
      }
    }
  }
</script>

<div class="button-editor">
  <div class="button-header">
    <div class="header-left">
      <h3>{buttonDisplayName}</h3>
      {#if buttonDefReference && onNavigateToButtonDef}
        <button class="reference-link" onclick={() => onNavigateToButtonDef(buttonDefReference, true)}>
          {buttonDefReference} →
        </button>
      {:else if inheritedSource && onNavigateToTemplate}
        <button class="inherited-link" onclick={() => onNavigateToTemplate(inheritedSource, true)}>
          {inheritedSource} →
        </button>
      {/if}
    </div>
    {#if hasLocalConfig && !isButtonDef}
      <button class="header-clear-button" onclick={clearButton} title="Remove Configuration">
        ✕
      </button>
    {/if}
  </div>

  {#if isReadOnly}
    <div class="state-actions-row">
      <button class="state-action-button override-button" onclick={createOverride}>
        ✏️ Override
      </button>
      <div class="buttondef-dropdown-container">
        <button
          class="state-action-button reference-button"
          onclick={() => showButtonDefDropdown = !showButtonDefDropdown}
        >
          🔗 Reference
        </button>

        {#if showButtonDefDropdown}
          <div class="buttondef-dropdown-menu">
            <input
              type="text"
              class="buttondef-search"
              placeholder="Search..."
              bind:value={buttonDefSearchFilter}
              onclick={(e) => e.stopPropagation()}
            />
            <div class="buttondef-options">
              {#if filteredButtonDefs.length > 0}
                {#each filteredButtonDefs as defName}
                  <button
                    class="buttondef-option"
                    onclick={() => selectButtonDef(defName)}
                  >
                    {defName}
                  </button>
                {/each}
              {:else}
                <div class="no-options">No matching button definitions</div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {:else if !hasLocalConfig}
    <div class="buttondef-dropdown-container">
      <button
        class="state-action-button reference-button"
        onclick={() => showButtonDefDropdown = !showButtonDefDropdown}
      >
        🔗 Set Button Reference
      </button>

      {#if showButtonDefDropdown}
        <div class="buttondef-dropdown-menu">
          <input
            type="text"
            class="buttondef-search"
            placeholder="Search..."
            bind:value={buttonDefSearchFilter}
            onclick={(e) => e.stopPropagation()}
          />
          <div class="buttondef-options">
            {#if filteredButtonDefs.length > 0}
              {#each filteredButtonDefs as defName}
                <button
                  class="buttondef-option"
                  onclick={() => selectButtonDef(defName)}
                >
                  {defName}
                </button>
              {/each}
            {:else}
              <div class="no-options">No matching button definitions</div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <div class="form-group">
    <label>Text</label>
    <div class="input-container" class:readonly={isReadOnly} class:reference={buttonDefReference !== null} class:inherited={inheritedSource !== null}>
      <input
        type="text"
        value={getTextValue()}
        oninput={(e) => updateText(e.currentTarget.value)}
        placeholder="Button label"
        disabled={isReadOnly}
      />
    </div>
  </div>

  <div class="form-group">
    <label>Icon</label>
    {#if availableIcons.length > 0}
      <div class="icon-selector-row">
        <div class="icon-dropdown-container">
          <button
            class="icon-dropdown-trigger"
            class:reference={buttonDefReference !== null}
            class:inherited={inheritedSource !== null}
            onclick={() => showIconDropdown = !showIconDropdown}
            disabled={isReadOnly}
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
        <button class="icon-search-btn" onclick={openIconSearchDialog} title="Search for application icons">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
            <circle cx="6.5" cy="6.5" r="5" stroke="currentColor" stroke-width="1.5"/>
            <line x1="10.5" y1="10.5" x2="14.5" y2="14.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <!-- Application Browser -->
      {#if showAppBrowser}
        <div class="app-browser">
          <div class="app-browser-header">
            <input
              type="text"
              class="app-search"
              placeholder="Search applications..."
              bind:value={appSearchFilter}
              onclick={(e) => e.stopPropagation()}
            />
            {#if loadingApps}
              <span class="loading-text">Loading applications...</span>
            {/if}
          </div>
          <div class="app-list">
            {#if filteredApps.length > 0}
              {#each filteredApps as app}
                <button
                  class="app-option"
                  onclick={() => selectApp(app)}
                  disabled={isReadOnly}
                >
                  <img src={convertFileSrc(app.icon_path)} alt="" class="app-icon-thumb" />
                  <span>{app.name}</span>
                </button>
              {/each}
            {:else if !loadingApps}
              <p class="no-apps">No applications found</p>
            {/if}
          </div>
        </div>
      {/if}
    {:else}
      <input
        type="text"
        value={getDetailedConfig()?.icon || ""}
        oninput={(e) => updateIcon(e.currentTarget.value)}
        placeholder="icon.png or path to image"
        disabled={isReadOnly}
      />
    {/if}
    <p class="help">Icons are stored in: {iconDir || '~/.config/keydeck/icons'}</p>
  </div>

  <div class="form-group">
    <div class="actions-header">
      <label>Actions</label>
      {#if !isReadOnly}
        <button class="add-btn" onclick={addAction}>+</button>
      {/if}
    </div>
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
            disabled={isReadOnly}
            isReference={buttonDefReference !== null}
            isInherited={inheritedSource !== null}
          />
        {/each}
      {:else}
        <p class="empty">No actions configured</p>
      {/if}
    </div>
  </div>

  <div class="form-group">
    <label>Text Color</label>
    <ColorField
      value={getDetailedConfig()?.text_color || ""}
      placeholder="0xRRGGBB"
      onUpdate={updateTextColor}
      readonly={isReadOnly}
      reference={buttonDefReference !== null}
      inherited={inheritedSource !== null}
    />
  </div>

  <div class="form-group">
    <label>Background</label>
    <ColorField
      value={getDetailedConfig()?.background || ""}
      placeholder="0xRRGGBB"
      onUpdate={updateBackground}
      readonly={isReadOnly}
      reference={buttonDefReference !== null}
      inherited={inheritedSource !== null}
    />
  </div>

  <div class="form-group">
    <label>Outline</label>
    <ColorField
      value={getDetailedConfig()?.outline || ""}
      placeholder="0xRRGGBB"
      onUpdate={updateOutline}
      readonly={isReadOnly}
      reference={buttonDefReference !== null}
      inherited={inheritedSource !== null}
    />
  </div>

  <div class="form-group">
    <label>Font Size</label>
    <div class="input-container" class:readonly={isReadOnly} class:reference={buttonDefReference !== null} class:inherited={inheritedSource !== null}>
      <input
        type="text"
        inputmode="decimal"
        value={getFontSize() ?? ""}
        oninput={(e) => {
          const input = e.currentTarget;
          let value = input.value;

          // Remove any non-numeric characters except the first decimal point
          let cleaned = '';
          let hasDecimal = false;
          for (let i = 0; i < value.length; i++) {
            const char = value[i];
            if (char >= '0' && char <= '9') {
              cleaned += char;
            } else if (char === '.' && !hasDecimal) {
              cleaned += char;
              hasDecimal = true;
            }
            // Skip any other characters (including extra dots)
          }

          // Update input if value changed
          if (cleaned !== value) {
            const cursorPos = input.selectionStart || 0;
            input.value = cleaned;
            // Try to maintain cursor position
            const newPos = Math.min(cursorPos, cleaned.length);
            input.setSelectionRange(newPos, newPos);
          }

          updateFontSize(cleaned);
        }}
        onblur={(e) => {
          // On blur, revalidate and clean up the value
          const value = e.currentTarget.value;
          const fontSize = value ? parseFloat(value) : undefined;
          if (fontSize && fontSize > 0 && isFinite(fontSize)) {
            // Valid value - ensure it's displayed correctly
            e.currentTarget.value = fontSize.toString();
          } else if (value) {
            // Invalid value - clear it
            e.currentTarget.value = "";
            updateFontSize("");
          }
        }}
        placeholder="Auto (leave empty for automatic)"
        disabled={isReadOnly}
      />
    </div>
    <p class="help">
      Font size in points. Leave empty for automatic sizing based on canvas dimensions.
    </p>
  </div>

  <div class="form-group">
    <TriStateCheckbox
      value={getDetailedConfig()?.dynamic}
      label="Dynamic Button"
      onToggle={handleDynamicChange}
      inheritLabel="Auto-detect"
      trueLabel="Always dynamic"
      falseLabel="Never dynamic"
      disabled={isReadOnly}
    />
    <p class="help">
      {#if getDetailedConfig()?.dynamic === undefined}
        Automatically detected based on button content (${'{provider:arg}'} patterns)
      {:else if getDetailedConfig()?.dynamic === true}
        Button will always refresh periodically
      {:else}
        Button will never refresh even if dynamic patterns are detected
      {/if}
    </p>
  </div>

  <!-- Graphics Rendering (DrawConfig) -->
  <div class="form-group">
    <div class="actions-header">
      <label>Graphics Rendering</label>
      {#if !isReadOnly}
        <button class="add-btn" onclick={addGraphic}>+</button>
      {/if}
    </div>
    <div class="actions-list">
      {#if getDetailedConfig()?.draw && getDetailedConfig().draw.length > 0}
        {#each getDetailedConfig().draw as drawConfig, index}
          <DrawConfigEditor
            drawConfig={drawConfig}
            onUpdate={(newDrawConfig) => updateDraw(index, newDrawConfig)}
            onRemove={() => updateDraw(index, null)}
            disabled={isReadOnly}
            isReference={buttonDefReference !== null}
            isInherited={inheritedSource !== null}
            initiallyOpen={index === 0}
          />
        {/each}
      {:else}
        <p class="empty">No graphics configured</p>
      {/if}
    </div>
  </div>
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

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
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

  .reference-link {
    padding: 4px 8px;
    background-color: #b57edc;
    color: #1e1e1e;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
    transition: background-color 0.2s;
  }

  .reference-link:hover {
    background-color: #c598e6;
  }

  .icon-selector-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .icon-search-btn {
    width: 32px;
    height: 32px;
    padding: 0;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s;
    flex-shrink: 0;
  }

  .icon-search-btn:hover {
    background-color: #1177b8;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .actions-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
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
    flex: 1;
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

  .icon-dropdown-trigger:disabled.reference {
    background-color: #3e3a4a;
    border-color: #4f4565;
  }

  .icon-dropdown-trigger:disabled.reference:hover {
    background-color: #3e3a4a;
  }

  .icon-dropdown-trigger:disabled.inherited {
    background-color: #4a4238;
    border-color: #5f5545;
  }

  .icon-dropdown-trigger:disabled.inherited:hover {
    background-color: #4a4238;
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

  /* Application Browser Styles */
  .app-browser {
    margin-top: 8px;
    background-color: #2d2d30;
    border: 1px solid #555;
    border-radius: 4px;
    max-height: 400px;
    display: flex;
    flex-direction: column;
  }

  .app-browser-header {
    padding: 8px;
    border-bottom: 1px solid #555;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .app-search {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
  }

  .app-search:focus {
    outline: none;
    border-color: #0e639c;
  }

  .loading-text {
    color: #888;
    font-size: 12px;
    font-style: italic;
  }

  .app-list {
    overflow-y: auto;
    max-height: 350px;
    padding: 4px;
  }

  .app-option {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    background-color: transparent;
    color: #cccccc;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    transition: background-color 0.2s;
  }

  .app-option:hover {
    background-color: #3c3c3c;
  }

  .app-icon-thumb {
    width: 32px;
    height: 32px;
    object-fit: contain;
    background-color: #2a2a2a;
    border-radius: 4px;
    padding: 4px;
    flex-shrink: 0;
  }

  .no-apps {
    text-align: center;
    color: #888;
    padding: 20px;
    font-style: italic;
  }

  /* Button Definition Dropdown Styles */
  .buttondef-dropdown-container {
    position: relative;
    margin-bottom: 12px;
    align-self: stretch;
  }

  .buttondef-dropdown-menu {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 0;
    background-color: #2d2d30;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 1000;
    max-height: 300px;
    display: flex;
    flex-direction: column;
  }

  .buttondef-search {
    margin: 8px;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .buttondef-search:focus {
    outline: none;
    border-color: #0e639c;
  }

  .buttondef-options {
    overflow-y: auto;
    max-height: 240px;
  }

  .buttondef-option {
    width: 100%;
    padding: 10px 12px;
    background: none;
    border: none;
    color: #cccccc;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
  }

  .buttondef-option:hover {
    background-color: #3c3c3c;
  }

  .no-options {
    padding: 20px;
    text-align: center;
    color: #888;
    font-size: 12px;
    font-style: italic;
  }

  .input-container {
    padding: 8px;
    background-color: #3c3c3c;
    border: 1px solid #555;
    border-radius: 4px;
  }

  .input-container.readonly.reference {
    background-color: #3e3a4a;
    border-color: #4f4565;
  }

  .input-container.readonly.inherited {
    background-color: #4a4238;
    border-color: #5f5545;
  }

  .input-container input {
    width: 100%;
    padding: 0;
    background: none;
    border: none;
    color: inherit;
  }

  .input-container input:focus {
    outline: none;
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

  .help {
    margin: 0;
    font-size: 11px;
    color: #666;
    font-style: italic;
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

  .header-clear-button {
    padding: 4px 8px;
    background-color: #7a2d2d;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
    transition: background-color 0.2s;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
  }

  .header-clear-button:hover {
    background-color: #9a3d3d;
  }

  .state-actions-row {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
    align-items: stretch;
  }

  .state-actions-row > .state-action-button,
  .state-actions-row > .buttondef-dropdown-container > .state-action-button {
    flex: 1;
    margin-bottom: 0;
    min-height: 40px;
    height: auto;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .state-actions-row {
    position: relative;
  }

  .state-actions-row > .buttondef-dropdown-container {
    flex: 1;
    margin-bottom: 0;
    display: flex;
    min-width: 0;
    position: static;
  }

  .state-actions-row .buttondef-dropdown-menu {
    left: 0;
    right: 0;
    width: 100%;
    margin-top: 8px;
  }

  .state-actions-row .buttondef-dropdown-menu::before {
    content: '';
    position: absolute;
    top: -6px;
    left: 75%;
    transform: translateX(-50%);
    width: 0;
    height: 0;
    border-left: 6px solid transparent;
    border-right: 6px solid transparent;
    border-bottom: 6px solid #555;
  }

  .state-actions-row .buttondef-dropdown-menu::after {
    content: '';
    position: absolute;
    top: -5px;
    left: 75%;
    transform: translateX(-50%);
    width: 0;
    height: 0;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-bottom: 5px solid #2d2d30;
  }

  .buttondef-dropdown-container:not(.state-actions-row *) .buttondef-dropdown-menu {
    margin-top: 0;
  }

  .state-action-button {
    width: 100%;
    padding: 8px 12px;
    margin-bottom: 12px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    transition: background-color 0.2s;
  }

  .override-button {
    background-color: #4a4a4a;
    color: white;
  }

  .override-button:hover {
    background-color: #5a5a5a;
  }

  .reference-button {
    background-color: #4a4a6a;
    color: white;
  }

  .reference-button:hover {
    background-color: #5a5a7a;
  }
</style>
