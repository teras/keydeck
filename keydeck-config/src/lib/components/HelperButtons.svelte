<script lang="ts">
  interface Props {
    isEditMode: boolean;
    onHomeClick: () => void;
    onToggleMode: () => void;
  }

  let { isEditMode, onHomeClick, onToggleMode }: Props = $props();
</script>

<div class="helper-buttons">
  <div class="left-controls">
    <a
      class="helper-link home-link"
      onclick={onHomeClick}
      title="Jump to main page"
      role="button"
      tabindex="0"
    >
      🏠 Home
    </a>

    <div class="mode-toggle-container">
      <button
        class="mode-toggle-option"
        class:active={isEditMode}
        onclick={() => !isEditMode && onToggleMode()}
        title="Edit mode"
      >
        ✏️ Edit
      </button>
      <button
        class="mode-toggle-option"
        class:active={!isEditMode}
        onclick={() => isEditMode && onToggleMode()}
        title="Navigate mode"
      >
        ▶️ Navigate
      </button>
    </div>
  </div>
</div>

<style>
  .helper-buttons {
    display: flex;
    justify-content: flex-start;
    align-items: center;
    padding: 12px 20px;
    box-sizing: border-box;
  }

  .left-controls {
    display: flex;
    align-items: center;
    gap: 40px;
  }

  .helper-link {
    background: none;
    border: none;
    color: #cccccc;
    cursor: pointer;
    font-size: 16px;
    font-weight: 600;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: all 0.2s ease;
    padding: 6px 12px;
    user-select: none;
    border-radius: 4px;
  }

  .helper-link:hover {
    background-color: #2a2a2a;
    color: #4a9eff;
  }

  .helper-link:active {
    transform: scale(0.98);
  }

  .mode-toggle-container {
    display: flex;
    background-color: #2a2a2a;
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
  }

  .mode-toggle-option {
    padding: 6px 16px;
    background-color: #3c3c3c;
    border: none;
    color: #cccccc;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    border-radius: 4px;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 6px;
    user-select: none;
  }

  .mode-toggle-option:hover:not(.active) {
    color: #ffffff;
    background-color: #4a4a4a;
  }

  .mode-toggle-option.active {
    background-color: #0e639c;
    color: #ffffff;
    cursor: pointer;
    font-weight: 600;
  }

  .mode-toggle-option:active {
    transform: scale(0.98);
  }
</style>
