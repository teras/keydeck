<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { save, open, ask } from '@tauri-apps/plugin-dialog';
  import TitleBar from "../lib/components/TitleBar.svelte";
  import DeviceSelector from "../lib/components/DeviceSelector.svelte";
  import Sidebar from "../lib/components/Sidebar.svelte";
  import ButtonGrid from "../lib/components/ButtonGrid.svelte";
  import ButtonEditor from "../lib/components/ButtonEditor.svelte";
  import ButtonDefEditor from "../lib/components/ButtonDefEditor.svelte";
  import PageEditor from "../lib/components/PageEditor.svelte";
  import TemplateEditor from "../lib/components/TemplateEditor.svelte";
  import ActionEditor from "../lib/components/ActionEditor.svelte";

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

  let selectedDevice = $state<DeviceInfo | null>(null);
  let currentPage = $state<string>("");
  let currentTemplate = $state<string | null>(null);
  let selectedButton = $state<number | null>(null);
  let currentService = $state<string | null>(null);
  let currentMacro = $state<string | null>(null);
  let currentButtonDef = $state<string | null>(null);
  let config = $state<any>(null);
  let error = $state<string>("");
  let errorDismissed = $state<boolean>(false);
  let isSaving = $state<boolean>(false);
  let lastSaveTime = $state<string>("");
  let hasUnsavedChanges = $state<boolean>(false);
  let isRightPanelOpen = $state<boolean>(true);
  let rightPanelWidth = $state<number>(300);
  let isResizingRightPanel = $state<boolean>(false);
  let sidebarToggleTab: ((tab: 'pages' | 'templates' | 'services' | 'macros' | 'buttons' | 'device' | 'global' | null) => void) | null = null;

  // Parameter management state
  let showAddParam = $state<boolean>(false);
  let newParamName = $state<string>("");
  let showParamMenu = $state<string | null>(null);
  let renamingParam = $state<string | null>(null);
  let renameParamName = $state<string>("");
  let lastAddedParam = $state<string | null>(null);
  let paramNameInput = $state<HTMLInputElement | undefined>();

  function setError(message: string) {
    error = message;
    errorDismissed = false;
  }

  function dismissError() {
    errorDismissed = true;
  }

  onMount(async () => {
    try {
      // Load config from default path
      // If no config file exists (first launch), backend returns a default empty config
      const loadedConfig = await invoke("load_config", { path: null });
      processLoadedConfig(loadedConfig);

      // Ensure at least a default page group exists
      if (Object.keys(config.page_groups).length === 0) {
        console.log("No page groups found, initializing default...");
        config.page_groups = {
          default: {
            restore_mode: "main",
            Main: {}
          }
        };
        // Also add to root level since it's flattened
        config.default = config.page_groups.default;
      }

      // Auto-select the main page on load
      selectInitialPage();
    } catch (e) {
      // This should only happen for actual errors (e.g., parse errors, permission issues)
      const errorMsg = String(e);
      console.error("Failed to load configuration:", e);
      setError(`Failed to load configuration: ${errorMsg}\n\nStarting with a fresh configuration. Please check your config file for errors.`);

      // Initialize with default config
      config = {
        tick_time: 2.0,
        page_groups: {
          default: {
            restore_mode: "main",
            Main: {}
          }
        },
        default: {
          restore_mode: "main",
          Main: {}
        }
      };

      // Auto-select the main page on load
      selectInitialPage();
    }
  });

  // Auto-select initial page based on device or default
  function selectInitialPage() {
    if (!config?.page_groups) return;

    // Take snapshot BEFORE any auto-creation to preserve user changes
    const snapshotBeforeAutoCreate = lastConfigSnapshot;

    // Get or create page group for the selected device
    let pageGroup = selectedDevice
      ? (config.page_groups[selectedDevice.serial] || config.page_groups.default)
      : config.page_groups.default;

    // Track if we're creating new structures (to avoid marking as unsaved changes)
    let createdNewStructures = false;

    // If the device doesn't have its own page group and we're falling back to default,
    // OR if the page group doesn't exist at all, create a default page group
    if (!pageGroup || (selectedDevice && !config.page_groups[selectedDevice.serial])) {
      // Create a new page group for this device with a default 'Main' page
      const groupKey = selectedDevice?.serial || 'default';
      config.page_groups[groupKey] = {
        restore_mode: "main",
        Main: {}
      };
      // Also update root level since it's flattened
      config[groupKey] = config.page_groups[groupKey];
      pageGroup = config.page_groups[groupKey];
      createdNewStructures = true;
    }

    // Try to get main_page from config, otherwise find first available page
    const knownFields = ['main_page', 'restore_mode', 'on_tick'];
    const mainPageName = pageGroup.main_page || 'Main';
    const availablePages = Object.keys(pageGroup).filter(key => !knownFields.includes(key));

    // If no pages exist, create a default 'Main' page
    if (availablePages.length === 0) {
      const groupKey = selectedDevice?.serial || 'default';
      config.page_groups[groupKey]['Main'] = {};
      config[groupKey]['Main'] = {};
      currentPage = 'Main';
      createdNewStructures = true;
    } else if (availablePages.includes(mainPageName)) {
      // Select main_page if it exists
      currentPage = mainPageName;
    } else {
      // Select first available page
      currentPage = availablePages[0];
    }

    // If we created new structures, check if ONLY the auto-created structures changed
    // This preserves any legitimate user changes made before this function was called
    if (createdNewStructures && snapshotBeforeAutoCreate !== undefined) {
      setTimeout(() => {
        // Get current state after our auto-creation
        const currentSnapshot = JSON.stringify(config);

        // If the snapshot hasn't been updated by user changes since we started,
        // update it to include our auto-created structures
        if (lastConfigSnapshot === snapshotBeforeAutoCreate) {
          lastConfigSnapshot = currentSnapshot;
          hasUnsavedChanges = false;
        }
        // Otherwise, user made changes in the meantime, so keep hasUnsavedChanges = true
      }, 0);
    } else if (createdNewStructures && snapshotBeforeAutoCreate === undefined) {
      // First load case - just update the snapshot
      setTimeout(() => {
        lastConfigSnapshot = JSON.stringify(config);
        hasUnsavedChanges = false;
      }, 0);
    }
  }

  // When device changes, auto-select initial page
  let previousDeviceSerial = $state<string | null>(null);
  $effect(() => {
    const currentSerial = selectedDevice?.serial || null;
    if (currentSerial !== previousDeviceSerial && config) {
      selectInitialPage();
      previousDeviceSerial = currentSerial;
    }
  });

  // Track config changes by serializing and comparing
  // undefined = no config loaded yet, so no warning needed
  let lastConfigSnapshot = $state<string | undefined>(undefined);

  $effect(() => {
    if (config) {
      const currentSnapshot = JSON.stringify(config);

      if (lastConfigSnapshot === undefined) {
        // First load - just save snapshot, don't mark as changed
        lastConfigSnapshot = currentSnapshot;
      } else {
        // Check if current state matches the saved state
        hasUnsavedChanges = currentSnapshot !== lastConfigSnapshot;
      }
    }
  });

  function handleDeviceSelected(device: DeviceInfo) {
    console.log("Device selected:", device);
    selectedDevice = device;
  }

  function handlePageSelected(pageName: string) {
    currentPage = pageName;
    currentTemplate = null;
    selectedButton = null;
    currentService = null;
    currentMacro = null;
    currentButtonDef = null;
  }

  function handleTemplateSelected(templateName: string | null, keepButtonSelection: boolean = false) {
    currentTemplate = templateName;
    currentPage = "";
    currentService = null;
    currentMacro = null;
    currentButtonDef = null;
    if (!keepButtonSelection) {
      selectedButton = null;
    }
  }

  function handleButtonSelected(buttonIndex: number) {
    selectedButton = buttonIndex;
    // Clear left panel selections to show only button is selected
    currentService = null;
    currentMacro = null;
    currentButtonDef = null;
    // Open right panel to show button properties
    isRightPanelOpen = true;
  }

  function handleServiceSelected(serviceName: string | null) {
    currentService = serviceName;
    currentMacro = null;
    currentButtonDef = null;
    selectedButton = null;
    // Open right panel to show service properties
    if (serviceName) isRightPanelOpen = true;
  }

  function handleMacroSelected(macroName: string | null) {
    currentMacro = macroName;
    currentService = null;
    currentButtonDef = null;
    selectedButton = null;
    // Open right panel to show macro properties
    if (macroName) isRightPanelOpen = true;
  }

  function handleButtonDefSelected(buttonName: string | null) {
    currentButtonDef = buttonName;
    currentService = null;
    currentMacro = null;
    selectedButton = null;
    // Open right panel to show button definition properties
    if (buttonName) isRightPanelOpen = true;
  }

  function handleButtonDefNavigate(buttonDefName: string, keepButtonSelection: boolean = false) {
    // Switch to button definitions tab in sidebar
    if (sidebarToggleTab) {
      sidebarToggleTab('buttons');
    }
    // Clear button selection and page/template context to show button definition
    selectedButton = null;
    currentTemplate = null;
    currentPage = "";
    // Select the button definition
    currentButtonDef = buttonDefName;
    currentService = null;
    currentMacro = null;
  }

  function handlePageTitleClicked() {
    // Clear button selection
    selectedButton = null;

    // Open the appropriate sidebar tab
    if (sidebarToggleTab) {
      if (currentTemplate) {
        sidebarToggleTab('templates');
      } else if (currentPage) {
        sidebarToggleTab('pages');
      }
    }
  }

  // Helper function to process loaded config (shared between reload and import)
  // preserveSelection: true for reload (from default config), false for import (from arbitrary file)
  function processLoadedConfig(loadedConfig: any, preserveSelection: boolean = false) {
    // Save current selection state before reloading
    const savedTemplate = preserveSelection ? currentTemplate : null;
    const savedPage = preserveSelection ? currentPage : null;

    // Backend uses #[serde(flatten)] on page_groups, which means when serializing to JSON,
    // page groups are at the root level. We need to extract them into a separate property
    // for easier frontend access while keeping the flattened structure for saving.
    const knownConfigFields = ['image_dir', 'templates', 'buttons', 'colors', 'services', 'macros', 'tick_time', 'brightness'];
    const pageGroups: any = {};

    for (const key in loadedConfig) {
      if (!knownConfigFields.includes(key)) {
        pageGroups[key] = loadedConfig[key];
      }
    }

    // Add page_groups as a convenience property for frontend
    loadedConfig.page_groups = pageGroups;

    config = loadedConfig;

    // When reloading from default config file (preserveSelection=true), update the snapshot
    // When importing (preserveSelection=false), compare against the snapshot to detect changes
    const newConfigSnapshot = JSON.stringify(config);
    if (preserveSelection) {
      hasUnsavedChanges = false;
      lastConfigSnapshot = newConfigSnapshot;
    } else {
      // Importing: check if different from saved config
      // If lastConfigSnapshot is undefined, this is initial load - no warning needed
      if (lastConfigSnapshot === undefined) {
        hasUnsavedChanges = false;
        lastConfigSnapshot = newConfigSnapshot;
      } else {
        hasUnsavedChanges = newConfigSnapshot !== lastConfigSnapshot;
      }
    }

    // Restore previous selection if requested, otherwise auto-select initial page
    if (preserveSelection) {
      if (savedTemplate !== null) {
        // Was viewing a template
        if (config.templates && config.templates[savedTemplate]) {
          // Template still exists in new config
          currentTemplate = savedTemplate;
          currentPage = ""; // Clear page selection
        } else {
          // Template no longer exists, fall back to pages and select main page
          currentTemplate = null;
          selectInitialPage();
        }
      } else if (savedPage) {
        // Was viewing a page
        const pageGroup = selectedDevice
          ? (config.page_groups[selectedDevice.serial] || config.page_groups.default)
          : config.page_groups.default;

        if (pageGroup && pageGroup[savedPage]) {
          // Page still exists in new config
          currentPage = savedPage;
        } else {
          // Page no longer exists, fall back to main page
          selectInitialPage();
        }
      } else {
        // No previous selection, use default
        selectInitialPage();
      }
    } else {
      // Auto-select initial page after loading
      selectInitialPage();
    }
  }

  async function reloadConfig() {
    // Check confirmation FIRST, before doing anything
    if (hasUnsavedChanges) {
      const confirmed = await ask(
        "You have unsaved changes. Reload and discard them?",
        {
          title: "Confirm Reload",
          kind: "warning"
        }
      );
      if (!confirmed) {
        // User cancelled - do nothing
        return;
      }
    }

    // Only proceed with reload after user confirms
    isSaving = true;
    try {
      const loadedConfig = await invoke("load_config", { path: null });
      processLoadedConfig(loadedConfig, true); // Preserve current selection, reload from default config
      // Only dismiss error after successful reload
      dismissError();
    } catch (e) {
      setError(`Failed to reload configuration: ${e}`);
    } finally {
      isSaving = false;
    }
  }

  async function sendToDevice() {
    try {
      isSaving = true;

      // Sync page_groups changes back to root level before saving
      // The UI modifies config.page_groups, but the backend expects them at root level
      if (config.page_groups) {
        for (const [groupName, groupData] of Object.entries(config.page_groups)) {
          config[groupName] = groupData;
        }
      }

      // Remove the frontend-only page_groups property before sending to backend
      const { page_groups, ...backendConfig } = config;
      await invoke("save_config", { config: backendConfig });
      await invoke("reload_keydeck");
      // Only dismiss error after successful send
      dismissError();
      hasUnsavedChanges = false;
      lastConfigSnapshot = JSON.stringify(config);
      const now = new Date();
      lastSaveTime = now.toLocaleTimeString();
    } catch (e) {
      setError(`Failed to send to device: ${e}`);
    } finally {
      isSaving = false;
    }
  }

  async function exportConfiguration() {
    try {
      const filePath = await save({
        defaultPath: 'keydeck.yaml',
        filters: [{
          name: 'YAML',
          extensions: ['yaml', 'yml']
        }]
      });

      if (filePath) {
        // Sync page_groups changes back to root level before exporting
        if (config.page_groups) {
          for (const [groupName, groupData] of Object.entries(config.page_groups)) {
            config[groupName] = groupData;
          }
        }

        // Remove the frontend-only page_groups property before sending to backend
        const { page_groups, ...backendConfig } = config;
        await invoke("export_config", { config: backendConfig, path: filePath });
      }
    } catch (e) {
      setError(`Failed to export configuration: ${e}`);
    }
  }

  async function importConfiguration() {
    try {
      const filePath = await open({
        multiple: false,
        filters: [{
          name: 'YAML',
          extensions: ['yaml', 'yml']
        }]
      });

      if (filePath) {
        const importedConfig = await invoke("load_config", { path: filePath });
        processLoadedConfig(importedConfig, false); // Don't preserve selection, import from arbitrary file
      }
    } catch (e) {
      setError(`Failed to import configuration: ${e}`);
    }
  }

  // Parameter management functions
  function toggleAddParam() {
    showAddParam = !showAddParam;
    if (showAddParam) {
      setTimeout(() => paramNameInput?.focus(), 0);
    }
  }

  function addParam() {
    if (!newParamName.trim() || !currentMacro) return;

    if (!config.macros[currentMacro].params) {
      config.macros[currentMacro].params = {};
    }

    const paramName = newParamName.trim();
    if (config.macros[currentMacro].params[paramName]) {
      alert(`Parameter "${paramName}" already exists!`);
      return;
    }

    config.macros[currentMacro].params[paramName] = "";
    lastAddedParam = paramName;
    newParamName = "";
    showAddParam = false;

    // Focus on the newly added parameter's value input
    setTimeout(() => {
      const paramInput = document.querySelector(`input[data-param-name="${paramName}"]`) as HTMLInputElement;
      if (paramInput) {
        paramInput.focus();
        paramInput.setSelectionRange(paramInput.value.length, paramInput.value.length);
      }
      lastAddedParam = null;
    }, 50);
  }

  function deleteParam(paramName: string) {
    if (!currentMacro) return;
    showParamMenu = null;

    if (confirm(`Delete parameter "${paramName}"?`)) {
      delete config.macros[currentMacro].params[paramName];
      config.macros[currentMacro].params = { ...config.macros[currentMacro].params };
    }
  }

  function duplicateParam(paramName: string) {
    if (!currentMacro || !config.macros[currentMacro].params) return;

    const original = config.macros[currentMacro].params[paramName];
    let newName = `${paramName}_copy`;
    let counter = 1;
    while (config.macros[currentMacro].params[newName]) {
      newName = `${paramName}_copy${counter}`;
      counter++;
    }

    config.macros[currentMacro].params[newName] = original;
    config.macros[currentMacro].params = { ...config.macros[currentMacro].params };
    lastAddedParam = newName;
    showParamMenu = null;

    // Focus on the duplicated parameter's value input
    setTimeout(() => {
      const paramInput = document.querySelector(`input[data-param-name="${newName}"]`) as HTMLInputElement;
      if (paramInput) {
        paramInput.focus();
        paramInput.setSelectionRange(paramInput.value.length, paramInput.value.length);
      }
      lastAddedParam = null;
    }, 50);
  }

  function startRenameParam(paramName: string) {
    renamingParam = paramName;
    renameParamName = paramName;
    showParamMenu = null;
  }

  function renameParam(oldName: string) {
    if (!currentMacro || !renameParamName.trim() || renameParamName === oldName) {
      renameParamName = "";
      renamingParam = null;
      return;
    }

    if (config.macros[currentMacro].params[renameParamName]) {
      alert(`Parameter "${renameParamName}" already exists!`);
      renameParamName = "";
      renamingParam = null;
      return;
    }

    const newParams: any = {};
    for (const key of Object.keys(config.macros[currentMacro].params)) {
      if (key === oldName) {
        newParams[renameParamName] = config.macros[currentMacro].params[oldName];
      } else {
        newParams[key] = config.macros[currentMacro].params[key];
      }
    }
    config.macros[currentMacro].params = newParams;

    renameParamName = "";
    renamingParam = null;
  }

  // Click-outside handler for param menu
  $effect(() => {
    if (showParamMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.param-menu') && !target.closest('.param-menu-btn')) {
          showParamMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  // Right panel resize handlers
  let resizeStartX = $state<number>(0);
  let resizeStartWidth = $state<number>(0);

  function startResizeRightPanel(event: MouseEvent) {
    isResizingRightPanel = true;
    resizeStartX = event.clientX;
    resizeStartWidth = rightPanelWidth;
    event.preventDefault();
  }

  $effect(() => {
    if (isResizingRightPanel) {
      const handleMouseMove = (event: MouseEvent) => {
        const deltaX = resizeStartX - event.clientX;
        const newWidth = resizeStartWidth + deltaX;
        // Min width: 200px, Max width: 600px
        rightPanelWidth = Math.max(200, Math.min(600, newWidth));
      };

      const handleMouseUp = () => {
        isResizingRightPanel = false;
      };

      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'ew-resize';
      document.body.style.userSelect = 'none';

      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };
    }
  });
</script>

<TitleBar
  hasUnsavedChanges={hasUnsavedChanges}
  lastSaveTime={lastSaveTime}
  isSaving={isSaving}
  onSend={sendToDevice}
  onImport={importConfiguration}
  onExport={exportConfiguration}
/>

<div class="app-container">
  {#if error && !errorDismissed}
    <div class="error-banner">
      <div class="error-content">
        <div class="error-icon">⚠️</div>
        <div class="error-message">{error}</div>
        <button class="error-dismiss" onclick={dismissError} title="Dismiss error">✕</button>
      </div>
    </div>
  {/if}

  <div class="main-layout">
    <!-- Tabbed Sidebar -->
    <Sidebar
      config={config}
      selectedDevice={selectedDevice}
      currentPage={currentPage}
      currentTemplate={currentTemplate}
      currentService={currentService}
      currentMacro={currentMacro}
      currentButtonDef={currentButtonDef}
      selectedButton={selectedButton}
      onPageSelected={handlePageSelected}
      onTemplateSelected={handleTemplateSelected}
      onServiceSelected={handleServiceSelected}
      onMacroSelected={handleMacroSelected}
      onButtonDefSelected={handleButtonDefSelected}
      openTab={(fn) => sidebarToggleTab = fn}
    />

    <!-- Center: Button Grid -->
    <main class="center-panel">
      {#if selectedDevice && config && (currentPage || currentTemplate)}
        <ButtonGrid
          device={selectedDevice}
          config={config}
          currentPage={currentPage || currentTemplate}
          selectedButton={selectedButton}
          onButtonSelected={handleButtonSelected}
          isTemplate={!!currentTemplate}
          pageName={currentPage || currentTemplate}
          onPageTitleClicked={handlePageTitleClicked}
          onDeviceSelected={handleDeviceSelected}
          onRefresh={reloadConfig}
        />
      {:else}
        <div class="placeholder">
          {#if config}
            <DeviceSelector onDeviceSelected={handleDeviceSelected} onRefresh={reloadConfig} />
            <p style="margin-top: 20px;">{selectedDevice ? 'Select a page or template' : 'Select a device to get started'}</p>
          {:else}
            <p>Loading configuration...</p>
          {/if}
        </div>
      {/if}

      <!-- Right panel toggle button -->
      <button
        class="panel-toggle-btn right-toggle"
        onclick={() => isRightPanelOpen = !isRightPanelOpen}
        title={isRightPanelOpen ? "Hide properties panel" : "Show properties panel"}
      >
        {isRightPanelOpen ? '❯' : '❮'}
      </button>
    </main>

    <!-- Right Sidebar: Button/Page/Template Config -->
    <aside class="properties-panel" class:closed={!isRightPanelOpen} style="width: {isRightPanelOpen ? rightPanelWidth : 0}px;">
      {#if isRightPanelOpen}
        <div
          class="resize-handle"
          role="separator"
          aria-label="Resize properties panel"
          onmousedown={startResizeRightPanel}
        ></div>
      {/if}
      <div class="properties-content">
      {#if selectedButton !== null && selectedDevice && config}
        <ButtonEditor
          bind:config={config}
          currentPage={currentPage}
          currentTemplate={currentTemplate}
          buttonIndex={selectedButton}
          deviceSerial={selectedDevice.serial}
          isTemplate={!!currentTemplate}
          onNavigateToTemplate={handleTemplateSelected}
          onNavigateToButtonDef={handleButtonDefNavigate}
        />
      {:else if currentService && config}
        <div class="editor-panel">
          <h2>Service: {currentService}</h2>
          <div class="service-config">
            {#if typeof config.services[currentService] === 'string'}
              <div class="form-group">
                <label>Command</label>
                <textarea bind:value={config.services[currentService]} rows="3" placeholder='echo "your data"'></textarea>
                <p class="help">Shell command to execute (uses default interval: 1s, timeout: 5s)</p>
              </div>
            {:else}
              <div class="form-group">
                <label>Command</label>
                <textarea bind:value={config.services[currentService].exec} rows="3" placeholder='echo "your data"'></textarea>
              </div>
              <div class="form-group">
                <label>Interval (seconds)</label>
                <input type="number" bind:value={config.services[currentService].interval} min="0.1" step="0.1" />
                <p class="help">How often to run the command</p>
              </div>
              <div class="form-group">
                <label>Timeout (seconds)</label>
                <input type="number" bind:value={config.services[currentService].timeout} min="1" step="1" />
                <p class="help">Maximum time to wait for command completion</p>
              </div>
            {/if}
          </div>
        </div>
      {:else if currentMacro && config}
        <div class="editor-panel">
          <h2>Macro: {currentMacro}</h2>
          <p class="help">Macros contain reusable action sequences with optional parameters</p>

          {#if config.macros[currentMacro]}
            <!-- Macro Parameters -->
            <div class="macro-section">
              <div class="section-header">
                <h3>Parameters</h3>
                <button class="add-btn" onclick={toggleAddParam}>+</button>
              </div>

              {#if showAddParam}
                <div class="add-param">
                  <input
                    type="text"
                    bind:this={paramNameInput}
                    bind:value={newParamName}
                    placeholder="Parameter name"
                    onkeydown={(e) => e.key === 'Enter' && addParam()}
                  />
                  <button onclick={addParam} title="Add">✓</button>
                  <button onclick={() => showAddParam = false} title="Cancel">✕</button>
                </div>
              {/if}

              <div class="separator"></div>

              <div class="params-list">
                {#if config.macros[currentMacro].params && Object.keys(config.macros[currentMacro].params).length > 0}
                  {#each Object.entries(config.macros[currentMacro].params) as [paramName, paramValue]}
                    <div class="param-row">
                      {#if renamingParam === paramName}
                        <input
                          type="text"
                          bind:value={renameParamName}
                          class="rename-param-input"
                          onkeydown={(e) => {
                            if (e.key === 'Enter') renameParam(paramName);
                            if (e.key === 'Escape') { renameParamName = ""; renamingParam = null; }
                          }}
                          onblur={() => renameParam(paramName)}
                          onmousedown={(e) => e.stopPropagation()}
                          autofocus
                        />
                      {:else}
                        <div class="param-item">
                          <div class="param-info">
                            <span class="param-name">{paramName}</span>
                            <input
                              type="text"
                              value={paramValue}
                              data-param-name={paramName}
                              oninput={(e) => {
                                config.macros[currentMacro].params[paramName] = e.currentTarget.value;
                              }}
                              placeholder="Default value"
                              class="param-value"
                            />
                          </div>
                        </div>
                        <button
                          class="param-menu-btn"
                          onclick={(e) => {
                            e.stopPropagation();
                            showParamMenu = showParamMenu === paramName ? null : paramName;
                          }}
                        >
                          ⋮
                        </button>
                      {/if}

                      {#if showParamMenu === paramName}
                        <div class="param-menu">
                          <button onclick={(e) => { e.stopPropagation(); startRenameParam(paramName); }}>✏️ Rename</button>
                          <button onclick={(e) => { e.stopPropagation(); duplicateParam(paramName); }}>📋 Duplicate</button>
                          <button class="danger" onclick={(e) => { e.stopPropagation(); deleteParam(paramName); }}>🗑️ Delete</button>
                        </div>
                      {/if}
                    </div>
                  {/each}
                {:else}
                  <p class="empty">No parameters defined</p>
                {/if}
              </div>
            </div>

            <!-- Macro Actions -->
            <div class="form-group">
              <div class="actions-header">
                <label>Actions</label>
                <button class="add-btn" onclick={() => {
                  if (!config.macros[currentMacro].actions) {
                    config.macros[currentMacro].actions = [];
                  }
                  config.macros[currentMacro].actions.push({ jump: "" });
                }}>+</button>
              </div>
              <div class="actions-list">
                {#if config.macros[currentMacro].actions && config.macros[currentMacro].actions.length > 0}
                  {#each config.macros[currentMacro].actions as action, i (currentMacro + '-' + i)}
                    <ActionEditor
                      {action}
                      index={i}
                      {config}
                      deviceSerial={selectedDevice?.serial}
                      onUpdate={(newAction) => {
                        config.macros[currentMacro].actions[i] = newAction;
                      }}
                      onDelete={() => {
                        config.macros[currentMacro].actions.splice(i, 1);
                      }}
                    />
                  {/each}
                {:else}
                  <p class="empty">No actions configured</p>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      {:else if currentButtonDef && config}
        <ButtonDefEditor
          bind:config={config}
          buttonDefName={currentButtonDef}
        />
      {:else if currentPage && selectedDevice && config && !currentTemplate}
        <PageEditor
          bind:config={config}
          pageName={currentPage}
          deviceSerial={selectedDevice.serial}
        />
      {:else if currentTemplate && config}
        <TemplateEditor
          bind:config={config}
          templateName={currentTemplate}
        />
      {:else}
        <div class="placeholder">
          <p>Select a button, page, template, service, macro, or button definition to edit</p>
        </div>
      {/if}
      </div>
    </aside>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    background-color: #1e1e1e;
    color: #d4d4d4;
  }

  :global(select) {
    -webkit-appearance: none;
    -moz-appearance: none;
    appearance: none;
    background-color: #2a2a2a !important;
    color: #cccccc !important;
    background-image: url("data:image/svg+xml;charset=UTF-8,%3csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23cccccc' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3e%3cpolyline points='6 9 12 15 18 9'%3e%3c/polyline%3e%3c/svg%3e");
    background-repeat: no-repeat;
    background-position: right 8px center;
    background-size: 16px;
    padding-right: 32px !important;
  }

  :global(select option) {
    background-color: #2a2a2a !important;
    color: #cccccc !important;
  }

  .app-container {
    height: calc(100vh - 48px);
    margin-top: 48px;
    display: flex;
    flex-direction: column;
    flex: 1;
  }


  button {
    background-color: #0e639c;
    color: white;
    border: none;
    padding: 6px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  button:hover {
    background-color: #1177bb;
  }

  button:disabled {
    background-color: #555;
    cursor: not-allowed;
    opacity: 0.6;
  }

  .error-banner {
    background-color: #5a1d1d;
    border-bottom: 2px solid #be1100;
    animation: slideDown 0.3s ease-out;
    z-index: 1000;
  }

  @keyframes slideDown {
    from {
      transform: translateY(-100%);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .error-content {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 16px 20px;
    max-width: 100%;
  }

  .error-icon {
    font-size: 20px;
    flex-shrink: 0;
    line-height: 1;
  }

  .error-message {
    flex: 1;
    color: #f48771;
    font-size: 14px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .error-dismiss {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    padding: 0;
    background-color: transparent;
    color: #f48771;
    border: 1px solid #be1100;
    border-radius: 3px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s, color 0.2s;
  }

  .error-dismiss:hover {
    background-color: #be1100;
    color: white;
  }

  .main-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .center-panel {
    flex: 1;
    background-color: #1e1e1e;
    overflow: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    position: relative;
  }

  .properties-panel {
    position: relative;
    background-color: #252526;
    border-left: 1px solid #3e3e42;
    transition: width 0.2s ease-out, opacity 0.2s ease-out;
    display: flex;
    flex-direction: row;
  }

  .properties-panel.closed {
    opacity: 0;
    pointer-events: none;
  }

  .properties-content {
    flex: 1;
    overflow-y: auto;
    padding: 16px 16px 16px 2px;
  }

  .resize-handle {
    width: 16px;
    cursor: ew-resize;
    background-color: transparent;
    transition: background-color 0.2s;
    flex-shrink: 0;
    position: relative;
  }

  .resize-handle::after {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background-color: transparent;
    transition: background-color 0.2s;
  }

  .resize-handle:hover::after {
    background-color: #0e639c;
  }

  .resize-handle:active::after {
    background-color: #1177bb;
  }

  .editor-panel h2 {
    margin: 0 0 16px 0;
    font-size: 16px;
    color: #cccccc;
    padding-bottom: 12px;
    border-bottom: 1px solid #3e3e42;
  }

  .editor-panel .help {
    margin: 0 0 16px 0;
    font-size: 11px;
    color: #666;
    font-style: italic;
  }

  .service-config, .editor-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
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

  .form-group label {
    font-size: 12px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
  }

  .form-group textarea,
  .form-group input[type="number"] {
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
    font-family: monospace;
  }

  .form-group textarea {
    resize: vertical;
    min-height: 60px;
  }

  .form-group input:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: #0e639c;
  }

  .macro-section {
    margin-bottom: 20px;
  }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 12px;
  }

  .separator {
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 16px;
  }

  .section-header h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
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

  .params-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .param-row {
    position: relative;
    display: flex;
    gap: 4px;
  }

  .param-item {
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

  .param-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .param-name {
    font-size: 12px;
    font-weight: 600;
    color: #cccccc;
  }

  .param-value {
    padding: 6px 8px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 3px;
    font-size: 12px;
  }

  .param-value:focus {
    outline: none;
    border-color: #0e639c;
  }

  .param-menu-btn {
    width: 28px;
    padding: 4px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
  }

  .param-menu-btn:hover {
    background-color: #4a4a4a;
    color: #ff6b6b;
  }

  .add-param {
    display: flex;
    gap: 4px;
    margin-top: 12px;
    margin-bottom: 12px;
  }

  .add-param input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .add-param button {
    padding: 6px 12px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-param button:first-of-type {
    background-color: #2d7d46;
  }

  .add-param button:first-of-type:hover {
    background-color: #3a9d5a;
  }

  .add-param button:last-child {
    background-color: #7a2d2d;
  }

  .add-param button:last-child:hover {
    background-color: #9a3d3d;
  }

  .rename-param-input {
    flex: 1;
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #0e639c;
    border-radius: 4px;
    font-size: 12px;
  }

  .rename-param-input:focus {
    outline: none;
    border-color: #1177bb;
  }

  .param-menu {
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

  .param-menu button {
    padding: 8px 12px;
    background: none;
    color: #cccccc;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }

  .param-menu button:hover {
    background-color: #3c3c3c;
  }

  .param-menu button.danger {
    color: #f48771;
  }

  .param-menu button.danger:hover {
    background-color: #5a1d1d;
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

  .placeholder {
    text-align: center;
    color: #6a6a6a;
  }

  .panel-toggle-btn {
    position: absolute;
    background-color: #2d2d30;
    color: #cccccc;
    border: 1px solid #3e3e42;
    border-radius: 4px 0 0 4px;
    padding: 12px 6px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    z-index: 10;
    transition: background-color 0.2s, color 0.2s;
  }

  .panel-toggle-btn:hover {
    background-color: #3e3e42;
    color: #ffffff;
  }

  .right-toggle {
    top: 50%;
    right: 0;
    transform: translateY(-50%);
  }
</style>
