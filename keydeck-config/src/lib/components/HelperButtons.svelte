<script lang="ts">
  interface Props {
    isEditMode: boolean;
    onHomeClick: () => void;
    onToggleMode: () => void;
  }

  let { isEditMode, onHomeClick, onToggleMode }: Props = $props();
</script>

<div class="helper-buttons">
  <a
    class="helper-link home-link"
    onclick={onHomeClick}
    title="Jump to main page"
    role="button"
    tabindex="0"
  >
    🏠
  </a>

  <span class="mode-label">Mode:</span>
  <a
    class="helper-link mode-link"
    onclick={onToggleMode}
    title={isEditMode ? "Switch to play mode" : "Switch to edit mode"}
    role="button"
    tabindex="0"
  >
    {isEditMode ? '✏️' : '▶️'}
  </a>
</div>

<style>
  .helper-buttons {
    display: flex;
    gap: 20px;
    justify-content: center;
    margin-top: 20px;
    padding: 12px;
  }

  .helper-link {
    background: none;
    border: none;
    color: #888;
    cursor: pointer;
    font-size: 24px;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
    padding: 4px 8px;
    user-select: none;
  }

  .helper-link:hover {
    color: #0e639c;
    transform: scale(1.15);
  }

  .helper-link:active {
    transform: scale(1.0);
  }

  .home-link:hover {
    color: #4a9eff;
  }

  .mode-link:hover {
    color: #66bb6a;
  }

  .mode-label {
    color: #888;
    font-size: 14px;
    display: inline-flex;
    align-items: center;
    user-select: none;
    margin-left: 20px;
  }
</style>
