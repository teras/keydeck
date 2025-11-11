<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import LogViewer from "./LogViewer.svelte";

  interface Props {
    isEditMode: boolean;
    onHomeClick: () => void;
    onToggleMode: () => void;
    onError?: (message: string) => void;
  }

  let { isEditMode, onHomeClick, onToggleMode, onError }: Props = $props();

  interface DaemonStatus {
    running: boolean;
    pid: number | null;
    timestamp: number;
  }

  let daemonStatus = $state<DaemonStatus>({
    running: false,
    pid: null,
    timestamp: 0
  });
  let serviceEnabled = $state<boolean>(false);
  let statusCheckInterval: number | null = null;
  let showDaemonMenu = $state<boolean>(false);
  let showLogViewer = $state<boolean>(false);

  async function checkDaemonStatus() {
    try {
      const status = await invoke<DaemonStatus>("check_daemon_status");
      daemonStatus = status;

      // Also check if service is enabled
      const enabled = await invoke<boolean>("check_service_enabled");
      serviceEnabled = enabled;
    } catch (e) {
      console.error("Failed to check daemon status:", e);
    }
  }

  onMount(() => {
    // Initial check
    checkDaemonStatus();

    // Poll every 3 seconds
    statusCheckInterval = setInterval(checkDaemonStatus, 3000) as unknown as number;
  });

  onDestroy(() => {
    if (statusCheckInterval !== null) {
      clearInterval(statusCheckInterval);
    }
    document.removeEventListener('click', handleClickOutside);
  });

  function formatTimestamp(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    return date.toLocaleTimeString();
  }

  function toggleDaemonMenu() {
    showDaemonMenu = !showDaemonMenu;
  }

  function closeDaemonMenu() {
    showDaemonMenu = false;
  }

  function openLogViewer() {
    closeDaemonMenu();
    showLogViewer = true;
  }

  function closeLogViewer() {
    showLogViewer = false;
  }

  async function startDaemonService() {
    closeDaemonMenu();

    try {
      await invoke("start_daemon_service");
      // Wait a moment for the service to start
      setTimeout(() => checkDaemonStatus(), 1000);
    } catch (e) {
      console.error("Failed to start daemon as service:", e);
      if (onError) {
        // Extract the actual error message (Tauri wraps it)
        const errorMsg = typeof e === 'string' ? e : String(e);
        onError(errorMsg);
      }
    }
  }

  async function stopDaemonService() {
    closeDaemonMenu();

    try {
      await invoke("stop_daemon_service");
      // Wait a moment for the service to stop
      setTimeout(() => checkDaemonStatus(), 1000);
    } catch (e) {
      console.error("Failed to stop daemon service:", e);
      if (onError) {
        // Extract the actual error message (Tauri wraps it)
        const errorMsg = typeof e === 'string' ? e : String(e);
        onError(errorMsg);
      }
    }
  }

  async function reinstallDaemonService() {
    closeDaemonMenu();

    try {
      await invoke("reinstall_daemon_service");
      // Wait a moment for the service to start
      setTimeout(() => checkDaemonStatus(), 1000);
    } catch (e) {
      console.error("Failed to reinstall daemon service:", e);
      if (onError) {
        // Extract the actual error message (Tauri wraps it)
        const errorMsg = typeof e === 'string' ? e : String(e);
        onError(errorMsg);
      }
    }
  }

  // Close menu when clicking outside
  function handleClickOutside(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (!target.closest('.daemon-status')) {
      closeDaemonMenu();
    }
  }

  onMount(() => {
    // Initial check
    checkDaemonStatus();

    // Poll every 3 seconds
    statusCheckInterval = setInterval(checkDaemonStatus, 3000) as unknown as number;

    // Listen for clicks outside the menu
    document.addEventListener('click', handleClickOutside);
  });
</script>

