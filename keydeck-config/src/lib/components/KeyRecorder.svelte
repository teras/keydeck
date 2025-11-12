<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  interface Props {
    value: string;
    onUpdate: (value: string) => void;
    disabled?: boolean;
  }

  let { value, onUpdate, disabled = false }: Props = $props();

  let isRecording = $state(false);
  let recordedKeys = $state<string[]>([]);
  let inputElement = $state<HTMLInputElement | undefined>();
  let isEditing = $state(false);

  // Map of special keys to backend-compatible names
  // Backend is case-insensitive, so we use CamelCase for beauty
  // Only map keys where the name actually differs (not just case)
  const keyDisplayNames: Record<string, string> = {
    // Modifiers - map browser codes to backend names
    'ControlLeft': 'LCtrl',
    'ControlRight': 'RCtrl',
    'ShiftLeft': 'LShift',
    'ShiftRight': 'RShift',
    'AltLeft': 'LAlt',
    'AltRight': 'RAlt',

    // Super/Windows/Command key - varies by OS
    'MetaLeft': 'LSuper',
    'MetaRight': 'RSuper',
    'OSLeft': 'LSuper',
    'OSRight': 'RSuper',
    'SuperLeft': 'LSuper',
    'SuperRight': 'RSuper',

    // Generic modifiers (fallback without side)
    'Control': 'Ctrl',
    'Meta': 'Super',
    'OS': 'Super',

    // Special keys that have different names
    'Escape': 'Esc',
    'ContextMenu': 'Menu',
    'AltGraph': 'AltGr',

    // Punctuation and symbol keys - map code to symbol
    'Backquote': '`',
    'Minus': '-',
    'Equal': '=',
    'BracketLeft': '[',
    'BracketRight': ']',
    'Backslash': '\\',
    'Semicolon': ';',
    'Quote': '\'',
    'Comma': ',',
    'Period': '.',
    'Slash': '/',
    'IntlBackslash': '\\',
  };

  function getKeyName(event: KeyboardEvent): string {
    // Use code for better key identification
    const code = event.code;

    // Check if it's in our display names map
    if (keyDisplayNames[code]) {
      return keyDisplayNames[code];
    }

    // For letter keys (KeyA-KeyZ), extract just the letter
    if (code.startsWith('Key')) {
      return code.substring(3).toLowerCase();
    }

    // For digit keys (Digit0-Digit9), extract just the number
    if (code.startsWith('Digit')) {
      return code.substring(5);
    }

    // For regular single character keys, use the key value
    if (event.key.length === 1 && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
      return event.key.toLowerCase();
    }

    // Fallback to code
    return code;
  }

  function startRecording(event: MouseEvent) {
    if (disabled) return;

    event.preventDefault();
    event.stopPropagation();

    isRecording = true;
    isEditing = false;
    recordedKeys = [];

    // Focus the input to capture key events
    setTimeout(() => inputElement?.focus(), 0);
  }

  function toggleEdit() {
    if (disabled) return;
    isEditing = true;
    setTimeout(() => inputElement?.focus(), 0);
  }

  function stopRecording() {
    if (!isRecording) return;

    isRecording = false;

    // Build the key combination string
    if (recordedKeys.length > 0) {
      const keyString = recordedKeys.join('+');
      onUpdate(keyString);
    }

    recordedKeys = [];
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (isRecording) {
      event.preventDefault();
      event.stopPropagation();

      const keyName = getKeyName(event);

      // Add key if not already in the list
      if (!recordedKeys.includes(keyName)) {
        recordedKeys = [...recordedKeys, keyName];
      }
    } else if (isEditing) {
      // Allow normal typing when editing
      if (event.key === 'Enter') {
        event.preventDefault();
        inputElement?.blur();
      } else if (event.key === 'Escape') {
        event.preventDefault();
        // Restore original value
        if (inputElement) {
          inputElement.value = value;
        }
        isEditing = false;
      }
    }
  }

  function handleKeyUp(event: KeyboardEvent) {
    if (!isRecording) return;

    event.preventDefault();
    event.stopPropagation();

    // When any key is released, stop recording
    // This allows users to record complex combinations
    stopRecording();
  }

  function handleInput(event: Event) {
    if (isEditing) {
      const target = event.target as HTMLInputElement;
      onUpdate(target.value);
    }
  }

  function handleBlur() {
    if (isRecording) {
      stopRecording();
    }
    isEditing = false;
  }

  // Handle clicks outside while recording
  $effect(() => {
    if (isRecording) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!inputElement?.contains(target)) {
          stopRecording();
        }
      };

      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  let placeholderText = $derived(
    isRecording
      ? (recordedKeys.length > 0 ? recordedKeys.join('+') : 'Press keys...')
      : (value || 'Type or click record button')
  );
