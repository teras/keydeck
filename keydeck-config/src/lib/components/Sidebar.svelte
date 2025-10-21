<script lang="ts">
  import PageList from "./PageList.svelte";
  import TemplateList from "./TemplateList.svelte";
  import ServiceList from "./ServiceList.svelte";
  import MacroList from "./MacroList.svelte";
  import ButtonDefList from "./ButtonDefList.svelte";
  import SystemConfig from "./SystemConfig.svelte";

  interface Props {
    config: any;
    selectedDevice: any;
    currentPage: string;
    currentTemplate: string | null;
    onPageSelected: (pageName: string) => void;
    onTemplateSelected: (templateName: string | null, keepButtonSelection?: boolean) => void;
  }

  let {
    config,
    selectedDevice,
    currentPage,
    currentTemplate,
    onPageSelected,
    onTemplateSelected
  }: Props = $props();

  type Tab = 'pages' | 'templates' | 'services' | 'macros' | 'buttons' | 'system' | null;
  let activeTab = $state<Tab>('pages');
  let isOpen = $state(true);

  function toggleTab(tab: Tab) {
    if (activeTab === tab && isOpen) {
      // Clicking active tab closes the sidebar
      isOpen = false;
    } else {
      // Open sidebar and switch to clicked tab
      activeTab = tab;
      isOpen = true;
    }
  }
</script>

<div class="sidebar-container">
  <!-- Tab Bar (always visible) -->
  <div class="tab-bar">
    <button
      class="tab-button"
      class:active={activeTab === 'pages' && isOpen}
      onclick={() => toggleTab('pages')}
      title="Pages"
      disabled={!selectedDevice || !config}
    >
      <span class="icon">📄</span>
      <span class="label">Pages</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'templates' && isOpen}
      onclick={() => toggleTab('templates')}
      title="Templates"
      disabled={!config}
    >
      <span class="icon">📋</span>
      <span class="label">Templates</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'services' && isOpen}
      onclick={() => toggleTab('services')}
      title="Services"
      disabled={!config}
    >
      <span class="icon">🔌</span>
      <span class="label">Services</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'macros' && isOpen}
      onclick={() => toggleTab('macros')}
      title="Macros"
      disabled={!config}
    >
      <span class="icon">🔧</span>
      <span class="label">Macros</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'buttons' && isOpen}
      onclick={() => toggleTab('buttons')}
      title="Button Definitions"
      disabled={!config}
    >
      <span class="icon">🔘</span>
      <span class="label">Buttons</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'system' && isOpen}
      onclick={() => toggleTab('system')}
      title="System Configuration"
      disabled={!config}
    >
      <span class="icon">⚙️</span>
      <span class="label">System</span>
    </button>
  </div>

  <!-- Content Panel (slides in/out) -->
  <div class="content-panel" class:closed={!isOpen}>
    {#if activeTab === 'pages' && selectedDevice && config}
      <PageList
        config={config}
        deviceSerial={selectedDevice.serial}
        currentPage={currentPage}
        onPageSelected={onPageSelected}
      />
    {:else if activeTab === 'templates' && config}
      <TemplateList
        config={config}
        currentTemplate={currentTemplate}
        onTemplateSelected={onTemplateSelected}
      />
    {:else if activeTab === 'services' && config}
      <ServiceList config={config} />
    {:else if activeTab === 'macros' && config}
      <MacroList config={config} />
    {:else if activeTab === 'buttons' && config}
      <ButtonDefList config={config} />
    {:else if activeTab === 'system' && config}
      <SystemConfig config={config} selectedDevice={selectedDevice} />
    {/if}
  </div>
</div>

<style>
  .sidebar-container {
    display: flex;
    flex-direction: row;
    height: 100%;
  }

  .tab-bar {
    width: 50px;
    flex-shrink: 0;
    background-color: #2d2d30;
    border-right: 1px solid #3e3e42;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 8px 0;
    gap: 4px;
  }

  .tab-button {
    width: 48px;
    height: 48px;
    background: none;
    border: none;
    color: #858585;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 4px;
    border-radius: 4px;
    transition: background-color 0.2s, color 0.2s;
    position: relative;
  }

  .tab-button:hover:not(:disabled) {
    background-color: #3e3e42;
    color: #cccccc;
  }

  .tab-button.active {
    background-color: #37373d;
    color: #ffffff;
  }

  .tab-button.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 2px;
    background-color: #0e639c;
  }

  .tab-button:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .icon {
    font-size: 20px;
    line-height: 1;
  }

  .label {
    font-size: 9px;
    line-height: 1;
    writing-mode: horizontal-tb;
    text-align: center;
    font-weight: 500;
    letter-spacing: 0.3px;
  }

  .content-panel {
    width: 250px;
    flex-shrink: 0;
    background-color: #252526;
    border-right: 1px solid #3e3e42;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 16px;
    transition: width 0.2s ease-out, padding 0.2s ease-out, opacity 0.2s ease-out;
  }

  .content-panel.closed {
    width: 0;
    padding-left: 0;
    padding-right: 0;
    opacity: 0;
    pointer-events: none;
  }
</style>
