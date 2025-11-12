<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import DeviceSelector from './DeviceSelector.svelte';
  import HelperButtons from './HelperButtons.svelte';
  import { processEscapeSequences } from '$lib/utils/escapeChars';
  import { iconRefreshTrigger } from '$lib/stores';

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
    isEditMode?: boolean;
    onHomeClick?: () => void;
    onToggleMode?: () => void;
    onPageJump?: (pageName: string) => void;
  }

  let { device, config, currentPage, selectedButton, onButtonSelected, isTemplate = false, pageName, onPageTitleClicked, onDeviceSelected, onRefresh, isEditMode = true, onHomeClick, onToggleMode, onPageJump }: Props = $props();

  let draggedButtonIndex = $state<number | null>(null);
  let dropTargetIndex = $state<number | null>(null);

  // Load available icons with data URLs
  let availableIcons = $state<{filename: string; data_url: string}[]>([]);

  async function loadIcons() {
    try {
      const icons = await invoke<{filename: string; data_url: string}[]>('list_icons');
      availableIcons = icons || [];
    } catch (e) {
      console.error('Failed to load icons:', e);
      availableIcons = [];
    }
  }

  // Load icons on mount and when config changes
  $effect(() => {
    if (config) {
      loadIcons();
    }
  });

  // Reload icons when cleanup is triggered
  $effect(() => {
    // Subscribe to iconRefreshTrigger changes
    $iconRefreshTrigger;
    loadIcons();
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

    // Find icon data URL from loaded icons
    const icon = availableIcons.find(i => i.filename === buttonConfig.icon);
    return icon?.data_url || null;
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

  // Find the first jump or autojump action in a button's configuration
  // Helper function to recursively search for jump/auto_jump/focus actions
  function searchActionsRecursive(actions: any[], visited = new Set<string>()): { target: string; isAutoJump: boolean; isFocus?: boolean } | null {
    for (const action of actions) {
      // Check for direct navigation actions
      if (action.jump) {
        return { target: action.jump, isAutoJump: false };
      }
      if ('auto_jump' in action) {
        return { target: action.auto_jump, isAutoJump: true };
      }
      if (action.focus) {
        return { target: action.focus, isAutoJump: false, isFocus: true };
      }

      // Recursively search in macro actions
      if (action.macro) {
        const macroName = typeof action.macro === 'string' ? action.macro : action.macro?.name;

        // Prevent infinite loops from recursive macros
        if (macroName && !visited.has(macroName)) {
          visited.add(macroName);

          const macro = config?.macros?.[macroName];
          if (macro && macro.actions && Array.isArray(macro.actions)) {
            const result = searchActionsRecursive(macro.actions, visited);
            if (result) return result;
          }
        }
      }

      // Recursively search in try blocks (both try and else branches)
      if (action.try && Array.isArray(action.try)) {
        const result = searchActionsRecursive(action.try, visited);
        if (result) return result;
      }
      if (action.else && Array.isArray(action.else)) {
        const result = searchActionsRecursive(action.else, visited);
        if (result) return result;
      }

      // Recursively search in or blocks
      if (action.or && Array.isArray(action.or)) {
        const result = searchActionsRecursive(action.or, visited);
        if (result) return result;
      }

      // Recursively search in and blocks
      if (action.and && Array.isArray(action.and)) {
        const result = searchActionsRecursive(action.and, visited);
        if (result) return result;
      }

      // Skip 'not' blocks as requested
    }

    return null;
  }

  function findFirstJumpAction(index: number): { target: string; isAutoJump: boolean; isFocus?: boolean } | null {
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

    // Check if button has actions
    if (!buttonConfig.actions || !Array.isArray(buttonConfig.actions)) {
      return null;
    }

    // Recursively find the first jump, auto_jump, or focus action
    return searchActionsRecursive(buttonConfig.actions);
  }

  // Get all pages with their window names from the configuration
  function getPagesWithWindowNames(): { pageName: string; windowName: string | null }[] {
    const pages: { pageName: string; windowName: string | null }[] = [];

    // Check if pages are in page_groups
    if (config?.page_groups) {
      for (const [groupName, groupConfig] of Object.entries(config.page_groups)) {
        const group = groupConfig as any;

        // Known fields that are not page names
        const knownFields = ['main_page', 'restore_mode', 'on_tick'];

        // Iterate through all keys in the group
        for (const [key, value] of Object.entries(group)) {
          // Skip known fields
          if (knownFields.includes(key)) continue;

          const page = value as any;
          // Check if this is a page object (has properties like buttons, inherits, etc.)
          if (typeof page === 'object' && page !== null) {
            pages.push({
              pageName: key,
              windowName: page.window_name || null
            });
          }
        }
      }
    }

    return pages;
  }

  // Simple fuzzy matching function - returns similarity score between 0 and 1
  function fuzzyMatch(str1: string, str2: string): number {
    const s1 = str1.toLowerCase();
    const s2 = str2.toLowerCase();
    
    // Exact match gets highest score
    if (s1 === s2) return 1.0;
    
    // Check if one contains the other
    if (s1.includes(s2) || s2.includes(s1)) return 0.8;
    
    // Simple character matching score
    let matches = 0;
    let shorter = s1.length < s2.length ? s1 : s2;
    let longer = s1.length >= s2.length ? s1 : s2;
    
    for (let i = 0; i < shorter.length; i++) {
      if (longer.includes(shorter[i])) {
        matches++;
      }
    }
    
    return matches / longer.length;
  }

  // Find the best matching page for a given target using fuzzy matching against window names
  function findBestPageMatch(target: string): string | null {
    if (!target) return null;

    const allPages = getPagesWithWindowNames();
    if (allPages.length === 0) return null;

    let bestMatch = null;
    let bestScore = 0;

    for (const page of allPages) {
      // Only match against pages that have a window name
      if (page.windowName) {
        const score = fuzzyMatch(target, page.windowName);
        if (score > bestScore) {
          bestScore = score;
          bestMatch = page.pageName;
        }
      }
    }

    // Only return a match if it has a reasonable similarity score (0.3 or higher)
    return bestScore >= 0.3 ? bestMatch : null;
  }

  // Handle button click - either select for editing or jump in play mode
  function handleButtonClick(index: number) {
    if (isEditMode || isTemplate) {
      // In edit mode or viewing a template, select the button for editing
      onButtonSelected(index);
    } else {
      // In play mode, find first jump, auto_jump, or focus action and navigate
      const jumpResult = findFirstJumpAction(index);

      if (jumpResult) {
        if (jumpResult.isAutoJump) {
          // For auto_jump, go to main page
          if (onHomeClick) {
            onHomeClick();
          }
        } else if (jumpResult.isFocus) {
          // For focus, use fuzzy matching to find the best page match
          const bestMatch = findBestPageMatch(jumpResult.target);
          if (bestMatch && onPageJump) {
            onPageJump(bestMatch);
          }
        } else {
          // For regular jump, go to the specified target
          if (onPageJump) {
            onPageJump(jumpResult.target);
          }
        }
      }
    }
  }

  // Drag & drop handlers for button configuration copying
  function handleDragStart(event: DragEvent, index: number) {
    if (!isEditMode && !isTemplate) return; // Only in edit mode

    // Only allow dragging configured buttons that are not inherited or references
    if (!hasConfig(index) || isInherited(index) || isButtonDefReference(index)) {
      event.preventDefault();
      return;
    }

    draggedButtonIndex = index;
    event.dataTransfer!.effectAllowed = 'copy';
    event.dataTransfer!.setData('text/plain', index.toString());

    // Create a custom drag image to avoid clipping issues
    const draggedElement = event.target as HTMLElement;
    const clone = draggedElement.cloneNode(true) as HTMLElement;

    // Adjust for HiDPI displays
    const scale = 1 / window.devicePixelRatio;
    const width = draggedElement.offsetWidth;
    const height = draggedElement.offsetHeight;

    // Style the clone to ensure it's fully visible
    clone.style.position = 'absolute';
    clone.style.top = '-1000px';
    clone.style.left = '-1000px';
    clone.style.width = width + 'px';
    clone.style.height = height + 'px';
    clone.style.transform = `scale(${scale})`;
    clone.style.transformOrigin = '0 0';
    clone.style.pointerEvents = 'none';

    document.body.appendChild(clone);

    // Use the clone as drag image, centered
    event.dataTransfer!.setDragImage(clone, (width * scale) / 2, (height * scale) / 2);

    // Remove clone after a short delay
    setTimeout(() => {
      document.body.removeChild(clone);
    }, 0);
  }

  function handleDragEnd() {
    draggedButtonIndex = null;
    dropTargetIndex = null;
  }

  function handleDragOver(event: DragEvent, index: number) {
    if (!isEditMode && !isTemplate) return;
    if (draggedButtonIndex === null) return;
    if (draggedButtonIndex === index) return; // Can't drop on itself

    // Allow dropping on:
    // 1. Empty buttons (no config at all)
    // 2. Inherited buttons (no local config, only inherited)
    // Don't allow dropping on:
    // 1. Locally configured buttons (would overwrite user's config)
    // 2. Button def references (these are intentional references, not empty)
    if (hasConfig(index) && !isInherited(index)) {
      return; // Don't allow dropping on locally configured buttons or references
    }

    event.preventDefault();
    event.dataTransfer!.dropEffect = 'copy';
    dropTargetIndex = index;
  }

  function handleDragLeave(event: DragEvent, index: number) {
    if (dropTargetIndex === index) {
      dropTargetIndex = null;
    }
  }

  function handleDrop(event: DragEvent, targetIndex: number) {
    event.preventDefault();

    if (draggedButtonIndex === null) return;
    if (!hasConfig(draggedButtonIndex)) return;

    // Don't allow dropping on locally configured buttons or button def references
    if (hasConfig(targetIndex) && !isInherited(targetIndex)) return;

    // Get the source button config
    const sourceButtonKey = `button${draggedButtonIndex}`;
    const targetButtonKey = `button${targetIndex}`;

    // Get the source config
    let sourceConfig = getButtonConfig(draggedButtonIndex);
    if (!sourceConfig) return;

    // Deep clone the config to avoid reference issues
    const clonedConfig = JSON.parse(JSON.stringify(sourceConfig));

    // Apply to target
    if (isTemplate) {
      const template = config.templates[currentPage];
      if (template) {
        template[targetButtonKey] = clonedConfig;
        config.templates = { ...config.templates };
      }
    } else {
      const groupKey = config.page_groups[device.serial] ? device.serial : 'default';
      const page = config.page_groups[groupKey][currentPage];
      if (page) {
        page[targetButtonKey] = clonedConfig;
        config.page_groups = { ...config.page_groups };
      }
    }

    dropTargetIndex = null;
    draggedButtonIndex = null;

    // Auto-select the new button
    onButtonSelected(targetIndex);
  }
</script>

<div class="button-grid-container">
  <div class="device-info">
    {#if config}
      <DeviceSelector onDeviceSelected={onDeviceSelected || (() => {})} onRefresh={onRefresh || (() => {})} />
    {/if}
  </div>

  <div class="main-content">
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
        {@const isConfigured = hasConfig(buttonIndex) && !isInherited(buttonIndex) && !isButtonDefReference(buttonIndex)}
        {@const canDropHere = !isConfigured}
        {@const hasNavigation = !isEditMode && !isTemplate && findFirstJumpAction(buttonIndex) !== null}
        <button
          class="grid-button"
          class:selected={selectedButton === buttonIndex}
          class:configured={isConfigured}
          class:inherited={isInherited(buttonIndex)}
          class:button-def-reference={isButtonDefReference(buttonIndex)}
          class:has-icon={iconUrl !== null}
          class:dragging={draggedButtonIndex === buttonIndex}
          class:drop-target={dropTargetIndex === buttonIndex}
          class:navigatable={hasNavigation}
          class:non-navigatable={!isEditMode && !isTemplate && !hasNavigation}
          draggable={isConfigured && (isEditMode || isTemplate)}
          ondragstart={(e) => handleDragStart(e, buttonIndex)}
          ondragend={handleDragEnd}
          ondragover={(e) => handleDragOver(e, buttonIndex)}
          ondragleave={(e) => handleDragLeave(e, buttonIndex)}
          ondrop={(e) => handleDrop(e, buttonIndex)}
          onclick={() => handleButtonClick(buttonIndex)}
          title="Button {buttonIndex}{isConfigured ? ' (drag to copy)' : canDropHere ? ' (drop here to override)' : ''}"
        >
          {#if iconUrl}
            <img src={iconUrl} alt="Button {buttonIndex}" class="button-icon" draggable="false" />
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
  </div>
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

  .main-content {
    display: flex;
    flex-direction: column;
    flex: 1;
    width: 100%;
    align-items: center;
    justify-content: center;
  }

  .center-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
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

  /* Navigate mode styling */
  .grid-button.navigatable {
    border-color: #d0d0d0;
  }

  .grid-button.navigatable:hover {
    border-color: #e0e0e0;
  }

  .grid-button.non-navigatable {
    border-color: #555;
  }

  .grid-button.non-navigatable:hover {
    border-color: #777;
  }

  .grid-button.dragging {
    opacity: 0.5;
    cursor: grabbing;
  }

  .grid-button.drop-target {
    border-color: #4ec9b0;
    border-width: 3px;
    background: linear-gradient(135deg, rgba(78, 201, 176, 0.2) 0%, rgba(78, 201, 176, 0.1) 100%);
    box-shadow: 0 0 12px rgba(78, 201, 176, 0.5);
  }

  .grid-button.drop-target::after {
    content: '📋';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 32px;
    opacity: 0.7;
    pointer-events: none;
  }

  .grid-button[draggable="true"]:not(.dragging) {
    cursor: pointer;
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
