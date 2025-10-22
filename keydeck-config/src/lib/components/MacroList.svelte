<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog';

  interface Props {
    config: any;
    currentMacro: string | null;
    onMacroSelected: (macroName: string | null) => void;
  }

  let { config, currentMacro, onMacroSelected }: Props = $props();

  let macros = $derived(Object.keys(config.macros || {}));
  let showAddMacro = $state(false);
  let newMacroName = $state("");
  let showMacroMenu = $state<string | null>(null);
  let macroNameInput: HTMLInputElement | undefined;
  let renameMacroName = $state("");
  let renamingMacro = $state<string | null>(null);

  function toggleAddMacro() {
    showAddMacro = !showAddMacro;
    if (showAddMacro) {
      setTimeout(() => macroNameInput?.focus(), 0);
    }
  }

  // Click-outside handler for menu
  $effect(() => {
    if (showMacroMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.macro-menu') && !target.closest('.macro-menu-btn')) {
          showMacroMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  function addMacro() {
    if (!newMacroName.trim()) return;

    if (!config.macros) {
      config.macros = {};
    }

    const macroName = newMacroName.trim();
    if (config.macros[macroName]) {
      alert(`Macro "${macroName}" already exists!`);
      return;
    }

    config.macros[macroName] = {
      actions: []
    };
    newMacroName = "";
    showAddMacro = false;
    // Select the newly added macro
    if (onMacroSelected) {
      onMacroSelected(macroName);
    }
  }

  async function deleteMacro(macroName: string) {
    showMacroMenu = null;

    const confirmed = await ask(`Delete macro "${macroName}"?`, {
      title: 'Confirm Delete',
      kind: 'warning'
    });

    if (confirmed) {
      delete config.macros[macroName];
      if (currentMacro === macroName) {
        onMacroSelected(null);
      }
    }
  }

  function duplicateMacro(macroName: string) {
    const original = config.macros[macroName];
    let newName = `${macroName}_copy`;
    let counter = 1;
    while (config.macros[newName]) {
      newName = `${macroName}_copy${counter}`;
      counter++;
    }

    config.macros[newName] = JSON.parse(JSON.stringify(original));
    onMacroSelected(newName);
    showMacroMenu = null;
  }

  function startRename(macroName: string) {
    renamingMacro = macroName;
    renameMacroName = macroName;
    showMacroMenu = null;
  }

  function renameMacro(oldName: string) {
    if (!renameMacroName.trim() || renameMacroName === oldName) {
      renameMacroName = "";
      renamingMacro = null;
      return;
    }

    if (config.macros[renameMacroName]) {
      alert(`Macro "${renameMacroName}" already exists!`);
      renameMacroName = "";
      renamingMacro = null;
      return;
    }

    // Rebuild object preserving order
    const newMacros: any = {};
    for (const key of Object.keys(config.macros)) {
      if (key === oldName) {
        newMacros[renameMacroName] = config.macros[oldName];
      } else {
        newMacros[key] = config.macros[key];
      }
    }
    config.macros = newMacros;

    if (currentMacro === oldName) {
      onMacroSelected(renameMacroName);
    }

    renameMacroName = "";
    renamingMacro = null;
  }
</script>

<div class="macro-list">
  <div class="header">
    <h3>Macros</h3>
    <button class="add-btn" onclick={toggleAddMacro}>+</button>
  </div>

  {#if showAddMacro}
    <div class="add-macro">
      <input
        type="text"
        bind:this={macroNameInput}
        bind:value={newMacroName}
        placeholder="Macro name"
        onkeydown={(e) => e.key === 'Enter' && addMacro()}
      />
      <button onclick={addMacro} title="Add">✓</button>
      <button onclick={() => showAddMacro = false} title="Cancel">✕</button>
    </div>
  {/if}

  <div class="separator"></div>

  <div class="macros">
    {#each macros as macro}
      <div class="macro-row">
        {#if renamingMacro === macro}
          <input
            type="text"
            bind:value={renameMacroName}
            class="rename-input"
            onkeydown={(e) => {
              if (e.key === 'Enter') renameMacro(macro);
              if (e.key === 'Escape') { renameMacroName = ""; renamingMacro = null; }
            }}
            onblur={() => renameMacro(macro)}
            onmousedown={(e) => e.stopPropagation()}
            autofocus
          />
        {:else}
          <button
            class="macro-item"
            class:active={macro === currentMacro}
            onclick={() => onMacroSelected(macro)}
          >
            {macro}
          </button>
          <button
            class="macro-menu-btn"
            onclick={(e) => {
              e.stopPropagation();
              showMacroMenu = showMacroMenu === macro ? null : macro;
            }}
          >
            ⋮
          </button>
        {/if}

        {#if showMacroMenu === macro}
          <div class="macro-menu">
            <button onclick={(e) => { e.stopPropagation(); startRename(macro); }}>✏️ Rename</button>
            <button onclick={(e) => { e.stopPropagation(); duplicateMacro(macro); }}>📋 Duplicate</button>
            <button class="danger" onclick={(e) => { e.stopPropagation(); deleteMacro(macro); }}>🗑️ Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .macro-list {
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

  .add-macro {
    display: flex;
    gap: 4px;
    margin-top: 12px;
    margin-bottom: 12px;
  }

  .separator {
    border-bottom: 1px solid #3e3e42;
    margin-bottom: 16px;
  }

  .add-macro input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .add-macro button {
    padding: 6px 12px;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-macro button:first-of-type {
    background-color: #2d7d46;
  }

  .add-macro button:first-of-type:hover {
    background-color: #3a9d5a;
  }

  .add-macro button:last-child {
    background-color: #7a2d2d;
  }

  .add-macro button:last-child:hover {
    background-color: #9a3d3d;
  }

  .macros {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .macro-row {
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

  .macro-item {
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

  .macro-item:hover {
    background-color: #4a4a4a;
  }

  .macro-item.active {
    background-color: #354a5f;
    border-color: #5b9bd5;
  }

  .macro-menu-btn {
    width: 28px;
    padding: 4px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
  }

  .macro-menu-btn:hover {
    background-color: #4a4a4a;
    color: #cccccc;
  }

  .macro-menu {
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

  .macro-menu button {
    padding: 8px 12px;
    background: none;
    color: #cccccc;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }

  .macro-menu button:hover {
    background-color: #3c3c3c;
  }
</style>
