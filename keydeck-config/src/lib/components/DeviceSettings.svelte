<script lang="ts">
  interface Props {
    config: any;
    selectedDevice: any;
  }

  let { config, selectedDevice }: Props = $props();

  function getDevicePageGroup() {
    if (!selectedDevice || !config.page_groups) return null;
    return config.page_groups[selectedDevice.serial] || config.page_groups.default;
  }

  function updateMainPage(value: string) {
    const pageGroup = getDevicePageGroup();
    if (!pageGroup) return;

    if (value.trim()) {
      pageGroup.main_page = value.trim();
    } else {
      delete pageGroup.main_page;
    }
  }

  function updateRestoreMode(value: string) {
    const pageGroup = getDevicePageGroup();
    if (!pageGroup) return;

    pageGroup.restore_mode = value;
  }

  function getAvailablePages(): string[] {
    const pageGroup = getDevicePageGroup();
    if (!pageGroup) return [];

    const knownFields = ['main_page', 'restore_mode', 'on_tick'];
    return Object.keys(pageGroup).filter(key => !knownFields.includes(key));
  }

  let pageGroup = $derived(getDevicePageGroup());
  let availablePages = $derived(getAvailablePages());
</script>

<div class="device-settings">
  <div class="header">
    <h3>{selectedDevice.model}</h3>
  </div>

  <div class="separator"></div>

  <div class="settings-content">
    <div class="form-group">
      <label>Main Page</label>
      <select
        value={pageGroup?.main_page || ""}
        onchange={(e) => updateMainPage(e.currentTarget.value)}
      >
        <option value="">Auto (first page)</option>
        {#each availablePages as pageName}
          <option value={pageName}>{pageName}</option>
        {/each}
      </select>
      <p class="help">Default page to show when device starts</p>
    </div>

    <div class="form-group">
      <label>Restore Mode</label>
      <select
        value={pageGroup?.restore_mode || "main"}
        onchange={(e) => updateRestoreMode(e.currentTarget.value)}
      >
        <option value="keep">Keep - Stay on current page</option>
        <option value="last">Last - Return to last page</option>
        <option value="main">Main - Return to main page</option>
      </select>
      <p class="help">Page behavior on window focus change</p>
    </div>
  </div>
</div>

<style>
  .device-settings {
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 12px;
  }

  h3 {
    margin: 0;
    font-size: 16px;
    color: #cccccc;
  }

  .separator {
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 16px;
  }

  .settings-content {
    display: flex;
    flex-direction: column;
    gap: 16px;
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

  select {
    width: 100%;
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
  }

  select:focus {
    outline: none;
    border-color: #0e639c;
  }

  .help {
    margin: 0;
    font-size: 11px;
    color: #666;
    font-style: italic;
  }
</style>
