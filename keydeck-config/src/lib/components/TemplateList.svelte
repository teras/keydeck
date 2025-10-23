<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog';

  interface Props {
    config: any;
    currentTemplate: string | null;
    selectedButton: number | null;
    onTemplateSelected: (templateName: string | null) => void;
  }

  let { config, currentTemplate, selectedButton, onTemplateSelected }: Props = $props();

  let templates = $derived(Object.keys(config.templates || {}));
  let showTemplateMenu = $state<string | null>(null);
  let showAddTemplate = $state(false);
  let newTemplateName = $state("");
  let renameTemplateName = $state("");
  let renamingTemplate = $state<string | null>(null);
  let templateNameInput: HTMLInputElement | undefined;

  function toggleAddTemplate() {
    showAddTemplate = !showAddTemplate;
    if (showAddTemplate) {
      setTimeout(() => templateNameInput?.focus(), 0);
    }
  }

  // Click-outside handler for menu
  $effect(() => {
    if (showTemplateMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.template-menu') && !target.closest('.template-menu-btn')) {
          showTemplateMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  function addTemplate() {
    if (!newTemplateName.trim()) return;

    if (!config.templates) {
      config.templates = {};
    }

    const templateName = newTemplateName.trim();
    if (config.templates[templateName]) {
      alert(`Template "${templateName}" already exists!`);
      return;
    }

    config.templates[templateName] = {};
    newTemplateName = "";
    showAddTemplate = false;
    // Select the newly added template
    onTemplateSelected(templateName);
  }

  async function deleteTemplate(templateName: string) {
    showTemplateMenu = null;

    const confirmed = await ask(`Delete template "${templateName}"?`, {
      title: 'Confirm Delete',
      kind: 'warning'
    });

    if (confirmed) {
      delete config.templates[templateName];
      if (currentTemplate === templateName) {
        onTemplateSelected(null);
      }
    }
  }

  function duplicateTemplate(templateName: string) {
    const original = config.templates[templateName];
    let newName = `${templateName}_copy`;
    let counter = 1;
    while (config.templates[newName]) {
      newName = `${templateName}_copy${counter}`;
      counter++;
    }

    // Deep clone the template
    config.templates[newName] = JSON.parse(JSON.stringify(original));
    onTemplateSelected(newName);
    showTemplateMenu = null;
  }

  function startRename(templateName: string) {
    renamingTemplate = templateName;
    renameTemplateName = templateName;
    showTemplateMenu = null;
  }

  function renameTemplate(oldName: string) {
    if (!renameTemplateName.trim() || renameTemplateName === oldName) {
      renameTemplateName = "";
      renamingTemplate = null;
      return;
    }

    if (config.templates[renameTemplateName]) {
      alert(`Template "${renameTemplateName}" already exists!`);
      renameTemplateName = "";
      renamingTemplate = null;
      return;
    }

    // Rebuild object preserving order
    const newTemplates: any = {};
    for (const key of Object.keys(config.templates)) {
      if (key === oldName) {
        newTemplates[renameTemplateName] = config.templates[oldName];
      } else {
        newTemplates[key] = config.templates[key];
      }
    }
    config.templates = newTemplates;

    if (currentTemplate === oldName) {
      onTemplateSelected(renameTemplateName);
    }

    renameTemplateName = "";
    renamingTemplate = null;
  }
</script>

<div class="template-list">
  <div class="header">
    <h3>Templates</h3>
    <button class="add-btn" onclick={toggleAddTemplate}>+</button>
  </div>

  {#if showAddTemplate}
    <div class="add-template">
      <input
        type="text"
        bind:this={templateNameInput}
        bind:value={newTemplateName}
        placeholder="Template name"
        onkeydown={(e) => e.key === 'Enter' && addTemplate()}
      />
      <button onclick={addTemplate} title="Add">✓</button>
      <button onclick={() => showAddTemplate = false} title="Cancel">✕</button>
    </div>
  {/if}

  <div class="separator"></div>

  <div class="templates">
    {#each templates as template}
      <div class="template-row">
        {#if renamingTemplate === template}
          <input
            type="text"
            bind:value={renameTemplateName}
            class="rename-input"
            onkeydown={(e) => {
              if (e.key === 'Enter') renameTemplate(template);
              if (e.key === 'Escape') renameTemplateName = "";
            }}
            onblur={() => renameTemplate(template)}
            onmousedown={(e) => e.stopPropagation()}
            autofocus
          />
        {:else}
          <button
            class="template-item"
            class:active={template === currentTemplate && selectedButton === null}
            onclick={() => onTemplateSelected(template)}
          >
            {template}
          </button>
          <button
            class="template-menu-btn"
            onclick={(e) => {
              e.stopPropagation();
              showTemplateMenu = showTemplateMenu === template ? null : template;
            }}
          >
            ⋮
          </button>
        {/if}

        {#if showTemplateMenu === template}
          <div class="template-menu">
            <button onclick={(e) => { e.stopPropagation(); startRename(template); }}>✏️ Rename</button>
            <button onclick={(e) => { e.stopPropagation(); duplicateTemplate(template); }}>📋 Duplicate</button>
            <button class="danger" onclick={(e) => { e.stopPropagation(); deleteTemplate(template); }}>🗑️ Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .template-list {
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

  .add-template {
    display: flex;
    gap: 4px;
    margin-top: 12px;
    margin-bottom: 12px;
  }

  .separator {
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 16px;
  }

  .add-template input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .add-template button {
    padding: 6px 12px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-template button:first-of-type {
    background-color: #2d7d46;
  }

  .add-template button:first-of-type:hover {
    background-color: #3a9d5a;
  }

  .add-template button:last-child {
    background-color: #7a2d2d;
  }

  .add-template button:last-child:hover {
    background-color: #9a3d3d;
  }

  .templates {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .template-row {
    position: relative;
    display: flex;
    gap: 4px;
  }

  .template-item {
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

  .template-item:hover {
    background-color: #4a4a4a;
  }

  .template-item.active {
    background-color: #354a5f;
    border-color: #5b9bd5;
  }

  .template-menu-btn {
    width: 28px;
    padding: 4px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
  }

  .template-menu-btn:hover {
    background-color: #4a4a4a;
    color: #cccccc;
  }

  .template-menu {
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

  .template-menu button {
    padding: 8px 12px;
    background: none;
    color: #cccccc;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }

  .template-menu button:hover {
    background-color: #3c3c3c;
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
</style>
