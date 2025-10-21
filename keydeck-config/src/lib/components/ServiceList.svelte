<script lang="ts">
  interface Props {
    config: any;
    onServiceSelected?: (serviceName: string | null) => void;
  }

  let { config, onServiceSelected }: Props = $props();

  let services = $derived(Object.keys(config.services || {}));
  let showAddService = $state(false);
  let newServiceName = $state("");
  let showServiceMenu = $state<string | null>(null);

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

    config.services[newServiceName] = "echo 'command here'";
    newServiceName = "";
    showAddService = false;
    if (onServiceSelected) {
      onServiceSelected(newServiceName);
    }
  }

  function deleteService(serviceName: string) {
    if (confirm(`Delete service "${serviceName}"?`)) {
      delete config.services[serviceName];
      showServiceMenu = null;
    }
  }
</script>

<div class="service-list">
  <div class="header">
    <h3>Services</h3>
    <button class="add-btn" onclick={() => showAddService = !showAddService}>+</button>
  </div>

  {#if showAddService}
    <div class="add-service">
      <input
        type="text"
        bind:value={newServiceName}
        placeholder="Service name"
        onkeydown={(e) => e.key === 'Enter' && addService()}
      />
      <button onclick={addService}>Add</button>
      <button onclick={() => showAddService = false}>Cancel</button>
    </div>
  {/if}

  <div class="services">
    {#each services as service}
      <div class="service-row">
        <button class="service-item">
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

        {#if showServiceMenu === service}
          <div class="service-menu">
            <button onclick={() => deleteService(service)}>Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .service-list {
    margin-top: 20px;
    padding-top: 20px;
    border-top: 1px solid #3e3e42;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
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
    margin-bottom: 8px;
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
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .add-service button:hover {
    background-color: #1177bb;
  }

  .add-service button:last-child {
    background-color: #555;
  }

  .add-service button:last-child:hover {
    background-color: #666;
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
