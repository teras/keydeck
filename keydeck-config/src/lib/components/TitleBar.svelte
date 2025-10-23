<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  interface Props {
    hasUnsavedChanges?: boolean;
    lastSaveTime?: string;
    isSaving?: boolean;
    onSave?: () => void;
    onSend?: () => void;
    onImport?: () => void;
    onExport?: () => void;
  }

  let { hasUnsavedChanges = false, lastSaveTime = "", isSaving = false, onSave, onSend, onImport, onExport }: Props = $props();

  const appWindow = getCurrentWindow();

  let isDragging = false;

  async function minimizeWindow() {
    await appWindow.minimize();
  }

  async function maximizeWindow() {
    await appWindow.toggleMaximize();
  }

  async function closeWindow() {
    await appWindow.close();
  }

  async function startDrag(e: MouseEvent) {
    if (e.button === 0 && !isDragging) {
      isDragging = true;
      await appWindow.startDragging();
      isDragging = false;
    }
  }

  async function handleDoubleClick() {
    await appWindow.toggleMaximize();
  }
</script>

<div class="titlebar">
  <div
    class="titlebar-title"
    role="banner"
    onmousedown={startDrag}
    ondblclick={handleDoubleClick}
  >
    KeyDeck Configuration
  </div>

  <div class="titlebar-toolbar">
    {#if hasUnsavedChanges}
      <span class="unsaved-indicator" title="Unsaved changes">🔴</span>
    {/if}
    {#if lastSaveTime}
      <span class="last-save">Last: {lastSaveTime}</span>
    {/if}
    {#if onSave}
      <button class="toolbar-btn" onclick={onSave} disabled={isSaving} title="Save configuration to ~/.config/keydeck.yaml">
        Save
      </button>
    {/if}
    {#if onSend}
      <button class="toolbar-btn" onclick={onSend} disabled={isSaving} title="Save and send SIGHUP to reload device">
        Send
      </button>
    {/if}
    {#if onImport}
      <button class="toolbar-btn" onclick={onImport} title="Import from YAML file">
        Import
      </button>
    {/if}
    {#if onExport}
      <button class="toolbar-btn" onclick={onExport} title="Export to YAML file">
        Export
      </button>
    {/if}
  </div>

  <div class="titlebar-buttons">
    <button class="titlebar-button" onclick={minimizeWindow}>
      <svg width="12" height="12" viewBox="0 0 12 12">
        <rect y="5" width="12" height="1" fill="currentColor"/>
      </svg>
    </button>
    <button class="titlebar-button" onclick={maximizeWindow}>
      <svg width="12" height="12" viewBox="0 0 12 12">
        <rect x="1" y="1" width="10" height="10" stroke="currentColor" stroke-width="1" fill="none"/>
      </svg>
    </button>
    <button class="titlebar-button titlebar-close" onclick={closeWindow}>
      <svg width="12" height="12" viewBox="0 0 12 12">
        <line x1="1" y1="1" x2="11" y2="11" stroke="currentColor" stroke-width="1"/>
        <line x1="11" y1="1" x2="1" y2="11" stroke="currentColor" stroke-width="1"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: 48px;
    background: #2a2a2a;
    display: flex;
    justify-content: space-between;
    align-items: center;
    user-select: none;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 1000;
    border-bottom: 1px solid #3a3a3a;
  }

  .titlebar-title {
    padding-left: 12px;
    font-size: 18px;
    font-weight: 500;
    color: #e0e0e0;
    flex: 1;
    cursor: move;
  }

  .titlebar-buttons {
    display: flex;
    height: 100%;
  }

  .titlebar-button {
    width: 45px;
    height: 100%;
    border: none;
    background: transparent;
    color: #e0e0e0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.15s;
  }

  .titlebar-button:hover {
    background: #3a3a3a;
  }

  .titlebar-close:hover {
    background: #e81123;
  }

  .titlebar-button:active {
    opacity: 0.8;
  }

  .titlebar-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    height: 100%;
  }

  .unsaved-indicator {
    font-size: 10px;
    display: flex;
    align-items: center;
  }

  .last-save {
    font-size: 12px;
    color: #888;
  }

  .toolbar-btn {
    background-color: #0e639c;
    color: white;
    border: none;
    padding: 6px 14px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    transition: background-color 0.15s;
  }

  .toolbar-btn:hover {
    background-color: #1177bb;
  }

  .toolbar-btn:disabled {
    background-color: #555;
    cursor: not-allowed;
    opacity: 0.6;
  }

  .toolbar-btn:active:not(:disabled) {
    opacity: 0.8;
  }
</style>
