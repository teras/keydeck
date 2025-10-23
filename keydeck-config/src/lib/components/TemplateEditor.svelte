<script lang="ts">
  import ActionEditor from './ActionEditor.svelte';
  import TemplateSelector from './TemplateSelector.svelte';

  interface Props {
    config: any;
    templateName: string;
  }

  let { config, templateName }: Props = $props();

  let template = $derived(config.templates?.[templateName] || {});

  function updateInherits(templates: string[]) {
    if (!config.templates) config.templates = {};
    if (!config.templates[templateName]) config.templates[templateName] = {};

    if (templates.length > 0) {
      config.templates[templateName].inherits = templates;
    } else {
      delete config.templates[templateName].inherits;
    }
  }

  function getSelectedTemplates(): string[] {
    if (!template.inherits) return [];
    if (Array.isArray(template.inherits)) return template.inherits;
    return [template.inherits];
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
      <TemplateSelector
        {config}
        selectedTemplates={getSelectedTemplates()}
        currentTemplateName={templateName}
        onUpdate={updateInherits}
      />
      <p class="help">Select one or more templates to inherit from</p>
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
