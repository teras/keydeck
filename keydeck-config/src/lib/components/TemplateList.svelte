<script lang="ts">
  interface Props {
    config: any;
    currentTemplate: string | null;
    onTemplateSelected: (templateName: string | null) => void;
  }

  let { config, currentTemplate, onTemplateSelected }: Props = $props();

  let templates = $derived(Object.keys(config.templates || {}));
  let showTemplateMenu = $state<string | null>(null);
  let showAddTemplate = $state(false);
  let newTemplateName = $state("");
  let renameTemplateName = $state("");

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

    config.templates[newTemplateName] = {};
    newTemplateName = "";
    showAddTemplate = false;
  }

  function deleteTemplate(templateName: string) {
    if (confirm(`Delete template "${templateName}"?`)) {
      delete config.templates[templateName];
      if (currentTemplate === templateName) {
        onTemplateSelected(null);
      }
      showTemplateMenu = null;
    }
  }

  function startRename(templateName: string) {
    renameTemplateName = templateName;
    showTemplateMenu = null;
  }

  function renameTemplate(oldName: string) {
    if (!renameTemplateName.trim() || renameTemplateName === oldName) {
      renameTemplateName = "";
      return;
    }

    const templateData = config.templates[oldName];
    config.templates[renameTemplateName] = templateData;
    delete config.templates[oldName];

    if (currentTemplate === oldName) {
      onTemplateSelected(renameTemplateName);
    }

    renameTemplateName = "";
  }
</script>

<div class="template-list">
  <div class="header">
    <h3>Templates</h3>
    <button class="add-btn" onclick={() => showAddTemplate = !showAddTemplate}>+</button>
  </div>

  {#if showAddTemplate}
    <div class="add-template">
      <input
        type="text"
        bind:value={newTemplateName}
        placeholder="Template name"
        onkeydown={(e) => e.key === 'Enter' && addTemplate()}
      />
      <button onclick={addTemplate}>Add</button>
      <button onclick={() => showAddTemplate = false}>Cancel</button>
    </div>
  {/if}

  <div class="templates">
    {#each templates as template}
      <div class="template-row">
        {#if renameTemplateName && renameTemplateName === template}
          <input
            type="text"
            bind:value={renameTemplateName}
            class="rename-input"
            onkeydown={(e) => e.key === 'Enter' && renameTemplate(template)}
            onblur={() => renameTemplate(template)}
            autofocus
          />
        {:else}
          <button
            class="template-item"
            class:active={template === currentTemplate}
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
            <button onclick={() => startRename(template)}>Rename</button>
            <button onclick={() => deleteTemplate(template)}>Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .template-list {
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

  .add-template {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
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
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .add-template button:hover {
    background-color: #1177bb;
  }

  .add-template button:last-child {
    background-color: #555;
  }

  .add-template button:last-child:hover {
    background-color: #666;
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
    background-color: #5a4a2a;
    border-color: #d7a964;
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
