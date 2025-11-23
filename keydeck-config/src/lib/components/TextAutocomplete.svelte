<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { emojiList } from '$lib/data/emojis';

  type ServiceSource = 'embedded' | 'user' | 'both';

  interface ServiceSuggestion {
    name: string;
    source?: ServiceSource;
    description?: string;
  }

  interface EmojiSuggestion {
    kind: 'emoji';
    emoji: string;
    name: string;
  }

  interface ServiceSuggestionItem {
    kind: 'service';
    name: string;
    source: ServiceSource;
    description?: string;
  }

  type AutocompleteSuggestion = EmojiSuggestion | ServiceSuggestionItem;

  interface Props {
    value: string;
    onUpdate: (value: string) => void;
    disabled?: boolean;
    placeholder?: string;
    multiline?: boolean;
    rows?: number;
    serviceSuggestions?: ServiceSuggestion[];
  }

  let {
    value,
    onUpdate,
    disabled = false,
    placeholder = '',
    multiline = false,
    rows = 3,
    serviceSuggestions = []
  }: Props = $props();

  let inputElement = $state<HTMLInputElement | HTMLTextAreaElement | undefined>();
  let showSuggestions = $state(false);
  let suggestions = $state<AutocompleteSuggestion[]>([]);
  let selectedIndex = $state(0);
  let searchStart = $state(0);

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement | HTMLTextAreaElement;
    const newValue = target.value;
    const cursorPos = target.selectionStart || 0;

    onUpdate(newValue);

    const textBeforeCursor = newValue.slice(0, cursorPos);

    // Service autocomplete (triggered with ${service:...})
    const servicePrefix = '${service:';
    const lastServiceIndex = textBeforeCursor.lastIndexOf(servicePrefix);

    if (serviceSuggestions.length > 0 && lastServiceIndex !== -1) {
      const serviceSearch = textBeforeCursor.slice(lastServiceIndex + servicePrefix.length);

      // Stop if the current token already closed or contains whitespace
      if (!/\s/.test(serviceSearch) && !serviceSearch.includes('}')) {
        const filteredServices = serviceSuggestions
          .filter(item => item.name.toLowerCase().includes(serviceSearch.toLowerCase()))
          .slice(0, 10)
          .map(item => ({
            kind: 'service',
            name: item.name,
            source: item.source ?? 'user',
            description: item.description
          } as ServiceSuggestionItem));

        if (filteredServices.length > 0) {
          searchStart = lastServiceIndex + servicePrefix.length;
          suggestions = filteredServices;
          showSuggestions = true;
          selectedIndex = 0;
          return;
        }
      }
    }

    // Emoji suggestions (triggered with :)
    const lastColonIndex = textBeforeCursor.lastIndexOf(':');

    if (lastColonIndex !== -1) {
      const searchText = textBeforeCursor.slice(lastColonIndex + 1);

      // Only show suggestions if:
      // 1. There's a colon
      // 2. The text after colon doesn't contain spaces
      // 3. The cursor is right after the search text
      if (!searchText.includes(' ') && searchText.length >= 0) {
        searchStart = lastColonIndex;
        const filtered = emojiList
          .filter(item => item.name.toLowerCase().includes(searchText.toLowerCase()))
          .slice(0, 10)
          .map(item => ({
            kind: 'emoji',
            emoji: item.emoji,
            name: item.name,
          } as EmojiSuggestion));

        if (filtered.length > 0) {
          suggestions = filtered;
          showSuggestions = true;
          selectedIndex = 0;
          return;
        }
      }
    }

    showSuggestions = false;
    suggestions = [];
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

  function insertService(name: string) {
    if (!inputElement) return;

    const cursorPos = inputElement.selectionStart || 0;
    const textBefore = value.slice(0, searchStart);
    const textAfter = value.slice(cursorPos);
    const newValue = textBefore + name + textAfter;
    const newCursorPos = searchStart + name.length;

    onUpdate(newValue);
    showSuggestions = false;

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
      if (selected.kind === 'emoji') {
        insertEmoji(selected.emoji, selected.name);
      } else {
        insertService(selected.name);
      }
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
      if (!relatedTarget || !relatedTarget.closest('.autocomplete-suggestions')) {
        showSuggestions = false;
      }
    }, 200);
  }

  function getServiceSourceLabel(source: ServiceSource): string {
    switch (source) {
      case 'embedded':
        return 'Embedded';
      case 'both':
        return 'Embedded + Custom';
      default:
        return 'Custom';
    }
  }
</script>

<div class="text-autocomplete-wrapper">
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
    <div class="autocomplete-suggestions">
      {#each suggestions as suggestion, index}
        <button
          type="button"
          class="autocomplete-suggestion"
          class:selected={index === selectedIndex}
          onmousedown={(e) => {
            e.preventDefault();
            if (suggestion.kind === 'emoji') {
              insertEmoji(suggestion.emoji, suggestion.name);
            } else {
              insertService(suggestion.name);
            }
          }}
        >
          {#if suggestion.kind === 'emoji'}
            <span class="emoji-icon">{suggestion.emoji}</span>
            <span class="emoji-name">:{suggestion.name}:</span>
          {:else}
            <div class="service-suggestion">
              <div class="service-symbol">$</div>
              <div class="service-details">
                <div class="service-name">{suggestion.name}</div>
                <div class="service-type">{getServiceSourceLabel(suggestion.source)}</div>
                {#if suggestion.description}
                  <div class="service-description">{suggestion.description}</div>
                {/if}
              </div>
            </div>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .text-autocomplete-wrapper {
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

  .autocomplete-suggestions {
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

  .autocomplete-suggestion {
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

  .autocomplete-suggestion:hover,
  .autocomplete-suggestion.selected {
    background-color: #2a2a2a;
  }

  .emoji-icon {
    font-size: 18px;
    flex-shrink: 0;
  }

  .emoji-name {
    font-size: 12px;
    font-family: 'Courier New', monospace;
    color: #888;
  }

  .autocomplete-suggestion.selected .emoji-name {
    color: #4ec9b0;
  }

  .service-suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .service-symbol {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background-color: #353535;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    color: #9cdcfe;
    flex-shrink: 0;
  }

  .service-details {
    display: flex;
    flex-direction: column;
    flex: 1;
  }

  .service-name {
    font-size: 13px;
    color: #fff;
  }

  .service-type {
    font-size: 11px;
    color: #888;
  }

  .service-description {
    font-size: 11px;
    color: #aaaaaa;
    margin-top: 2px;
  }

  .autocomplete-suggestion.selected .service-name {
    color: #4ec9b0;
  }
</style>
