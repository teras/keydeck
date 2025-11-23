<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { emojiList } from '$lib/data/emojis';

  interface Props {
    value: string;
    onUpdate: (value: string) => void;
    disabled?: boolean;
    placeholder?: string;
    multiline?: boolean;
    rows?: number;
  }

  let { value, onUpdate, disabled = false, placeholder = '', multiline = false, rows = 3 }: Props = $props();

  let inputElement = $state<HTMLInputElement | HTMLTextAreaElement | undefined>();
  let showSuggestions = $state(false);
  let suggestions = $state<Array<{emoji: string, name: string}>>([]);
  let selectedIndex = $state(0);
  let searchStart = $state(0);

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement | HTMLTextAreaElement;
    const newValue = target.value;
    const cursorPos = target.selectionStart || 0;

    onUpdate(newValue);

    // Check if we should show emoji suggestions
    const textBeforeCursor = newValue.slice(0, cursorPos);
    const lastColonIndex = textBeforeCursor.lastIndexOf(':');

    if (lastColonIndex !== -1) {
      const searchText = textBeforeCursor.slice(lastColonIndex + 1);

      // Only show suggestions if:
      // 1. There's a colon
      // 2. The text after colon doesn't contain spaces
      // 3. The cursor is right after the search text
      if (!searchText.includes(' ') && searchText.length >= 0) {
        searchStart = lastColonIndex;
        const filtered = emojiList.filter(item =>
          item.name.toLowerCase().includes(searchText.toLowerCase())
        ).slice(0, 10); // Limit to 10 suggestions

        if (filtered.length > 0) {
          suggestions = filtered;
          showSuggestions = true;
          selectedIndex = 0;
          return;
        }
      }
    }

    showSuggestions = false;
  }

  function insertEmoji(emoji: string, name: string) {
    if (!inputElement) return;

    const cursorPos = inputElement.selectionStart || 0;
    const textBefore = value.slice(0, searchStart);
    const textAfter = value.slice(cursorPos);
    const newValue = textBefore + emoji + textAfter;
    const newCursorPos = searchStart + emoji.length;

    onUpdate(newValue);
    showSuggestions = false;

    // Restore focus and cursor position
    setTimeout(() => {
      if (inputElement) {
        inputElement.focus();
        inputElement.setSelectionRange(newCursorPos, newCursorPos);
      }
    }, 0);
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!showSuggestions) return;

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      selectedIndex = (selectedIndex + 1) % suggestions.length;
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      selectedIndex = (selectedIndex - 1 + suggestions.length) % suggestions.length;
    } else if (event.key === 'Enter' && suggestions.length > 0) {
      event.preventDefault();
      const selected = suggestions[selectedIndex];
      insertEmoji(selected.emoji, selected.name);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      showSuggestions = false;
    }
  }

  function handleBlur(event: FocusEvent) {
    // Delay to allow clicking on suggestions
    setTimeout(() => {
      // Check if the new focus target is within the suggestions
      const relatedTarget = event.relatedTarget as HTMLElement;
      if (!relatedTarget || !relatedTarget.closest('.emoji-suggestions')) {
        showSuggestions = false;
      }
    }, 200);
  }
</script>

<div class="emoji-autocomplete-wrapper">
  {#if multiline}
    <textarea
      bind:this={inputElement}
      value={value}
      {placeholder}
      {disabled}
      {rows}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      onblur={handleBlur}
    ></textarea>
  {:else}
    <input
      type="text"
      bind:this={inputElement}
      value={value}
      {placeholder}
      {disabled}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      onblur={handleBlur}
    />
  {/if}

  {#if showSuggestions && suggestions.length > 0}
    <div class="emoji-suggestions">
      {#each suggestions as suggestion, index}
        <button
          type="button"
          class="emoji-suggestion"
          class:selected={index === selectedIndex}
          onmousedown={(e) => {
            e.preventDefault();
            insertEmoji(suggestion.emoji, suggestion.name);
          }}
        >
          <span class="emoji">{suggestion.emoji}</span>
          <span class="name">:{suggestion.name}:</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .emoji-autocomplete-wrapper {
    position: relative;
    width: 100%;
  }

  input,
  textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 6px 8px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 3px;
    font-size: 12px;
    font-family: 'Courier New', monospace;
  }

  input:focus,
  textarea:focus {
    outline: none;
    border-color: #0e639c;
  }

  textarea {
    resize: vertical;
    font-family: 'Courier New', monospace;
  }

  .emoji-suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    background-color: #1e1e1e;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    max-height: 300px;
    overflow-y: auto;
    z-index: 1000;
  }

  .emoji-suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    color: #cccccc;
    cursor: pointer;
    text-align: left;
    transition: background-color 0.1s;
  }

  .emoji-suggestion:hover,
  .emoji-suggestion.selected {
    background-color: #2a2a2a;
  }

  .emoji {
    font-size: 18px;
    flex-shrink: 0;
  }

  .name {
    font-size: 12px;
    font-family: 'Courier New', monospace;
    color: #888;
  }

  .emoji-suggestion.selected .name {
    color: #4ec9b0;
  }
</style>
