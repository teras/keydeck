<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { fade } from 'svelte/transition';
  import PageList from "./PageList.svelte";
  import TemplateList from "./TemplateList.svelte";
  import ServiceList from "./ServiceList.svelte";
  import MacroList from "./MacroList.svelte";
  import ButtonDefList from "./ButtonDefList.svelte";
  import DeviceSettings from "./DeviceSettings.svelte";
  import GlobalSettings from "./GlobalSettings.svelte";

  interface Props {
    config: any;
    selectedDevice: any;
    currentPage: string;
    currentTemplate: string | null;
    currentService: string | null;
    currentMacro: string | null;
    currentButtonDef: string | null;
    selectedButton: number | null;
    onPageSelected: (pageName: string) => void;
    onTemplateSelected: (templateName: string | null, keepButtonSelection?: boolean) => void;
    onServiceSelected: (serviceName: string | null) => void;
    onMacroSelected: (macroName: string | null) => void;
    onButtonDefSelected: (buttonName: string | null) => void;
    openTab?: (tab: Tab) => void;
  }

  export type SidebarTab = 'pages' | 'templates' | 'services' | 'macros' | 'buttons' | 'device' | 'global' | null;

  let {
    config,
    selectedDevice,
    currentPage,
    currentTemplate,
    currentService,
    currentMacro,
    currentButtonDef,
    selectedButton,
    onPageSelected,
    onTemplateSelected,
    onServiceSelected,
    onMacroSelected,
    onButtonDefSelected,
    openTab
  }: Props = $props();

  type Tab = 'pages' | 'templates' | 'services' | 'macros' | 'buttons' | 'device' | 'global' | null;
  let activeTab = $state<Tab>('pages');
  let isOpen = $state(false);
  let contentPanel: HTMLDivElement | undefined;

  function toggleTab(tab: Tab) {
    if (activeTab === tab && isOpen) {
      // Clicking active tab closes the sidebar
      isOpen = false;
    } else {
      // Open sidebar and switch to clicked tab
      activeTab = tab;
      isOpen = true;

      // Smooth scroll to top when switching tabs
      if (contentPanel) {
        contentPanel.scrollTo({ top: 0, behavior: 'smooth' });
      }
    }
  }

  function openTabOnly(tab: Tab) {
    // Always open the tab without toggling
    activeTab = tab;
    isOpen = true;

    // Smooth scroll to top when switching tabs
    if (contentPanel) {
      contentPanel.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }

  // Expose openTabOnly to parent via openTab callback
  if (openTab) {
    openTab(openTabOnly as any);
  }
</script>

<div class="sidebar-container">
  <!-- Tab Bar (always visible) -->
  <div class="tab-bar">
    <button
      class="tab-button"
      class:active={activeTab === 'device' && isOpen}
      onclick={() => toggleTab('device')}
      title="Device Settings"
      disabled={!selectedDevice || !config}
    >
      <span class="icon">📟</span>
      <span class="label">Device</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'pages' && isOpen}
      onclick={() => toggleTab('pages')}
      title="Pages"
      disabled={!selectedDevice || !config}
    >
      <span class="icon">🗂️</span>
      <span class="label">Pages</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'templates' && isOpen}
      onclick={() => toggleTab('templates')}
      title="Templates"
      disabled={!config}
    >
      <span class="icon">🏗️</span>
      <span class="label">Templates</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'services' && isOpen}
      onclick={() => toggleTab('services')}
      title="Services"
      disabled={!config}
    >
      <span class="icon">📡</span>
      <span class="label">Services</span>
    </button>

    <button
      class="tab-button"
      class:active={activeTab === 'macros' && isOpen}
      onclick={() => toggleTab('macros')}
      title="Macros"
      disabled={!config}
    >
      <span class="icon">🤖</span>
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
      class:active={activeTab === 'global' && isOpen}
      onclick={() => toggleTab('global')}
      title="Global Settings"
      disabled={!config}
    >
      <span class="icon">🌍</span>
      <span class="label">Global</span>
    </button>
  </div>

  <!-- Content Panel (slides in/out) -->
  <div class="content-panel" class:closed={!isOpen} bind:this={contentPanel}>
    {#key activeTab}
      <div class="tab-content" in:fade={{ duration: 200 }}>
        {#if activeTab === 'pages' && selectedDevice && config}
          <PageList
            config={config}
            deviceSerial={selectedDevice.serial}
            currentPage={currentPage}
            selectedButton={selectedButton}
            onPageSelected={onPageSelected}
          />
        {:else if activeTab === 'templates' && config}
          <TemplateList
            config={config}
            currentTemplate={currentTemplate}
            selectedButton={selectedButton}
            onTemplateSelected={onTemplateSelected}
          />
        {:else if activeTab === 'services' && config}
          <ServiceList
            config={config}
            currentService={currentService}
            onServiceSelected={onServiceSelected}
          />
        {:else if activeTab === 'macros' && config}
          <MacroList
            config={config}
            currentMacro={currentMacro}
            onMacroSelected={onMacroSelected}
          />
        {:else if activeTab === 'buttons' && config}
          <ButtonDefList
            config={config}
            currentButtonDef={currentButtonDef}
            onButtonDefSelected={onButtonDefSelected}
          />
        {:else if activeTab === 'device' && config && selectedDevice}
          <DeviceSettings config={config} selectedDevice={selectedDevice} />
        {:else if activeTab === 'global' && config}
          <GlobalSettings config={config} />
        {/if}
      </div>
    {/key}
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
    right: 0;
    bottom: 0;
    height: 2px;
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
    scroll-behavior: smooth;
  }

  .content-panel.closed {
    width: 0;
    padding-left: 0;
    padding-right: 0;
    opacity: 0;
    pointer-events: none;
  }
</style>
