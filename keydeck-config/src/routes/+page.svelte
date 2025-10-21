<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { save, open } from '@tauri-apps/plugin-dialog';
  import DeviceSelector from "../lib/components/DeviceSelector.svelte";
  import Sidebar from "../lib/components/Sidebar.svelte";
  import ButtonGrid from "../lib/components/ButtonGrid.svelte";
  import ButtonEditor from "../lib/components/ButtonEditor.svelte";
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
  let isSaving = $state<boolean>(false);
  let lastSaveTime = $state<string>("");
  let hasUnsavedChanges = $state<boolean>(false);

  onMount(async () => {
    try {
      // Try to load existing config
      config = await invoke("load_config");

      // The config structure uses flattened page_groups, meaning page groups
      // are at the root level (not under a "page_groups" key)
      // We need to identify which root-level keys are page groups vs config fields
      const knownConfigFields = ['image_dir', 'templates', 'buttons', 'colors', 'services', 'macros', 'tick_time'];

      // Extract page_groups by filtering out known config fields
      const pageGroups: any = {};
      for (const key in config) {
        if (!knownConfigFields.includes(key)) {
          pageGroups[key] = config[key];
        }
      }

      // Store page_groups separately for easier access in components
      config.page_groups = pageGroups;

      // Ensure at least a default page group exists
      if (Object.keys(config.page_groups).length === 0) {
        console.log("No page groups found, initializing default...");
        config.page_groups = {
          default: {
            restore_mode: "main",
            main: {
              buttons: {}
            }
          }
        };
        // Also add to root level since it's flattened
        config.default = config.page_groups.default;
      }

      // Auto-select the main page on load
      selectInitialPage();
    } catch (e) {
      console.log("No existing config found, starting fresh", e);
      config = {
        tick_time: 2.0,
        page_groups: {
          default: {
            restore_mode: "main",
            main: {
              buttons: {}
            }
          }
        },
        default: {
          restore_mode: "main",
          main: {
            buttons: {}
          }
        }
      };

      // Auto-select the main page on load
      selectInitialPage();
    }
  });

  // Auto-select initial page based on device or default
  function selectInitialPage() {
    if (!config?.page_groups) return;

    const pageGroup = selectedDevice
      ? (config.page_groups[selectedDevice.serial] || config.page_groups.default)
      : config.page_groups.default;

    if (!pageGroup) return;

    // Try to get main_page from config, otherwise find first available page
    const knownFields = ['main_page', 'restore_mode', 'on_tick'];
    const mainPageName = pageGroup.main_page || 'main';
    const availablePages = Object.keys(pageGroup).filter(key => !knownFields.includes(key));

    // Select main_page if it exists, otherwise select first available page
    if (availablePages.includes(mainPageName)) {
      currentPage = mainPageName;
    } else if (availablePages.length > 0) {
      currentPage = availablePages[0];
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

  // Track config changes
  $effect(() => {
    if (config) {
      hasUnsavedChanges = true;
    }
  });

  function handleDeviceSelected(device: DeviceInfo) {
    console.log("Device selected:", device);
    selectedDevice = device;
    error = "";
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
  }

  function handleServiceSelected(serviceName: string | null) {
    currentService = serviceName;
    currentMacro = null;
    currentButtonDef = null;
    currentPage = "";
    currentTemplate = null;
    selectedButton = null;
  }

  function handleMacroSelected(macroName: string | null) {
    currentMacro = macroName;
    currentService = null;
    currentButtonDef = null;
    currentPage = "";
    currentTemplate = null;
    selectedButton = null;
  }

  function handleButtonDefSelected(buttonName: string | null) {
    currentButtonDef = buttonName;
    currentService = null;
    currentMacro = null;
    currentPage = "";
    currentTemplate = null;
    selectedButton = null;
  }

  async function reloadConfig() {
    if (hasUnsavedChanges) {
      if (!confirm("You have unsaved changes. Reload and discard them?")) {
        return;
      }
    }
    try {
      isSaving = true;
      error = "";
      const loadedConfig = await invoke("load_config");

      // Parse page_groups from flattened structure
      const knownConfigFields = ['image_dir', 'templates', 'buttons', 'colors', 'services', 'macros', 'tick_time'];
      const pageGroups: any = {};
      for (const key in loadedConfig) {
        if (!knownConfigFields.includes(key)) {
          pageGroups[key] = loadedConfig[key];
        }
      }

      // Store page_groups separately
      loadedConfig.page_groups = pageGroups;

      config = loadedConfig;
      hasUnsavedChanges = false;

      // Auto-select initial page after reload
      selectInitialPage();
    } catch (e) {
      error = `Failed to reload configuration: ${e}`;
    } finally {
      isSaving = false;
    }
  }

  async function saveConfig() {
    try {
      isSaving = true;
      error = "";
      await invoke("save_config", { config });
      hasUnsavedChanges = false;
      const now = new Date();
      lastSaveTime = now.toLocaleTimeString();
      alert("Configuration saved!");
    } catch (e) {
      error = `Failed to save configuration: ${e}`;
    } finally {
      isSaving = false;
    }
  }

  async function sendToDevice() {
    try {
      isSaving = true;
      error = "";
      await invoke("save_config", { config });
      await invoke("reload_keydeck");
      hasUnsavedChanges = false;
      const now = new Date();
      lastSaveTime = now.toLocaleTimeString();
      alert("Configuration sent to device and reloaded!");
    } catch (e) {
      error = `Failed to send to device: ${e}`;
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
        await invoke("export_config", { config, path: filePath });
        alert("Configuration exported successfully!");
      }
    } catch (e) {
      error = `Failed to export configuration: ${e}`;
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
        const importedConfig = await invoke("import_config", { path: filePath });
        config = importedConfig;
        alert("Configuration imported successfully!");
      }
    } catch (e) {
      error = `Failed to import configuration: ${e}`;
    }
  }
