<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface DeviceListItem {
    device_id: string;
    serial: string;
    model: string;
  }

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
    onDeviceSelected: (device: DeviceInfo) => void;
    onRefresh?: () => void;
  }

  let { onDeviceSelected, onRefresh }: Props = $props();

  let devices = $state<DeviceListItem[]>([]);
  let selectedDeviceId = $state<string | null>(null);
  let loading = $state(false);
  let error = $state<string>("");

  onMount(async () => {
    await refreshDevices();
  });

  async function refreshDevices() {
    error = "";
    try {
      // Call the parent's refresh callback to reload config
      // This might show a confirm dialog, so don't set loading=true yet
      if (onRefresh) {
        await onRefresh();
      }

      // Only set loading after dialog is dismissed and reload completes
      loading = true;
      devices = await invoke("list_devices");
      // Auto-select first device if available (or re-select if already selected)
      if (devices.length > 0) {
        const deviceToSelect = selectedDeviceId || devices[0].device_id;
        await selectDevice(deviceToSelect);
      }
    } catch (e) {
      console.error("Failed to list devices:", e);
      error = `Failed to list devices: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function selectDevice(deviceId: string) {
    selectedDeviceId = deviceId;
    try {
      const deviceInfo: DeviceInfo = await invoke("get_device_info", { deviceId });
      onDeviceSelected(deviceInfo);
    } catch (e) {
      console.error("Failed to get device info:", e);
      error = `Failed to get device info: ${e}`;
    }
  }
</script>

<div class="device-selector">
  {#if loading}
    <span class="status">Loading devices...</span>
  {:else if error}
    <span class="error">{error}</span>
  {:else if devices.length === 0}
    <span class="status">No devices found</span>
  {:else}
    <select
      bind:value={selectedDeviceId}
      onchange={() => {
        console.log("Dropdown changed to:", selectedDeviceId);
        if (selectedDeviceId) {
          selectDevice(selectedDeviceId);
        }
      }}
    >
      {#each devices as device}
        <option value={device.device_id}>
          {device.model} ({device.serial.slice(-6)})
        </option>
      {/each}
    </select>
  {/if}

  <button onclick={refreshDevices} disabled={loading} title="Refresh devices and reload configuration">
    ↻
  </button>
</div>

<style>
  .device-selector {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  select {
    min-width: 250px;
    padding: 8px 12px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 16px;
  }

  button {
    padding: 8px 12px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 24px;
    line-height: 1;
    min-width: 42px;
    height: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  button:hover {
    background-color: #1177bb;
  }

  button:disabled {
    background-color: #555;
    cursor: not-allowed;
  }

  .status {
    color: #888;
    font-size: 13px;
    font-style: italic;
    white-space: nowrap;
  }

  .error {
    color: #f48771;
    font-size: 13px;
    white-space: nowrap;
  }
</style>