</script>

<div class="key-recorder-wrapper" class:disabled={disabled}>
  {#if !disabled}
    <div class="button-group">
      {#if isRecording}
        <span class="recording-indicator">●</span>
      {:else}
        <button
          class="record-btn"
          onclick={startRecording}
          title="Record keys"
          type="button"
        >
          <span class="record-dot"></span>
        </button>
      {/if}
    </div>
  {/if}
  <input
    type="text"
    class="key-input"
    class:recording={isRecording}
    class:editing={isEditing}
    bind:this={inputElement}
    value={isRecording ? recordedKeys.join('+') : value}
    placeholder={placeholderText}
    onclick={() => { if (!disabled && !isRecording) isEditing = true; }}
    oninput={handleInput}
    onkeydown={handleKeyDown}
    onkeyup={handleKeyUp}
    onblur={handleBlur}
    disabled={disabled}
    readonly={isRecording}
  />
  {#if !disabled && value && !isRecording}
    <button
      class="clear-btn"
      onclick={(e) => {
        e.stopPropagation();
        onUpdate('');
      }}
      title="Clear"
      type="button"
    >
      ✕
    </button>
  {/if}
</div>

<style>
  .key-recorder-wrapper {
    display: flex;
    align-items: center;
    gap: 4px;
    position: relative;
  }

  .key-recorder-wrapper.disabled {
    opacity: 0.5;
    pointer-events: none;
  }

  .key-input {
    flex: 1;
    padding: 6px 8px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 3px;
    font-size: 12px;
    font-family: 'Courier New', monospace;
    transition: all 0.2s;
  }

  .key-input:focus {
    outline: none;
    border-color: #0e639c;
  }

  .key-input.recording {
    border-color: #4ec9b0;
    background-color: #2a3a33;
    color: #4ec9b0;
    font-weight: 600;
    animation: pulse 1.5s ease-in-out infinite;
  }

  .key-input.editing {
    border-color: #0e639c;
    background-color: #2a2a2a;
  }

  .key-input::placeholder {
    color: #666;
    font-style: italic;
  }

  .key-input.recording::placeholder {
    color: #3a9980;
  }

  @keyframes pulse {
    0%, 100% {
      border-color: #4ec9b0;
    }
    50% {
      border-color: #3a9980;
    }
  }

  .button-group {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .record-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    background: radial-gradient(circle, #cc0000 0%, #990000 100%);
    border: 1px solid #660000;
    border-radius: 50%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3), inset 0 1px 1px rgba(255, 255, 255, 0.2);
  }

  .record-btn:hover {
    background: radial-gradient(circle, #dd0000 0%, #aa0000 100%);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.4), inset 0 1px 1px rgba(255, 255, 255, 0.3);
  }

  .record-btn:active {
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.4), inset 0 2px 3px rgba(0, 0, 0, 0.3);
  }

  .record-dot {
    width: 6px;
    height: 6px;
    background-color: #ffffff;
    border-radius: 50%;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .recording-indicator {
    color: #e74c3c;
    font-size: 16px;
    padding: 0 8px;
    animation: blink 1s ease-in-out infinite;
  }

  @keyframes blink {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }

  .clear-btn {
    padding: 4px 8px;
    background-color: #7a2d2d;
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
    line-height: 1;
    transition: background-color 0.2s;
  }

  .clear-btn:hover {
    background-color: #9a3d3d;
  }
</style>