</script>

<div class="app-container">
  <header>
    <div class="header-left">
      <h1>KeyDeck</h1>
      {#if config}
        <DeviceSelector onDeviceSelected={handleDeviceSelected} onRefresh={reloadConfig} />
      {/if}
    </div>
    <div class="toolbar">
      {#if hasUnsavedChanges}
        <span class="unsaved-indicator">● Unsaved changes</span>
      {/if}
      {#if lastSaveTime}
        <span class="last-save">Last updated: {lastSaveTime}</span>
      {/if}
      <button onclick={saveConfig} disabled={isSaving} title="Save configuration to ~/.config/keydeck.yaml">
        Save
      </button>
      <button onclick={sendToDevice} disabled={isSaving} title="Save and send SIGHUP to reload device">
        Send to Device
      </button>
      <div style="width: 12px;"></div>
      <button onclick={importConfiguration} title="Import from YAML file">Import</button>
      <button onclick={exportConfiguration} title="Export to YAML file">Export</button>
    </div>
  </header>

  {#if error}
    <div class="error">{error}</div>
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
      onPageSelected={handlePageSelected}
      onTemplateSelected={handleTemplateSelected}
      onServiceSelected={handleServiceSelected}
      onMacroSelected={handleMacroSelected}
      onButtonDefSelected={handleButtonDefSelected}
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
        />
      {:else}
        <div class="placeholder">
          <p>{selectedDevice ? 'Select a page or template' : 'Select a device to get started'}</p>
        </div>
      {/if}
    </main>

    <!-- Right Sidebar: Button/Page/Template Config -->
    <aside class="properties-panel">
      {#if selectedButton !== null && selectedDevice && config}
        <ButtonEditor
          config={config}
          currentPage={currentPage}
          currentTemplate={currentTemplate}
          buttonIndex={selectedButton}
          deviceSerial={selectedDevice.serial}
          isTemplate={!!currentTemplate}
          onNavigateToTemplate={handleTemplateSelected}
        />
      {:else if currentPage && selectedDevice && config && !currentTemplate}
        <PageEditor
          config={config}
          pageName={currentPage}
          deviceSerial={selectedDevice.serial}
        />
      {:else if currentTemplate && config}
        <TemplateEditor
          config={config}
          templateName={currentTemplate}
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
                <button class="add-btn" onclick={() => {
                  const newParamName = prompt("Parameter name:");
                  if (newParamName && newParamName.trim()) {
                    if (!config.macros[currentMacro].params) {
                      config.macros[currentMacro].params = {};
                    }
                    config.macros[currentMacro].params[newParamName.trim()] = "";
                  }
                }}>+</button>
              </div>

              <div class="params-list">
                {#if config.macros[currentMacro].params && Object.keys(config.macros[currentMacro].params).length > 0}
                  {#each Object.entries(config.macros[currentMacro].params) as [paramName, paramValue]}
                    <div class="param-item">
                      <div class="param-content">
                        <div class="param-name-display">{paramName}</div>
                        <input
                          type="text"
                          value={paramValue}
                          oninput={(e) => {
                            config.macros[currentMacro].params[paramName] = e.currentTarget.value;
                          }}
                          placeholder="Default value"
                          class="param-value-input"
                        />
                      </div>
                      <button
                        class="param-delete-btn"
                        onclick={() => {
                          if (confirm(`Delete parameter "${paramName}"?`)) {
                            delete config.macros[currentMacro].params[paramName];
                            // Trigger reactivity
                            config.macros[currentMacro].params = { ...config.macros[currentMacro].params };
                          }
                        }}
                        title="Delete parameter"
                      >×</button>
                    </div>
                  {/each}
                {:else}
                  <p class="empty">No parameters defined</p>
                {/if}
              </div>
            </div>

            <!-- Macro Actions -->
            <div class="form-group">
              <label>Actions</label>
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
              <button onclick={() => {
                if (!config.macros[currentMacro].actions) {
                  config.macros[currentMacro].actions = [];
                }
                config.macros[currentMacro].actions.push({ exec: "" });
              }}>+ Add Action</button>
            </div>
          {/if}
        </div>
      {:else if currentButtonDef && config}
        <div class="editor-panel">
          <h2>Button Definition: {currentButtonDef}</h2>
          <p class="help">Button definitions are reusable button configurations</p>

          {#if config.buttons[currentButtonDef]}
            <!-- Icon, Text, Colors similar to ButtonEditor -->
            <div class="form-group">
              <label>Text</label>
              <input
                type="text"
                value={config.buttons[currentButtonDef].text || ""}
                oninput={(e) => {
                  if (!config.buttons[currentButtonDef].text) {
                    config.buttons[currentButtonDef].text = "";
                  }
                  config.buttons[currentButtonDef].text = e.currentTarget.value;
                }}
                placeholder="Button label"
              />
            </div>

            <div class="form-group">
              <label>Icon</label>
              <input
                type="text"
                value={config.buttons[currentButtonDef].icon || ""}
                oninput={(e) => {
                  config.buttons[currentButtonDef].icon = e.currentTarget.value || undefined;
                }}
                placeholder="icon.png or path to image"
              />
            </div>

            <div class="form-group">
              <label>Background</label>
              <input
                type="text"
                value={config.buttons[currentButtonDef].background || ""}
                oninput={(e) => {
                  config.buttons[currentButtonDef].background = e.currentTarget.value || undefined;
                }}
                placeholder="0xRRGGBB"
              />
            </div>

            <div class="form-group">
              <label>Text Color</label>
              <input
                type="text"
                value={config.buttons[currentButtonDef].text_color || ""}
                oninput={(e) => {
                  config.buttons[currentButtonDef].text_color = e.currentTarget.value || undefined;
                }}
                placeholder="0xRRGGBB"
              />
            </div>

            <div class="form-group">
              <label>Outline</label>
              <input
                type="text"
                value={config.buttons[currentButtonDef].outline || ""}
                oninput={(e) => {
                  config.buttons[currentButtonDef].outline = e.currentTarget.value || undefined;
                }}
                placeholder="0xRRGGBB"
              />
            </div>

            <!-- Button Definition Actions -->
            <div class="form-group">
              <label>Actions</label>
              <div class="actions-list">
                {#if config.buttons[currentButtonDef].actions && config.buttons[currentButtonDef].actions.length > 0}
                  {#each config.buttons[currentButtonDef].actions as action, i (currentButtonDef + '-' + i)}
                    <ActionEditor
                      {action}
                      index={i}
                      {config}
                      deviceSerial={selectedDevice?.serial}
                      onUpdate={(newAction) => {
                        config.buttons[currentButtonDef].actions[i] = newAction;
                      }}
                      onDelete={() => {
                        config.buttons[currentButtonDef].actions.splice(i, 1);
                      }}
                    />
                  {/each}
                {:else}
                  <p class="empty">No actions configured</p>
                {/if}
              </div>
              <button onclick={() => {
                if (!config.buttons[currentButtonDef].actions) {
                  config.buttons[currentButtonDef].actions = [];
                }
                config.buttons[currentButtonDef].actions.push({ exec: "" });
              }}>+ Add Action</button>
            </div>
          {/if}
        </div>
      {:else}
        <div class="placeholder">
          <p>Select a button, page, template, service, macro, or button definition to edit</p>
        </div>
      {/if}
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
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  header {
    background-color: #252526;
    padding: 12px 20px;
    border-bottom: 1px solid #3e3e42;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header-left {
    display: flex;
    gap: 20px;
    align-items: center;
  }

  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
  }

  .toolbar {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .live-preview-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background-color: #1e1e1e;
    border-radius: 4px;
    border: 1px solid #3e3e42;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    user-select: none;
  }

  .toggle-label input[type="checkbox"] {
    cursor: pointer;
    width: 16px;
    height: 16px;
  }

  .toggle-text {
    font-size: 13px;
    color: #cccccc;
  }

  .status-indicator {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 3px;
    font-weight: 500;
  }

  .status-indicator.active {
    background-color: #1a472a;
    color: #4ec9b0;
  }

  .status-indicator.saving {
    background-color: #5a3e1d;
    color: #dcdcaa;
  }

  .last-save {
    font-size: 12px;
    color: #888;
    font-style: italic;
  }

  .unsaved-indicator {
    font-size: 12px;
    color: #f48771;
    font-weight: 500;
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

  .error {
    background-color: #5a1d1d;
    color: #f48771;
    padding: 12px 20px;
    border-bottom: 1px solid #be1100;
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
  }

  .properties-panel {
    width: 300px;
    background-color: #252526;
    border-left: 1px solid #3e3e42;
    overflow-y: auto;
    padding: 16px;
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
    padding-bottom: 8px;
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 12px;
  }

  .section-header h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
  }

  .add-btn {
    width: 20px;
    height: 20px;
    padding: 0;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 14px;
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
    gap: 6px;
  }

  .param-item {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .param-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
    background-color: #3c3c3c;
    border: 1px solid #555;
    border-radius: 4px;
  }

  .param-name-display {
    font-size: 11px;
    font-weight: 600;
    color: #4ec9b0;
    text-transform: uppercase;
  }

  .param-value-input {
    padding: 4px 6px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 3px;
    font-size: 12px;
    font-family: monospace;
  }

  .param-value-input:focus {
    outline: none;
    border-color: #0e639c;
  }

  .param-delete-btn {
    width: 28px;
    height: 28px;
    padding: 0;
    background-color: #3c3c3c;
    color: #f48771;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .param-delete-btn:hover {
    background-color: #4a4a4a;
    color: #ff6b6b;
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
</style>
