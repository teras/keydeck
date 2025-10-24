<script lang="ts">
  /**
   * Reusable tri-state checkbox component
   *
   * States:
   * - undefined: Inherit/Auto-detect (shows inner square)
   * - true: Enabled (shows checkmark with blue background)
   * - false: Disabled (shows empty checkbox)
   *
   * Props:
   * - value: boolean | undefined - current state
   * - label: string - label text to display
   * - onToggle: (value: boolean | undefined) => void - callback when toggled
   * - inheritLabel?: string - tooltip for inherit state (default: "Auto-detect")
   * - trueLabel?: string - tooltip for true state (default: "Enabled")
   * - falseLabel?: string - tooltip for false state (default: "Disabled")
   * - disabled?: boolean - if true, checkbox is read-only (default: false)
   */

  export let value: boolean | undefined = undefined;
  export let label: string;
  export let onToggle: (newValue: boolean | undefined) => void;
  export let inheritLabel: string = "Auto-detect";
  export let trueLabel: string = "Enabled";
  export let falseLabel: string = "Disabled";
  export let disabled: boolean = false;

  function toggle() {
    if (value === undefined) {
      // undefined → true
      onToggle(true);
    } else if (value === true) {
      // true → false
      onToggle(false);
    } else {
      // false → true
      onToggle(true);
    }
  }

  function reset() {
    onToggle(undefined);
  }

  $: tooltip = value === undefined ? inheritLabel : value === true ? trueLabel : falseLabel;
</script>

<div class="tristate-control">
  <label class="checkbox-label">
    <button
      type="button"
      class="tristate-checkbox"
      class:state-inherit={value === undefined}
      class:state-true={value === true}
      class:state-false={value === false}
      onclick={toggle}
      title={tooltip}
      disabled={disabled}
    >
      {#if value === true}
        ✓
      {:else if value === undefined}
        <span class="inner-square"></span>
      {/if}
    </button>
    <span class="checkbox-text">{label}</span>
  </label>
  {#if value !== undefined && !disabled}
    <button
      type="button"
      class="reset-btn"
      onclick={reset}
      title="Reset to {inheritLabel}"
    >
      ✕
    </button>
  {/if}
</div>

<style>
  .tristate-control {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    padding: 0;
    margin: 0;
  }

  .tristate-checkbox {
    width: 18px;
    height: 18px;
    margin: 0;
    padding: 0;
    cursor: pointer;
    background-color: #2a2a2a;
    border: 1px solid #555;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: bold;
    transition: all 0.2s;
    color: white;
  }

  .tristate-checkbox:hover:not(:disabled) {
    border-color: #0e639c;
  }

  .tristate-checkbox:disabled {
    opacity: 0.6;
    cursor: default;
  }

  /* Inherit state (undefined) - inner square */
  .tristate-checkbox.state-inherit {
    background-color: #2a2a2a;
    border-color: #555;
  }

  .tristate-checkbox.state-inherit:hover:not(:disabled) {
    background-color: #333;
    border-color: #0e639c;
  }

  .inner-square {
    display: block;
    width: 14px;
    height: 14px;
    background-color: #4a6d8c;
    border-radius: 1px;
  }

  /* Enabled state (true) - blue with checkmark */
  .tristate-checkbox.state-true {
    background-color: #0e639c;
    border-color: #0e639c;
    color: white;
  }

  .tristate-checkbox.state-true:hover:not(:disabled) {
    background-color: #1177bb;
    border-color: #1177bb;
  }

  /* Disabled state (false) - blank/empty */
  .tristate-checkbox.state-false {
    background-color: #2a2a2a;
    border-color: #555;
  }

  .tristate-checkbox.state-false:hover:not(:disabled) {
    background-color: #333;
    border-color: #0e639c;
  }

  .checkbox-text {
    color: #888;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    user-select: none;
  }

  .reset-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    background-color: #7a2d2d;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s;
    flex-shrink: 0;
  }

  .reset-btn:hover {
    background-color: #9a3d3d;
  }
</style>
