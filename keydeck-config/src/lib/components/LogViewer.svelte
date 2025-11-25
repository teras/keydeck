<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  interface LogEntry {
    id: number;
    timestamp: string;
    level: "info" | "warn" | "error";
    message: string;
    raw: string;
  }

  let logs = $state<LogEntry[]>([]);
  let isPaused = $state<boolean>(false);
  let filter = $state<string>("all");
  let searchQuery = $state<string>("");
  let scrollContainer = $state<HTMLDivElement | undefined>();
  let isAtBottom = $state<boolean>(true);
  let autoScroll = $state<boolean>(true);
  let logCounter = 0;
  let unlisten: (() => void) | null = null;

  function parseLogEntry(jsonLine: string): LogEntry | null {
    try {
      const data = JSON.parse(jsonLine);
      const message = data.MESSAGE || "";
      const timestamp = new Date(parseInt(data.__REALTIME_TIMESTAMP) / 1000).toLocaleTimeString();

      // Determine log level from systemd PRIORITY and message content
      const priority = parseInt(data.PRIORITY || "6");
      let level: "info" | "warn" | "error" = "info";

      // Check for "ERROR: " prefix in message
      if (message.startsWith("ERROR: ")) {
        level = "error";
      } else if (priority <= 3) {
        // Systemd priority 0-3 = error (stderr)
        level = "error";
      } else if (priority === 4) {
        // Systemd priority 4 = warning (stderr)
        level = "warn";
      } else if (priority >= 5) {
        // Systemd priority 5+ = info (stdout), but detect warnings from content
        if (message.includes("Image not found") ||
            message.includes("not supported") ||
            message.includes("Warning") ||
            message.includes("Failed to initialize")) {
          level = "warn";
        } else {
          level = "info";
        }
      }

      return {
        id: logCounter++,
        timestamp,
        level,
        message,
        raw: jsonLine
      };
    } catch (e) {
      console.error("Failed to parse log entry:", e);
      return null;
    }
  }

  function addLogEntry(jsonLine: string) {
    if (isPaused) return;

    const entry = parseLogEntry(jsonLine);
    if (!entry) return;

    logs = [...logs, entry];

    // Keep only last 1000 entries to prevent memory issues
    if (logs.length > 1000) {
      logs = logs.slice(-1000);
    }

    // Auto-scroll to bottom if enabled
    if (autoScroll && scrollContainer) {
      requestAnimationFrame(() => {
        if (scrollContainer) {
          scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
      });
    }
  }

  function handleScroll() {
    if (!scrollContainer) return;

    const threshold = 50;
    const isNearBottom = scrollContainer.scrollHeight - scrollContainer.scrollTop - scrollContainer.clientHeight < threshold;

    isAtBottom = isNearBottom;
    autoScroll = isNearBottom;
  }

  function scrollToBottom() {
    if (scrollContainer) {
      scrollContainer.scrollTop = scrollContainer.scrollHeight;
      autoScroll = true;
    }
  }

  function togglePause() {
    isPaused = !isPaused;
  }

  function clearLogs() {
    logs = [];
  }

  function copyLogs() {
    const text = filteredLogs.map(log => `[${log.timestamp}] ${log.message}`).join('\n');
    navigator.clipboard.writeText(text);
  }

  let filteredLogs = $derived.by(() => {
    let result = logs;

    // Filter by level
    if (filter === "errors") {
      result = result.filter(log => log.level === "error");
    } else if (filter === "warnings") {
      result = result.filter(log => log.level === "warn" || log.level === "error");
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(log =>
        log.message.toLowerCase().includes(query) ||
        log.timestamp.toLowerCase().includes(query)
      );
    }

    return result;
  });

  onMount(async () => {
    try {
      // Start streaming logs
      await invoke("stream_journal_logs");

      // Listen for log entries
      unlisten = await listen<string>("log-entry", (event) => {
        addLogEntry(event.payload);
      });

      console.log("Log viewer initialized, waiting for entries...");
    } catch (e) {
      console.error("Failed to start log streaming:", e);
      addLogEntry(JSON.stringify({
        MESSAGE: `Failed to start log streaming: ${e}`,
        __REALTIME_TIMESTAMP: Date.now() * 1000
      }));
    }
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });
</script>

<div class="log-viewer-overlay" onclick={onClose}>
  <div class="log-viewer" onclick={(e) => e.stopPropagation()}>
    <!-- Header -->
    <div class="header">
      <div class="title">
        <span class="icon">📋</span>
        <span>KeyDeck Daemon Logs</span>
      </div>
      <button class="close-btn" onclick={onClose} title="Close">✕</button>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <div class="filters">
        <button
          class="filter-btn"
          class:active={filter === "all"}
          onclick={() => filter = "all"}
        >
          All
        </button>
        <button
          class="filter-btn"
          class:active={filter === "warnings"}
          onclick={() => filter = "warnings"}
        >
          Warnings
        </button>
        <button
          class="filter-btn"
          class:active={filter === "errors"}
          onclick={() => filter = "errors"}
        >
          Errors
        </button>
      </div>

      <input
        type="search"
        class="search-input"
        bind:value={searchQuery}
        placeholder="Search logs..."
      />

      <div class="actions">
        <button
          class="action-btn"
          class:active={isPaused}
          onclick={togglePause}
          title={isPaused ? "Resume" : "Pause"}
        >
          {isPaused ? "▶️" : "⏸️"}
        </button>
        <button class="action-btn" onclick={clearLogs} title="Clear">
          🗑️
        </button>
        <button class="action-btn" onclick={copyLogs} title="Copy all logs">
          📋
        </button>
      </div>
    </div>

    <!-- Log container -->
    <div
      class="log-container"
      bind:this={scrollContainer}
      onscroll={handleScroll}
    >
      {#if filteredLogs.length === 0}
        <div class="empty-state">
          {#if logs.length === 0}
            <p>No logs yet. Waiting for daemon output...</p>
          {:else}
            <p>No logs match your filters.</p>
          {/if}
        </div>
      {:else}
        {#each filteredLogs as log (log.id)}
          <div class="log-entry {log.level}">
            <span class="timestamp">{log.timestamp}</span>
            <span class="level-badge {log.level}">
              {#if log.level === "error"}
                ❌
              {:else if log.level === "warn"}
                ⚠️
              {:else}
                ℹ️
              {/if}
            </span>
            <span class="message">{log.message}</span>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Jump to bottom button -->
    {#if !isAtBottom && !isPaused}
      <button class="jump-to-bottom" onclick={scrollToBottom}>
        ↓ New messages
      </button>
    {/if}

    <!-- Status bar -->
    <div class="status-bar">
      <span class="status-item">
        Showing: {filteredLogs.length} / Total: {logs.length} entries
      </span>
      {#if isPaused}
        <span class="status-item paused">⏸️ Paused</span>
      {:else}
        <span class="status-item live">● Live</span>
      {/if}
    </div>
  </div>
</div>

<style>
  .log-viewer-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    backdrop-filter: blur(4px);
  }

  .log-viewer {
    background: #1e1e1e;
    border: 1px solid #3c3c3c;
    border-radius: 8px;
    width: 90%;
    max-width: 1200px;
    height: 80%;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
    font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid #3c3c3c;
    background: #252525;
    border-radius: 8px 8px 0 0;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 16px;
    font-weight: 600;
    color: #e0e0e0;
  }

  .icon {
    font-size: 20px;
  }

  .close-btn {
    background: none;
    border: none;
    color: #999;
    font-size: 20px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .close-btn:hover {
    background: #3c3c3c;
    color: #fff;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 20px;
    border-bottom: 1px solid #3c3c3c;
    background: #252525;
  }

  .filters {
    display: flex;
    gap: 4px;
  }

  .filter-btn {
    padding: 6px 12px;
    background: #2a2a2a;
    border: 1px solid #3c3c3c;
    color: #999;
    font-size: 12px;
    font-weight: 500;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .filter-btn:hover {
    background: #3c3c3c;
    color: #e0e0e0;
  }

  .filter-btn.active {
    background: #0e639c;
    border-color: #0e639c;
    color: #fff;
  }

  .search-input {
    flex: 1;
    padding: 6px 12px;
    background: #2a2a2a;
    border: 1px solid #3c3c3c;
    color: #e0e0e0;
    font-size: 13px;
    border-radius: 4px;
    outline: none;
    font-family: inherit;
  }

  .search-input:focus {
    border-color: #0e639c;
  }

  .actions {
    display: flex;
    gap: 4px;
  }

  .action-btn {
    padding: 6px 10px;
    background: #2a2a2a;
    border: 1px solid #3c3c3c;
    color: #999;
    font-size: 14px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn:hover {
    background: #3c3c3c;
  }

  .action-btn.active {
    background: #f44336;
    border-color: #f44336;
    color: #fff;
  }

  .log-container {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
    background: #1e1e1e;
  }

  .log-container::-webkit-scrollbar {
    width: 12px;
  }

  .log-container::-webkit-scrollbar-track {
    background: #1e1e1e;
  }

  .log-container::-webkit-scrollbar-thumb {
    background: #3c3c3c;
    border-radius: 6px;
  }

  .log-container::-webkit-scrollbar-thumb:hover {
    background: #4a4a4a;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
    font-size: 14px;
  }

  .log-entry {
    padding: 6px 20px;
    border-left: 3px solid transparent;
    transition: all 0.2s;
    display: flex;
    align-items: baseline;
    gap: 12px;
    animation: fade-in 0.3s ease;
  }

  @keyframes fade-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .log-entry:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .log-entry.error {
    border-left-color: #f44336;
    background: rgba(244, 67, 54, 0.05);
  }

  .log-entry.warn {
    border-left-color: #ff9800;
    background: rgba(255, 152, 0, 0.05);
  }

  .log-entry.info {
    border-left-color: #2196f3;
  }

  .timestamp {
    color: #666;
    font-size: 12px;
    min-width: 80px;
    flex-shrink: 0;
  }

  .level-badge {
    font-size: 14px;
    flex-shrink: 0;
  }

  .message {
    color: #d4d4d4;
    font-size: 13px;
    word-break: break-word;
  }

  .log-entry.error .message {
    color: #ff6b6b;
  }

  .log-entry.warn .message {
    color: #ffa726;
  }

  .jump-to-bottom {
    position: absolute;
    bottom: 60px;
    right: 30px;
    padding: 10px 16px;
    background: #0e639c;
    border: none;
    color: #fff;
    font-size: 13px;
    font-weight: 600;
    border-radius: 6px;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    transition: all 0.2s;
    animation: slide-up 0.3s ease;
  }

  @keyframes slide-up {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .jump-to-bottom:hover {
    background: #1976d2;
    transform: translateY(-2px);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.5);
  }

  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 20px;
    border-top: 1px solid #3c3c3c;
    background: #252525;
    font-size: 12px;
    color: #666;
    border-radius: 0 0 8px 8px;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-item.live {
    color: #4caf50;
    font-weight: 600;
  }

  .status-item.paused {
    color: #ff9800;
    font-weight: 600;
  }
</style>
