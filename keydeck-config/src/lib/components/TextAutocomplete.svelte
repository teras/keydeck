<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import { emojiList } from '$lib/data/emojis';

  type ServiceSource = 'embedded' | 'user' | 'both';

  interface ProviderEntry {
    name: string;
    description?: string;
    source?: ServiceSource;
  }

  interface EmojiSuggestion {
    kind: 'emoji';
    emoji: string;
    name: string;
  }

  interface ProviderSuggestionItem {
    kind: 'provider';
    provider: string;
    name: string;
    description?: string;
    source?: ServiceSource;
  }

  interface ProviderTypeSuggestion {
    kind: 'provider-type';
    provider: string;
    displayName: string;
  }

  interface InfoSuggestion {
    kind: 'info';
    message: string;
  }

  type AutocompleteSuggestion = EmojiSuggestion | ProviderSuggestionItem | ProviderTypeSuggestion | InfoSuggestion;

  interface ProviderSuggestionGroup {
    provider: string;
    displayName?: string;
    entries: ProviderEntry[];
    insertTemplate?: (name: string) => string;
  }

  interface Props {
    value: string;
    onUpdate: (value: string) => void;
    disabled?: boolean;
    placeholder?: string;
    multiline?: boolean;
    rows?: number;
    serviceSuggestions?: ProviderEntry[];
    providerSuggestionMap?: Record<string, ProviderSuggestionGroup>;
  }

  let {
    value,
    onUpdate,
    disabled = false,
    placeholder = '',
    multiline = false,
    rows = 3,
    serviceSuggestions = [],
    providerSuggestionMap = {}
  }: Props = $props();

  let inputElement = $state<HTMLInputElement | HTMLTextAreaElement | undefined>();
  let showSuggestions = $state(false);
  let suggestions = $state<AutocompleteSuggestion[]>([]);
  let selectedIndex = $state(0);
  let searchStart = $state(0);
  let providerTriggerMode = $state<'curly' | 'dollar' | null>(null);
  let activeProvider = $state<string | null>(null);
  const MAX_RESULTS = 50;

  function getProviderEntries(provider: string): ProviderEntry[] {
    if (providerSuggestionMap[provider]) {
      const entries = providerSuggestionMap[provider].entries;
      if (entries.length > 0) {
        return entries;
      }
    }
    if (provider === 'service') {
      return serviceSuggestions;
    }
    return [];
  }

  function applyLimit<T>(items: T[]): { list: T[]; truncated: boolean } {
    if (items.length <= MAX_RESULTS) {
      return { list: items, truncated: false };
    }
    return { list: items.slice(0, MAX_RESULTS), truncated: true };
  }

  function buildProviderEntrySuggestions(provider: string, searchText: string): AutocompleteSuggestion[] {
    const entries = getProviderEntries(provider);
    if (!entries.length) return [];

    const normalized = searchText.toLowerCase();
    const filtered = entries.filter(entry => entry.name.toLowerCase().includes(normalized));
    const limited = applyLimit(filtered);
    const items: AutocompleteSuggestion[] = limited.list.map(entry => ({
      kind: 'provider',
      provider,
      name: entry.name,
      description: entry.description,
      source: entry.source,
    }));

    if (limited.truncated) {
      items.push({
        kind: 'info',
        message: 'More matches available… keep typing',
      });
    }

    return items;
  }

  function getAvailableProviders(): string[] {
    const keys = Object.keys(providerSuggestionMap);
    if (!keys.includes('service')) {
      if (getProviderEntries('service').length > 0 || Object.keys(providerSuggestionMap).length === 0) {
        keys.push('service');
      }
    }
    return keys;
  }

  function buildProviderTypeSuggestions(searchText: string): AutocompleteSuggestion[] {
    const keys = getAvailableProviders();
    if (keys.length === 0) return [];
    const normalized = searchText.toLowerCase();
    const filtered = keys.filter(key => key.toLowerCase().includes(normalized));
    const limited = applyLimit(filtered);
    const list: AutocompleteSuggestion[] = limited.list.map(key => ({
      kind: 'provider-type',
      provider: key,
      displayName: getProviderLabel(key),
    }));

    if (limited.truncated) {
      list.push({
        kind: 'info',
        message: 'More providers available… keep typing',
      });
    }

    return list;
  }

  function activateProviderEntries(
    provider: string,
    searchText: string,
    triggerMode: 'curly' | 'dollar',
    startPosition: number
  ): boolean {
    const list = buildProviderEntrySuggestions(provider, searchText);
    if (list.length === 0) return false;

    searchStart = startPosition;
    providerTriggerMode = triggerMode;
    activeProvider = provider;
    suggestions = list;
    showSuggestions = true;
    selectedIndex = 0;
    return true;
  }

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement | HTMLTextAreaElement;
    const newValue = target.value;
    const cursorPos = target.selectionStart || 0;

    onUpdate(newValue);

    const textBeforeCursor = newValue.slice(0, cursorPos);
    const providerKeys = getAvailableProviders();

    // Continue suggestions when inside an inserted provider template
    if (providerTriggerMode === 'curly' && activeProvider && cursorPos >= searchStart) {
      const searchText = newValue.slice(searchStart, cursorPos);
      if (activateProviderEntries(activeProvider, searchText, 'curly', searchStart)) {
        return;
      }
    }

    // Manual ${provider:...} typing
    if (providerKeys.length > 0) {
      for (const provider of providerKeys) {
        const prefix = `${'${'}${provider}:`;
        const lastIndex = textBeforeCursor.lastIndexOf(prefix);
        if (lastIndex !== -1) {
          const start = lastIndex + prefix.length;
          const searchText = textBeforeCursor.slice(start);
          if (!/\s/.test(searchText) && !searchText.includes('}')) {
            if (activateProviderEntries(provider, searchText, 'curly', start)) {
              return;
            }
          }
        }
      }
    }

    // Dollar-triggered providers ($provider or $provider:value)
    const lastDollarIndex = textBeforeCursor.lastIndexOf('$');
    if (providerKeys.length > 0 && lastDollarIndex !== -1) {
      const providerExpr = textBeforeCursor.slice(lastDollarIndex + 1);
      const colonIndex = providerExpr.indexOf(':');

      if (colonIndex >= 0) {
        const provider = providerExpr.slice(0, colonIndex);
        const searchText = providerExpr.slice(colonIndex + 1);
        if (provider && activateProviderEntries(provider, searchText, 'dollar', lastDollarIndex)) {
          return;
        }
      } else {
      const providerSuggestions = buildProviderTypeSuggestions(providerExpr).slice(0, MAX_RESULTS);
        if (providerSuggestions.length > 0) {
          searchStart = lastDollarIndex;
          providerTriggerMode = null;
          activeProvider = null;
          suggestions = providerSuggestions;
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
          .map(item => ({
            kind: 'emoji',
            emoji: item.emoji,
            name: item.name,
          } as EmojiSuggestion));

        if (filtered.length > 0) {
          const { list, truncated } = applyLimit(filtered);
          suggestions = truncated
            ? [...list, { kind: 'info', message: 'More emoji available… keep typing' }]
            : list;
          showSuggestions = true;
          selectedIndex = 0;
          return;
        }
      }
    }

    showSuggestions = false;
    suggestions = [];
    providerTriggerMode = null;
    activeProvider = null;
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
    providerTriggerMode = null;

    // Restore focus and cursor position
    setTimeout(() => {
      if (inputElement) {
        inputElement.focus();
        inputElement.setSelectionRange(newCursorPos, newCursorPos);
      }
    }, 0);
  }

  function insertProviderValue(provider: string, name: string) {
    if (!inputElement) return;

    const cursorPos = inputElement.selectionStart || 0;
    const textBefore = value.slice(0, searchStart);
    const textAfter = value.slice(cursorPos);
    const providerKey = provider || activeProvider || 'service';
    const group = providerSuggestionMap[providerKey];

    let insertion: string;
    if (providerTriggerMode === 'curly') {
      insertion = `${name}}`;
    } else if (group?.insertTemplate) {
      insertion = group.insertTemplate(name);
    } else {
      insertion = `\${${providerKey}:${name}}`;
    }

    const newValue = textBefore + insertion + textAfter;
    const newCursorPos = textBefore.length + insertion.length;

    onUpdate(newValue);
    showSuggestions = false;
    providerTriggerMode = null;
    activeProvider = null;

    setTimeout(() => {
      if (inputElement) {
        inputElement.focus();
        inputElement.setSelectionRange(newCursorPos, newCursorPos);
      }
    }, 0);
  }

  function insertProviderTemplate(provider: string) {
    if (!inputElement) return;

    const cursorPos = inputElement.selectionStart || 0;
    const textBefore = value.slice(0, searchStart);
    const textAfter = value.slice(cursorPos);
    const insertion = `\${${provider}:`;
    const newValue = textBefore + insertion + textAfter;
    const newCursorPos = textBefore.length + insertion.length;

    onUpdate(newValue);
    showSuggestions = false;
    providerTriggerMode = 'curly';
    activeProvider = provider;
    searchStart = newCursorPos;

    setTimeout(() => {
      if (inputElement) {
        inputElement.focus();
        inputElement.setSelectionRange(newCursorPos, newCursorPos);
      }
      activateProviderEntries(provider, '', 'curly', searchStart);
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
      } else if (selected.kind === 'provider') {
        insertProviderValue(selected.provider, selected.name);
      } else if (selected.kind === 'provider-type') {
        insertProviderTemplate(selected.provider);
      } else {
        return;
      }
    } else if (event.key === 'Escape') {
      event.preventDefault();
      showSuggestions = false;
      providerTriggerMode = null;
      activeProvider = null;
    }
  }

  function handleBlur(event: FocusEvent) {
    // Delay to allow clicking on suggestions
    setTimeout(() => {
      // Check if the new focus target is within the suggestions
      const relatedTarget = event.relatedTarget as HTMLElement;
      if (!relatedTarget || !relatedTarget.closest('.autocomplete-suggestions')) {
        showSuggestions = false;
        providerTriggerMode = null;
        activeProvider = null;
      }
    }, 200);
  }

  function getSourceLabel(source?: ServiceSource): string {
    switch (source) {
      case 'embedded':
        return 'Embedded';
      case 'both':
        return 'Embedded + Custom';
      case 'user':
        return 'Custom';
      default:
        return '';
    }
  }

  function getProviderLabel(provider: string): string {
    if (providerSuggestionMap[provider]?.displayName) {
      return providerSuggestionMap[provider].displayName as string;
    }
    if (provider === 'service') {
      return 'Service';
    }
    return provider.charAt(0).toUpperCase() + provider.slice(1);
  }

  function getProviderSymbol(provider: string): string {
    switch (provider) {
      case 'service':
        return '$';
      case 'env':
        return 'ENV';
      case 'time':
        return '⏱';
      default:
        return provider.slice(0, 3).toUpperCase();
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
            } else if (suggestion.kind === 'provider') {
              insertProviderValue(suggestion.provider, suggestion.name);
            } else if (suggestion.kind === 'provider-type') {
              insertProviderTemplate(suggestion.provider);
            }
          }}
        >
          {#if suggestion.kind === 'emoji'}
            <span class="emoji-icon">{suggestion.emoji}</span>
            <span class="emoji-name">:{suggestion.name}:</span>
          {:else if suggestion.kind === 'provider'}
            <div class="provider-suggestion">
              <div class="provider-symbol">{getProviderSymbol(suggestion.provider)}</div>
              <div class="provider-details">
                <div class="provider-name">{suggestion.name}</div>
                <div class="provider-meta">
                  <span class="provider-type">{getProviderLabel(suggestion.provider)}</span>
                  {#if getSourceLabel(suggestion.source)}
                    <span class="provider-source">{getSourceLabel(suggestion.source)}</span>
                  {/if}
                </div>
                {#if suggestion.description}
                  <div class="provider-description">{suggestion.description}</div>
                {/if}
              </div>
            </div>
          {:else if suggestion.kind === 'provider-type'}
            <div class="provider-type-suggestion">
              <div class="provider-symbol">{getProviderSymbol(suggestion.provider)}</div>
              <div class="provider-details">
                <div class="provider-name">{suggestion.displayName}</div>
                <div class="provider-meta">
                  <span class="provider-type">Select provider</span>
                </div>
              </div>
            </div>
          {:else}
            <div class="info-suggestion">{suggestion.message}</div>
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

  .provider-suggestion,
  .provider-type-suggestion {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .provider-symbol {
    min-width: 28px;
    height: 24px;
    border-radius: 12px;
    background-color: #353535;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    color: #9cdcfe;
    flex-shrink: 0;
    padding: 0 8px;
    font-size: 11px;
  }

  .provider-details {
    display: flex;
    flex-direction: column;
    flex: 1;
  }

  .provider-name {
    font-size: 13px;
    color: #fff;
  }

  .provider-meta {
    font-size: 11px;
    color: #888;
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .provider-type {
    font-weight: 600;
    color: #9cdcfe;
  }

  .provider-source {
    font-size: 10px;
    color: #aaaaaa;
  }

  .provider-description {
    font-size: 11px;
    color: #aaaaaa;
    margin-top: 2px;
  }

  .info-suggestion {
    font-size: 11px;
    color: #bbbbbb;
    padding: 4px 8px;
    font-style: italic;
  }

  .autocomplete-suggestion.selected .provider-name {
    color: #4ec9b0;
  }
</style>