<div class="helper-buttons">
  <div class="left-controls">
    <a
      class="helper-link home-link"
      onclick={onHomeClick}
      title="Jump to main page"
      role="button"
      tabindex="0"
    >
      🏠 Home
    </a>

    <div class="mode-toggle-container">
      <button
        class="mode-toggle-option"
        class:active={isEditMode}
        onclick={() => !isEditMode && onToggleMode()}
        title="Edit mode"
      >
        ✏️ Edit
      </button>
      <button
        class="mode-toggle-option"
        class:active={!isEditMode}
        onclick={() => isEditMode && onToggleMode()}
        title="Navigate mode"
      >
        🧭 Navigate
      </button>
    </div>
  </div>

  <div class="daemon-status">
    <span
      class="status-indicator"
      class:running={daemonStatus.running}
      class:stopped={!daemonStatus.running}
      onclick={toggleDaemonMenu}
      role="button"
      tabindex="0"
      title={daemonStatus.running
        ? `Daemon running (PID: ${daemonStatus.pid})\nLast checked: ${formatTimestamp(daemonStatus.timestamp)}\nClick for options`
        : `Daemon not running\nLast checked: ${formatTimestamp(daemonStatus.timestamp)}\nClick for options`
      }
    >
      <span class="status-dot"></span>
      <span class="status-text">
        {daemonStatus.running ? 'Running' : 'Stopped'}
      </span>
    </span>

    {#if showDaemonMenu}
      <div class="daemon-menu">
        <!-- Reinstall Service option (always available if service file exists) -->
        {#if serviceEnabled}
          <button class="menu-item" onclick={() => reinstallDaemonService()}>
            <span class="menu-icon">♻️</span>
            Reinstall Service
          </button>
        {/if}

        <!-- View Logs option (always available) -->
        <button class="menu-item" onclick={openLogViewer}>
          <span class="menu-icon">📋</span>
          View Logs
        </button>

        <!-- Show service options based on whether service is enabled -->
        {#if serviceEnabled}
          <button class="menu-item" onclick={() => stopDaemonService()}>
            <span class="menu-icon">🛑</span>
            Stop Service
          </button>
        {:else}
          <button class="menu-item" onclick={() => startDaemonService()}>
            <span class="menu-icon">▶️</span>
            Start as Service
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<!-- Log Viewer Modal -->
{#if showLogViewer}
  <LogViewer onClose={closeLogViewer} />
{/if}

<style>
  .helper-buttons {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    box-sizing: border-box;
  }

  .left-controls {
    display: flex;
    align-items: center;
    gap: 40px;
  }

  .daemon-status {
    position: relative;
    display: flex;
    align-items: center;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 13px;
    font-weight: 500;
    user-select: none;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .status-indicator:hover {
    background-color: #2a2a2a;
  }

  .status-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    transition: background-color 0.3s ease;
  }

  .status-indicator.running .status-dot {
    background-color: #4caf50;
    box-shadow: 0 0 8px rgba(76, 175, 80, 0.6);
  }

  .status-indicator.stopped .status-dot {
    background-color: #f44336;
    box-shadow: 0 0 8px rgba(244, 67, 54, 0.6);
  }

  .status-text {
    color: #cccccc;
    font-size: 13px;
  }

  .status-indicator.running .status-text {
    color: #4caf50;
  }

  .status-indicator.stopped .status-text {
    color: #f44336;
  }

  .helper-link {
    background: none;
    border: none;
    color: #cccccc;
    cursor: pointer;
    font-size: 16px;
    font-weight: 600;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: all 0.2s ease;
    padding: 6px 12px;
    user-select: none;
    border-radius: 4px;
  }

  .helper-link:hover {
    background-color: #2a2a2a;
    color: #4a9eff;
  }

  .helper-link:active {
    transform: scale(0.98);
  }

  .mode-toggle-container {
    display: flex;
    background-color: #2a2a2a;
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
  }

  .mode-toggle-option {
    padding: 6px 16px;
    background-color: #3c3c3c;
    border: none;
    color: #cccccc;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    border-radius: 4px;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 6px;
    user-select: none;
  }

  .mode-toggle-option:hover:not(.active) {
    color: #ffffff;
    background-color: #4a4a4a;
  }

  .mode-toggle-option.active {
    background-color: #0e639c;
    color: #ffffff;
    cursor: pointer;
    font-weight: 600;
  }

  .mode-toggle-option:active {
    transform: scale(0.98);
  }

  .daemon-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    right: 0;
    background-color: #2a2a2a;
    border: 1px solid #3c3c3c;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    min-width: 180px;
    overflow: hidden;
    z-index: 1000;
  }

  .menu-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: none;
    border: none;
    color: #cccccc;
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.2s ease;
    border-bottom: 1px solid #3c3c3c;
  }

  .menu-item:last-child {
    border-bottom: none;
  }

  .menu-item:hover {
    background-color: #3c3c3c;
    color: #ffffff;
  }

  .menu-item:active {
    background-color: #4a4a4a;
  }

  .menu-icon {
    font-size: 14px;
    width: 18px;
    display: inline-flex;
    justify-content: center;
  }
</style>
