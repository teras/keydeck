<script lang="ts">
  interface Props {
    config: any;
    onMacroSelected?: (macroName: string | null) => void;
  }

  let { config, onMacroSelected }: Props = $props();

  let macros = $derived(Object.keys(config.macros || {}));
  let showAddMacro = $state(false);
  let newMacroName = $state("");
  let showMacroMenu = $state<string | null>(null);

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

    config.macros[newMacroName] = {
      actions: []
    };
    newMacroName = "";
    showAddMacro = false;
    if (onMacroSelected) {
      onMacroSelected(newMacroName);
    }
  }

  function deleteMacro(macroName: string) {
    if (confirm(`Delete macro "${macroName}"?`)) {
      delete config.macros[macroName];
      showMacroMenu = null;
    }
  }
</script>

<div class="macro-list">
  <div class="header">
    <h3>Macros</h3>
    <button class="add-btn" onclick={() => showAddMacro = !showAddMacro}>+</button>
  </div>

  {#if showAddMacro}
    <div class="add-macro">
      <input
        type="text"
        bind:value={newMacroName}
        placeholder="Macro name"
        onkeydown={(e) => e.key === 'Enter' && addMacro()}
      />
      <button onclick={addMacro}>Add</button>
      <button onclick={() => showAddMacro = false}>Cancel</button>
    </div>
  {/if}

  <div class="macros">
    {#each macros as macro}
      <div class="macro-row">
        <button class="macro-item">
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

        {#if showMacroMenu === macro}
          <div class="macro-menu">
            <button onclick={() => deleteMacro(macro)}>Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .macro-list {
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

  .add-macro {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
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
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .add-macro button:hover {
    background-color: #1177bb;
  }

  .add-macro button:last-child {
    background-color: #555;
  }

  .add-macro button:last-child:hover {
    background-color: #666;
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
