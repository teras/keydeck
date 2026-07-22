<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<!--
  Editor for a page's `when` auto-switch conditions (disjunctive normal form).

  A page activates when ANY group matches; a group matches when ALL its conditions
  match. A condition is `key: value`, where reserved keys window / class / title test
  the focused window (substring) and any other key tests a context variable (set via
  `keydeck --set key=value`). A value may be a comma-separated list, meaning OR.

  In YAML this serializes as a single mapping (one group) or a list of mappings.
-->
<script module lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  // Window-class list for autocomplete, cached across every WhenEditor instance so
  // it is fetched at most once per app session — NOT on every page selection. On
  // Wayland the fetch runs a one-shot KWin script, so refetching per page-open was
  // both wasteful and a needless source of KWin script churn.
  let windowClassCache: string[] | null = null;
  let windowClassInFlight: Promise<string[]> | null = null;

  export function fetchWindowClasses(): Promise<string[]> {
    if (windowClassCache) return Promise.resolve(windowClassCache);
    if (windowClassInFlight) return windowClassInFlight;
    windowClassInFlight = invoke<string[]>("list_window_classes")
      .then((r) => (windowClassCache = r || []))
      .catch(() => (windowClassCache = []))
      .finally(() => (windowClassInFlight = null));
    return windowClassInFlight;
  }
</script>

<script lang="ts">
  import { untrack } from "svelte";

  interface Condition {
    key: string;
    value: string; // comma-separated = OR
  }
  interface Group {
    rows: Condition[];
  }

  interface Props {
    /** Current `when` value from the config (object, array of objects, or undefined). */
    when: any;
    /** Changes when the edited page changes, so the editor resyncs from `when`. */
    resetKey: string;
    /** Called with the new `when` value (undefined clears it). */
    onUpdate: (when: any) => void;
  }

  let { when, resetKey, onUpdate }: Props = $props();

  // Keys that test the focused window (drive value autocomplete + placeholders).
  const RESERVED = ["window", "class", "title"];
  // Keys offered as suggestions. `context` is the conventional context-variable
  // key (any non-reserved key works, but this makes it discoverable).
  const KEY_SUGGESTIONS = [...RESERVED, "context"];

  function whenToGroups(value: any): Group[] {
    if (!value) return [];
    const arr = Array.isArray(value) ? value : [value];
    return arr
      .filter((g) => g && typeof g === "object")
      .map((g) => ({
        rows: Object.entries(g).map(([key, v]) => ({
          key,
          value: Array.isArray(v) ? v.join(", ") : String(v),
        })),
      }));
  }

  function groupsToWhen(groups: Group[]): any {
    const cleaned = groups
      .map((g) => {
        const obj: Record<string, string | string[]> = {};
        for (const row of g.rows) {
          const key = row.key.trim();
          if (!key) continue;
          const parts = row.value
            .split(",")
            .map((s) => s.trim())
            .filter(Boolean);
          if (parts.length === 0) continue;
          obj[key] = parts.length === 1 ? parts[0] : parts;
        }
        return obj;
      })
      .filter((obj) => Object.keys(obj).length > 0);

    if (cleaned.length === 0) return undefined;
    return cleaned.length === 1 ? cleaned[0] : cleaned;
  }

  let groups = $state<Group[]>(whenToGroups(when));
  let lastKey = $state(resetKey);
  let windowClasses = $state<string[]>([]);

  // Resync from the config only when a different page is selected, not on our own edits.
  $effect(() => {
    if (resetKey !== lastKey) {
      lastKey = resetKey;
      groups = whenToGroups(untrack(() => when));
    }
  });

  // Populate the autocomplete list lazily: only when a window-condition value field
  // is first focused (see the input's `onfocus`), reusing the session-wide cache.
  let windowClassesLoaded = false;
  function loadWindowClasses() {
    if (windowClassesLoaded) return;
    windowClassesLoaded = true;
    fetchWindowClasses().then((r) => (windowClasses = r));
  }

  function emit() {
    onUpdate(groupsToWhen(groups));
  }

  function addGroup() {
    groups.push({ rows: [{ key: "window", value: "" }] });
    emit();
  }
  function addRow(gi: number) {
    groups[gi].rows.push({ key: "", value: "" });
    emit();
  }
  function removeRow(gi: number, ri: number) {
    groups[gi].rows.splice(ri, 1);
    if (groups[gi].rows.length === 0) groups.splice(gi, 1);
    emit();
  }

  function isWindowKey(key: string): boolean {
    return RESERVED.includes(key.trim().toLowerCase());
  }
