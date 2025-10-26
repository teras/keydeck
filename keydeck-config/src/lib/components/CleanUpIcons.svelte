<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { ask } from '@tauri-apps/plugin-dialog';

  interface Props {
    config: any;
    onCleanupComplete?: () => void;
  }

  let { config, onCleanupComplete }: Props = $props();

  interface IconCleanupPreview {
    in_use: string[];
    protected: string[];
    unused: string[];
  }

  let showDialog = $state(false);
  let preview = $state<IconCleanupPreview | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let activeTab = $state<'unused' | 'in_use' | 'protected'>('unused');

  async function openPreview() {
    showDialog = true;
    loading = true;
    error = null;

    try {
      // Send only the protected patterns, backend will read config from disk
      const protectedPatterns = config.protected_icons || [];
      preview = await invoke<IconCleanupPreview>('preview_icon_cleanup', {
        protectedPatterns
      });
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  function closeDialog() {
    showDialog = false;
    preview = null;
    error = null;
    activeTab = 'unused';
  }

  async function executeCleanup() {
    if (!preview || preview.unused.length === 0) {
      return;
    }

    const confirmed = await ask(
      `This will permanently delete ${preview.unused.length} unused icon file${preview.unused.length === 1 ? '' : 's'}. This action cannot be undone.`,
      { title: 'Confirm Icon Cleanup', kind: 'warning' }
    );

    if (!confirmed) {
      return;
    }

    loading = true;
    error = null;

    try {
      // Send only the protected patterns, backend will read config from disk
      const protectedPatterns = config.protected_icons || [];
      await invoke<number>('execute_icon_cleanup', {
        protectedPatterns
      });

      // Close dialog and notify parent
      closeDialog();

      if (onCleanupComplete) {
        onCleanupComplete();
      }
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  function getIconCount(category: 'unused' | 'in_use' | 'protected'): number {
    if (!preview) return 0;
    return preview[category].length;
  }

  function getActiveList(): string[] {
    if (!preview) return [];
    return preview[activeTab];
  }
</script>

<button class="cleanup-button" onclick={openPreview} disabled={loading}>
  Clean Up Icons
</button>

{#if showDialog}
  <div class="dialog-overlay" onclick={closeDialog}>
    <div class="dialog" onclick={(e) => e.stopPropagation()}>
      <div class="dialog-header">
        <h3>Icon Cleanup Preview</h3>
        <button class="close-btn" onclick={closeDialog}>✕</button>
      </div>

      <div class="dialog-content">
        {#if loading}
          <div class="loading">Loading icon analysis...</div>
        {:else if error}
          <div class="error">
            <p>Error: {error}</p>
          </div>
        {:else if preview}
          <div class="tabs">
            <button
              class="tab"
              class:active={activeTab === 'unused'}
              onclick={() => activeTab = 'unused'}
            >
              Unused ({getIconCount('unused')})
            </button>
            <button
              class="tab"
              class:active={activeTab === 'in_use'}
              onclick={() => activeTab = 'in_use'}
            >
              In Use ({getIconCount('in_use')})
            </button>
            <button
              class="tab"
              class:active={activeTab === 'protected'}
              onclick={() => activeTab = 'protected'}
            >
              Protected ({getIconCount('protected')})
            </button>
          </div>

          <div class="icon-list">
            {#if getActiveList().length === 0}
              <p class="empty">
                {#if activeTab === 'unused'}
                  No unused icons found.
                {:else if activeTab === 'in_use'}
                  No icons currently in use.
                {:else}
                  No protected icons.
                {/if}
              </p>
            {:else}
              <div class="icon-grid">
                {#each getActiveList() as icon}
                  <div class="icon-item" title={icon}>
                    <div class="icon-filename">{icon}</div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <div class="dialog-footer">
            {#if activeTab === 'unused' && preview.unused.length > 0}
              <p class="warning">
                {preview.unused.length} icon{preview.unused.length === 1 ? '' : 's'} will be permanently deleted.
              </p>
              <div class="footer-buttons">
                <button class="cancel-btn" onclick={closeDialog} disabled={loading}>
                  Cancel
                </button>
                <button class="delete-btn" onclick={executeCleanup} disabled={loading}>
                  {loading ? 'Deleting...' : 'Delete Unused Icons'}
                </button>
              </div>
            {:else}
              <div class="footer-buttons">
                <button class="cancel-btn" onclick={closeDialog}>
                  Close
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .cleanup-button {
    padding: 8px 16px;
    background: #dc3545;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: background 0.2s;
  }

  .cleanup-button:hover:not(:disabled) {
    background: #c82333;
  }

  .cleanup-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: #252526;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.5);
    width: 90%;
    max-width: 600px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }

  .dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid #3e3e42;
  }

  .dialog-header h3 {
    margin: 0;
    font-size: 16px;
    color: #cccccc;
  }

  .close-btn {
    background: none;
    border: none;
    color: #aaa;
    font-size: 20px;
    cursor: pointer;
    padding: 4px 8px;
    line-height: 1;
    transition: color 0.2s;
  }

  .close-btn:hover {
    color: #fff;
  }

  .dialog-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    padding: 16px 20px;
  }

  .loading, .error {
    padding: 20px;
    text-align: center;
    color: #aaa;
  }

  .error {
    color: #dc3545;
  }

  .tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 16px;
    border-bottom: 1px solid #3e3e42;
  }

  .tab {
    padding: 8px 16px;
    background: none;
    border: none;
    color: #aaa;
    cursor: pointer;
    font-size: 13px;
    border-bottom: 2px solid transparent;
    transition: all 0.2s;
  }

  .tab:hover {
    color: #fff;
  }

  .tab.active {
    color: #fff;
    border-bottom-color: #007acc;
  }

  .icon-list {
    flex: 1;
    overflow-y: auto;
    min-height: 200px;
  }

  .icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 8px;
  }

  .icon-item {
    background: #1e1e1e;
    border: 1px solid #3e3e42;
    border-radius: 4px;
    padding: 8px;
    font-size: 12px;
    color: #aaa;
    word-break: break-all;
    transition: all 0.2s;
  }

  .icon-item:hover {
    background: #2a2a2a;
    border-color: #555;
  }

  .icon-filename {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    text-align: center;
    color: #666;
    padding: 40px 20px;
    font-size: 14px;
  }

  .dialog-footer {
    border-top: 1px solid #3e3e42;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .warning {
    margin: 0;
    color: #ffc107;
    font-size: 13px;
    text-align: center;
  }

  .footer-buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .cancel-btn, .delete-btn {
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .cancel-btn {
    background: #3e3e42;
    color: #ccc;
  }

  .cancel-btn:hover:not(:disabled) {
    background: #4e4e52;
  }

  .delete-btn {
    background: #dc3545;
    color: white;
  }

  .delete-btn:hover:not(:disabled) {
    background: #c82333;
  }

  .cancel-btn:disabled, .delete-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
