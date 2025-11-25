<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog';

  interface Props {
    config: any;
    currentService: string | null;
    onServiceSelected: (serviceName: string | null) => void;
  }

  let { config, currentService, onServiceSelected }: Props = $props();

  let services = $derived(Object.keys(config.services || {}));
  let showAddService = $state(false);
  let newServiceName = $state("");
  let showServiceMenu = $state<string | null>(null);
  let serviceNameInput = $state<HTMLInputElement | undefined>();
  let renameServiceName = $state("");
  let renamingService = $state<string | null>(null);

  function toggleAddService() {
    showAddService = !showAddService;
    if (showAddService) {
      setTimeout(() => serviceNameInput?.focus(), 0);
    }
  }

  // Click-outside handler for menu
  $effect(() => {
    if (showServiceMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.service-menu') && !target.closest('.service-menu-btn')) {
          showServiceMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  function addService() {
    if (!newServiceName.trim()) return;

    if (!config.services) {
      config.services = {};
    }

    const serviceName = newServiceName.trim();
    if (config.services[serviceName]) {
      alert(`Service "${serviceName}" already exists!`);
      return;
    }

    config.services[serviceName] = {
      exec: ""
    };
    newServiceName = "";
    showAddService = false;
    // Select the newly added service
    if (onServiceSelected) {
      onServiceSelected(serviceName);
    }
  }

  async function deleteService(serviceName: string) {
    showServiceMenu = null;

    const confirmed = await ask(`Delete service "${serviceName}"?`, {
      title: 'Confirm Delete',
      kind: 'warning'
    });

    if (confirmed) {
      delete config.services[serviceName];
      if (currentService === serviceName) {
        onServiceSelected(null);
      }
    }
  }

  function duplicateService(serviceName: string) {
    const original = config.services[serviceName];
    let newName = `${serviceName}_copy`;
    let counter = 1;
    while (config.services[newName]) {
      newName = `${serviceName}_copy${counter}`;
      counter++;
    }

    // Deep clone the service
    config.services[newName] = JSON.parse(JSON.stringify(original));
    onServiceSelected(newName);
    showServiceMenu = null;
  }

  function startRename(serviceName: string) {
    renamingService = serviceName;
    renameServiceName = serviceName;
    showServiceMenu = null;
  }

  function renameService(oldName: string) {
    if (!renameServiceName.trim() || renameServiceName === oldName) {
      renameServiceName = "";
      renamingService = null;
      return;
    }

    if (config.services[renameServiceName]) {
      alert(`Service "${renameServiceName}" already exists!`);
      renameServiceName = "";
      renamingService = null;
      return;
    }

    // Rebuild object preserving order
    const newServices: any = {};
    for (const key of Object.keys(config.services)) {
      if (key === oldName) {
        newServices[renameServiceName] = config.services[oldName];
      } else {
        newServices[key] = config.services[key];
      }
    }
    config.services = newServices;

    if (currentService === oldName) {
      onServiceSelected(renameServiceName);
    }

    renameServiceName = "";
    renamingService = null;
  }
</script>

<div class="service-list">
  <div class="header">
    <h3>Services</h3>
    <button class="add-btn" onclick={toggleAddService}>+</button>
  </div>

  {#if showAddService}
    <div class="add-service">
      <input
        type="text"
        bind:this={serviceNameInput}
        bind:value={newServiceName}
        placeholder="Service name"
        onkeydown={(e) => e.key === 'Enter' && addService()}
      />
      <button onclick={addService} title="Add">✓</button>
      <button onclick={() => showAddService = false} title="Cancel">✕</button>
    </div>
  {/if}

  <div class="separator"></div>

  <div class="services">
    {#each services as service}
      <div class="service-row">
        {#if renamingService === service}
          <input
            type="text"
            bind:value={renameServiceName}
            class="rename-input"
            onkeydown={(e) => {
              if (e.key === 'Enter') renameService(service);
              if (e.key === 'Escape') { renameServiceName = ""; renamingService = null; }
            }}
            onblur={() => renameService(service)}
            onmousedown={(e) => e.stopPropagation()}
            autofocus
          />
        {:else}
          <button
            class="service-item"
            class:active={service === currentService}
            onclick={() => onServiceSelected(service)}
          >
            {service}
          </button>
          <button
            class="service-menu-btn"
            onclick={(e) => {
              e.stopPropagation();
              showServiceMenu = showServiceMenu === service ? null : service;
            }}
          >
            ⋮
          </button>
        {/if}

        {#if showServiceMenu === service}
          <div class="service-menu">
            <button onclick={(e) => { e.stopPropagation(); startRename(service); }}>✏️ Rename</button>
            <button onclick={(e) => { e.stopPropagation(); duplicateService(service); }}>📋 Duplicate</button>
            <button class="danger" onclick={(e) => { e.stopPropagation(); deleteService(service); }}>🗑️ Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .service-list {
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

  .add-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .add-btn:hover {
    background-color: #1177bb;
  }

  .add-service {
    display: flex;
    gap: 4px;
    margin-top: 12px;
    margin-bottom: 12px;
  }

  .separator {
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 16px;
  }

  .add-service input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .add-service button {
    padding: 6px 12px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-service button:first-of-type {
    background-color: #2d7d46;
  }

  .add-service button:first-of-type:hover {
    background-color: #3a9d5a;
  }

  .add-service button:last-child {
    background-color: #7a2d2d;
  }

  .add-service button:last-child:hover {
    background-color: #9a3d3d;
  }

  .services {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .service-row {
    position: relative;
    display: flex;
    gap: 4px;
  }

  .rename-input {
    flex: 1;
    padding: 8px 12px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #0e639c;
    border-radius: 4px;
    font-size: 13px;
  }

  .service-item {
    flex: 1;
    padding: 8px 12px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
  }

  .service-item:hover {
    background-color: #4a4a4a;
  }

  .service-item.active {
    background-color: #354a5f;
    border-color: #5b9bd5;
  }

  .service-menu-btn {
    width: 28px;
    padding: 4px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
  }

  .service-menu-btn:hover {
    background-color: #4a4a4a;
    color: #cccccc;
  }

  .service-menu {
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

  .service-menu button {
    padding: 8px 12px;
    background: none;
    color: #cccccc;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }

  .service-menu button:hover {
    background-color: #3c3c3c;
  }
</style>
