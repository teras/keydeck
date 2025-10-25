<script lang="ts">
  import ColorPicker from './ColorPicker.svelte';

  interface Props {
    value: string;
    placeholder?: string;
    onUpdate: (value: string) => void;
    disabled?: boolean;
    readonly?: boolean;
    reference?: boolean;
    inherited?: boolean;
  }

  let {
    value,
    placeholder = "0xRRGGBB",
    onUpdate,
    disabled = false,
    readonly = false,
    reference = false,
    inherited = false
  }: Props = $props();
</script>

<div
  class="color-item"
  class:readonly={readonly}
  class:reference={reference}
  class:inherited={inherited}
>
  <div class="color-info">
    <ColorPicker
      {value}
      {placeholder}
      {onUpdate}
      disabled={disabled || readonly}
    />
  </div>
</div>

<style>
  .color-item {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    padding: 8px;
    background-color: #3c3c3c;
    border: 1px solid #555;
    border-radius: 4px;
    gap: 8px;
    min-width: 0;
  }

  .color-item.readonly.reference {
    background-color: #3e3a4a;
    border-color: #4f4565;
  }

  .color-item.readonly.inherited {
    background-color: #4a4238;
    border-color: #5f5545;
  }

  .color-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }
</style>
