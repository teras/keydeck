<script lang="ts">
  import ActionEditor from './ActionEditor.svelte';
  import TemplateSelector from './TemplateSelector.svelte';

  interface Props {
    config: any;
    pageName: string;
    deviceSerial: string;
  }

  let { config, pageName, deviceSerial }: Props = $props();

  let pageGroup = $derived(config.page_groups?.[deviceSerial] || config.page_groups?.default);
  let page = $derived(pageGroup?.[pageName]);
  let groupKey = $derived(config.page_groups?.[deviceSerial] ? deviceSerial : 'default');

  function updateWindowClass(value: string) {
    if (!page) return;
    if (value.trim()) {
      page.window_class = value;
      config[groupKey][pageName].window_class = value;
    } else {
      delete page.window_class;
      delete config[groupKey][pageName].window_class;
    }
  }

  function updateInherits(templates: string[]) {
    if (!page) return;
    if (templates.length > 0) {
      page.inherits = templates;
      config[groupKey][pageName].inherits = templates;
    } else {
      delete page.inherits;
      delete config[groupKey][pageName].inherits;
    }
  }

  function getSelectedTemplates(): string[] {
    if (!page?.inherits) return [];
    if (Array.isArray(page.inherits)) return page.inherits;
    return [page.inherits];
  }

  function addOnTickAction() {
    if (!page) return;
    if (!page.on_tick) {
      page.on_tick = [];
    }
    page.on_tick = [...page.on_tick, { refresh: 'dynamic' }];
    config[groupKey][pageName].on_tick = page.on_tick;
  }

  function updateOnTickAction(index: number, newAction: any) {
    if (!page?.on_tick) return;
    page.on_tick[index] = newAction;
    config[groupKey][pageName].on_tick = page.on_tick;
  }

  function removeOnTickAction(index: number) {
    if (!page?.on_tick) return;
    page.on_tick.splice(index, 1);
    config[groupKey][pageName].on_tick = page.on_tick;
  }
</script>

<div class="page-editor">
  <h3>Page: {pageName}</h3>

  <div class="section">
    <h4>Page Settings</h4>

    <div class="form-group">
      <label>Window Class / Title</label>
      <input
        type="text"
        value={page?.window_class || ""}
        oninput={(e) => updateWindowClass(e.currentTarget.value)}
        placeholder="window class or title pattern"
      />
      <p class="help">Page will be activated when a window matching this class or title is focused</p>
    </div>

    <div class="form-group">
      <label>Inherits Templates</label>
      <TemplateSelector
        {config}
        selectedTemplates={getSelectedTemplates()}
        onUpdate={updateInherits}
      />
      <p class="help">Select one or more templates to inherit</p>
    </div>
  </div>

  <div class="section">
    <h4>On Tick Actions</h4>
    <p class="help">Actions executed periodically based on global tick_time</p>
    <div class="actions-list">
      {#if page?.on_tick && page.on_tick.length > 0}
        {#each page.on_tick as action, i}
          <ActionEditor
            {action}
            index={i}
            {config}
            deviceSerial={deviceSerial}
            onUpdate={(newAction) => updateOnTickAction(i, newAction)}
            onDelete={() => removeOnTickAction(i)}
          />
        {/each}
      {:else}
        <p class="empty">No on_tick actions configured</p>
      {/if}
    </div>
    <button onclick={addOnTickAction}>+ Add On Tick Action</button>
  </div>
</div>

<style>
  .page-editor {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  h3 {
    margin: 0;
    font-size: 16px;
    color: #cccccc;
    padding-bottom: 12px;
    border-bottom: 1px solid #3e3e42;
  }

  h4 {
    margin: 0 0 12px 0;
    font-size: 13px;
    color: #aaa;
  }

  .section {
    padding-top: 12px;
    border-top: 1px solid #3e3e42;
  }

  .section:first-of-type {
    padding-top: 0;
    border-top: none;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 16px;
  }

  label {
    font-size: 12px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
  }

  input {
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
  }

  input:focus {
    outline: none;
    border-color: #0e639c;
  }

  .help {
    margin: 0;
    font-size: 11px;
    color: #666;
    font-style: italic;
  }

  .actions-list {
    display: flex;
    flex-direction: column;
    margin-bottom: 12px;
  }

  .empty {
    color: #666;
    font-size: 12px;
    font-style: italic;
    margin: 8px 0;
  }

  button {
    padding: 8px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  button:hover {
    background-color: #1177bb;
  }
</style>
