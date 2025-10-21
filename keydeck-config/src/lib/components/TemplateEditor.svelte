<script lang="ts">
  import ActionEditor from './ActionEditor.svelte';

  interface Props {
    config: any;
    templateName: string;
  }

  let { config, templateName }: Props = $props();

  let template = $derived(config.templates?.[templateName] || {});

  function updateInherits(value: string) {
    if (!config.templates) config.templates = {};
    if (!config.templates[templateName]) config.templates[templateName] = {};

    const inheritsArray = value.split(',').map(s => s.trim()).filter(s => s.length > 0);
    if (inheritsArray.length > 0) {
      config.templates[templateName].inherits = inheritsArray;
    } else {
      delete config.templates[templateName].inherits;
    }
  }

  function getInheritsValue(): string {
    return template.inherits ? template.inherits.join(', ') : '';
  }

  function addOnTickAction() {
    if (!config.templates[templateName]) {
      config.templates[templateName] = {};
    }
    if (!config.templates[templateName].on_tick) {
      config.templates[templateName].on_tick = [];
    }
    config.templates[templateName].on_tick = [
      ...config.templates[templateName].on_tick,
      { refresh: 'dynamic' }
    ];
  }

  function updateOnTickAction(index: number, newAction: any) {
    if (config.templates[templateName]?.on_tick) {
      config.templates[templateName].on_tick[index] = newAction;
    }
  }

  function removeOnTickAction(index: number) {
    if (config.templates[templateName]?.on_tick) {
      config.templates[templateName].on_tick.splice(index, 1);
    }
  }
</script>

<div class="template-editor">
  <h3>Template: {templateName}</h3>

  <div class="section">
    <h4>Template Settings</h4>

    <div class="form-group">
      <label>Inherits From</label>
      <input
        type="text"
        value={getInheritsValue()}
        oninput={(e) => updateInherits(e.currentTarget.value)}
        placeholder="template1, template2"
      />
      <p class="help">Comma-separated list of templates to inherit from</p>
    </div>
  </div>

  <div class="section">
    <h4>On Tick Actions</h4>
    <p class="help">Actions executed periodically based on global tick_time</p>
    <div class="actions-list">
      {#if template?.on_tick && template.on_tick.length > 0}
        {#each template.on_tick as action, i}
          <ActionEditor
            {action}
            index={i}
            {config}
            deviceSerial=""
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

  <div class="info-box">
    <p><strong>Note:</strong> Templates are page layouts that can be reused.</p>
    <p>Configure buttons in the grid view, and reference this template in pages using the "Inherits From" field.</p>
  </div>
</div>

<style>
  .template-editor {
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

  .info-box {
    padding: 12px;
    background-color: #2d2d30;
    border-left: 3px solid #0e639c;
    border-radius: 4px;
  }

  .info-box p {
    margin: 0 0 8px 0;
    font-size: 12px;
    color: #aaa;
    line-height: 1.4;
  }

  .info-box p:last-child {
    margin-bottom: 0;
  }

  .info-box strong {
    color: #cccccc;
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