</script>

<div class="when-editor">
  <div class="head">
    <h4>Auto-switch</h4>
    <button type="button" class="add-btn" title="Add condition group" onclick={addGroup}>+</button>
  </div>
  <p class="help">
    Activate this page when the focused window and/or context variables match.
  </p>

  <datalist id="when-keys">
    {#each KEY_SUGGESTIONS as k}
      <option value={k}></option>
    {/each}
  </datalist>
  <datalist id="when-window-values">
    {#each windowClasses as c}
      <option value={c}></option>
    {/each}
  </datalist>

  {#if groups.length === 0}
    <p class="empty">No auto-switch conditions configured</p>
  {:else}
    {#each groups as group, gi (gi)}
      {#if gi > 0}
        <div class="or">or</div>
      {/if}
      <div class="group">
        {#each group.rows as row, ri (ri)}
          <div class="row">
            <input
              class="key"
              list="when-keys"
              placeholder="window"
              bind:value={row.key}
              oninput={emit}
            />
            <input
              class="value"
              list={isWindowKey(row.key) ? "when-window-values" : undefined}
              placeholder={isWindowKey(row.key) ? "kitty, konsole" : "value"}
              bind:value={row.value}
              oninput={emit}
              onfocus={() => { if (isWindowKey(row.key)) loadWindowClasses(); }}
            />
            <button
              type="button"
              class="icon-btn"
              title="Remove condition"
              onclick={() => removeRow(gi, ri)}>✕</button
            >
          </div>
        {/each}
        <button type="button" class="add-row" title="Add condition" onclick={() => addRow(gi)}
          >+</button
        >
      </div>
    {/each}
  {/if}
</div>

<style>
  .when-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  h4 {
    margin: 0;
    font-size: 13px;
    color: #aaa;
  }

  .add-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 15px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .add-btn:hover {
    background-color: #1177bb;
  }

  .help {
    margin: 0;
    font-size: 11px;
    color: #666;
    font-style: italic;
  }

  .empty {
    color: #666;
    font-size: 12px;
    font-style: italic;
    margin: 4px 0;
  }

  .or {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #777;
    text-align: center;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    border: 1px solid #3e3e42;
    border-radius: 4px;
    background-color: #2b2b2b;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .row input {
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
    min-height: 32px;
  }

  .row input:focus {
    outline: none;
    border-color: #0e639c;
  }

  /* Clearly dim placeholders so an unfilled field reads differently from a real value. */
  .row input::placeholder {
    color: #6a6a6a;
    font-style: italic;
  }

  .row .key {
    flex: 0 0 34%;
    min-width: 0;
  }

  .row .value {
    flex: 1;
    min-width: 0;
  }

  .icon-btn {
    flex: 0 0 auto;
    width: 28px;
    height: 32px;
    padding: 0;
    background-color: transparent;
    color: #999;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
  }

  .icon-btn:hover {
    background-color: #4a2626;
    color: #ff9c9c;
    border-color: #6a3a3a;
  }

  .add-row {
    align-self: flex-start;
    width: 24px;
    height: 24px;
    padding: 0;
    background-color: transparent;
    color: #7fb4d6;
    border: 1px solid #3e3e42;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .add-row:hover {
    background-color: #333;
    color: #9cd;
  }
</style>
