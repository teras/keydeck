<script lang="ts">
  interface Props {
    config: any;
    onButtonDefSelected?: (buttonName: string | null) => void;
  }

  let { config, onButtonDefSelected }: Props = $props();

  let buttonDefs = $derived(Object.keys(config.buttons || {}));
  let showAddButton = $state(false);
  let newButtonName = $state("");
  let showButtonMenu = $state<string | null>(null);

  // Click-outside handler for menu
  $effect(() => {
    if (showButtonMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.button-menu') && !target.closest('.button-menu-btn')) {
          showButtonMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  function addButton() {
    if (!newButtonName.trim()) return;

    if (!config.buttons) {
      config.buttons = {};
    }

    config.buttons[newButtonName] = {
      actions: []
    };
    newButtonName = "";
    showAddButton = false;
    if (onButtonDefSelected) {
      onButtonDefSelected(newButtonName);
    }
  }

  function deleteButton(buttonName: string) {
    if (confirm(`Delete button definition "${buttonName}"?`)) {
      delete config.buttons[buttonName];
      showButtonMenu = null;
    }
  }
</script>

<div class="button-list">
  <div class="header">
    <h3>Button Definitions</h3>
    <button class="add-btn" onclick={() => showAddButton = !showAddButton}>+</button>
  </div>

  {#if showAddButton}
    <div class="add-button">
      <input
        type="text"
        bind:value={newButtonName}
        placeholder="Button name"
        onkeydown={(e) => e.key === 'Enter' && addButton()}
      />
      <button onclick={addButton}>Add</button>
      <button onclick={() => showAddButton = false}>Cancel</button>
    </div>
  {/if}

  <div class="buttons">
    {#each buttonDefs as button}
      <div class="button-row">
        <button class="button-item">
          {button}
        </button>
        <button
          class="button-menu-btn"
          onclick={(e) => {
            e.stopPropagation();
            showButtonMenu = showButtonMenu === button ? null : button;
          }}
        >
          ⋮
        </button>

        {#if showButtonMenu === button}
          <div class="button-menu">
            <button onclick={() => deleteButton(button)}>Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .button-list {
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

  .add-button {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
  }

  .add-button input {
    flex: 1;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 12px;
  }

  .add-button button {
    padding: 6px 12px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .add-button button:hover {
    background-color: #1177bb;
  }

  .add-button button:last-child {
    background-color: #555;
  }

  .add-button button:last-child:hover {
    background-color: #666;
  }

  .buttons {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .button-row {
    position: relative;
    display: flex;
    gap: 4px;
  }

  .button-item {
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

  .button-item:hover {
    background-color: #4a4a4a;
  }

  .button-menu-btn {
    width: 28px;
    padding: 4px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
  }

  .button-menu-btn:hover {
    background-color: #4a4a4a;
    color: #cccccc;
  }

  .button-menu {
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

  .button-menu button {
    padding: 8px 12px;
    background: none;
    color: #cccccc;
    border: none;
    text-align: left;
    cursor: pointer;
    font-size: 12px;
  }

  .button-menu button:hover {
    background-color: #3c3c3c;
  }
</style>
