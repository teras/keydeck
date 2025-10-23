<script lang="ts">
  import ActionEditor from './ActionEditor.svelte';

  interface Props {
    action: any;
    onUpdate: (action: any) => void;
    onDelete: () => void;
    index: number;
    depth?: number;
    config?: any;
    deviceSerial?: string;
    initiallyOpen?: boolean;
    onToggle?: () => void;
  }

  let { action, onUpdate, onDelete, index, depth = 0, config, deviceSerial, initiallyOpen = false, onToggle }: Props = $props();

  // Get list of available pages
  let availablePages = $derived.by(() => {
    if (!config || !deviceSerial) return [];
    const pageGroup = config.page_groups?.[deviceSerial] || config.page_groups?.default;
    if (!pageGroup) return [];
    const knownFields = ['main_page', 'restore_mode', 'on_tick'];
    return Object.keys(pageGroup).filter(key => !knownFields.includes(key));
  });

  // Get list of available macros
  let availableMacros = $derived(config?.macros ? Object.keys(config.macros) : []);

  // Get macro parameters for the selected macro
  function getMacroParams(macroName: string): string[] {
    if (!config?.macros?.[macroName]) return [];
    const macro = config.macros[macroName];
    return macro.params ? Object.keys(macro.params) : [];
  }

  // Determine action type from the action object
  function getActionType(action: any): string {
    if (action.refresh !== undefined) return 'refresh';
    if (action.exec !== undefined) return 'exec';
    if (action.jump !== undefined) return 'jump';
    if (action.auto_jump !== undefined) return 'auto_jump';
    if (action.focus !== undefined) return 'focus';
    if (action.wait_for !== undefined) return 'wait_for';
    if (action.key !== undefined) return 'key';
    if (action.text !== undefined) return 'text';
    if (action.wait !== undefined) return 'wait';
    if (action.try !== undefined) return 'try';
    if (action.macro !== undefined) return 'macro';
    if (action.return !== undefined) return 'return';
    if (action.fail !== undefined) return 'fail';
    if (action.and !== undefined) return 'and';
    if (action.or !== undefined) return 'or';
    if (action.not !== undefined) return 'not';
    return 'exec'; // default fallback
  }

  let actionType = $derived(getActionType(action));

  let isExpanded = $state(initiallyOpen);

  // Update isExpanded when initiallyOpen changes
  $effect(() => {
    isExpanded = initiallyOpen;
  });

  // Update action when type changes
  function changeActionType(newType: string) {
    switch (newType) {
      case 'exec':
        onUpdate({ exec: '', wait: false });
        break;
      case 'jump':
        onUpdate({ jump: '' });
        break;
      case 'auto_jump':
        onUpdate({ auto_jump: null });
        break;
      case 'focus':
        onUpdate({ focus: '' });
        break;
      case 'wait_for':
        onUpdate({ wait_for: '', timeout: 1.0 });
        break;
      case 'key':
        onUpdate({ key: '' });
        break;
      case 'text':
        onUpdate({ text: '' });
        break;
      case 'wait':
        onUpdate({ wait: 0.5 });
        break;
      case 'try':
        onUpdate({ try: [], else: [] });
        break;
      case 'macro':
        onUpdate({ macro: '' });
        break;
      case 'return':
        onUpdate({ return: null });
        break;
      case 'fail':
        onUpdate({ fail: null });
        break;
      case 'and':
        onUpdate({ and: [] });
        break;
      case 'or':
        onUpdate({ or: [] });
        break;
      case 'not':
        onUpdate({ not: { exec: '' } });
        break;
      case 'refresh':
        onUpdate({ refresh: 'dynamic' });
        break;
    }
  }

  let actionSummary = $derived.by(() => {
    switch (actionType) {
      case 'exec':
        return `Execute: ${action.exec || '(empty)'}`;
      case 'jump':
        return `Jump to: ${action.jump || '(empty)'}`;
      case 'auto_jump':
        return 'Auto Jump (return to previous)';
      case 'focus':
        return `Focus: ${action.focus || '(empty)'}`;
      case 'wait_for':
        return `Wait for: ${action.wait_for || '(empty)'}`;
      case 'key':
        return `Send Key: ${action.key || '(empty)'}`;
      case 'text':
        return `Send Text: ${action.text || '(empty)'}`;
      case 'wait':
        return `Wait ${action.wait || 0}s`;
      case 'try':
        return `Try/Else (${action.try?.length || 0}/${action.else?.length || 0} actions)`;
      case 'macro':
        return `Macro: ${action.macro || '(empty)'}`;
      case 'return':
        return 'Return (stop execution)';
      case 'fail':
        return 'Fail (trigger error)';
      case 'and':
        return `And (${action.and?.length || 0} actions)`;
      case 'or':
        return `Or (${action.or?.length || 0} actions)`;
      case 'not':
        return 'Not (invert result)';
      case 'refresh':
        if (action.refresh === 'dynamic') {
          return 'Refresh all dynamic buttons';
        } else if (Array.isArray(action.refresh)) {
          return `Refresh buttons: ${action.refresh.join(', ')}`;
        } else {
          return `Refresh button: ${action.refresh}`;
        }
      default:
        return 'Unknown action';
    }
  });
</script>

<div class="action-editor" style="--depth: {depth}">
  <div class="action-header" onclick={() => {
    isExpanded = !isExpanded;
    if (onToggle) onToggle();
  }}>
    <span class="action-index">#{index + 1}</span>
    <span class="action-summary">{actionSummary}</span>
    <div class="action-controls">
      <button class="btn-delete" onclick={(e) => { e.stopPropagation(); onDelete(); }} title="Delete">×</button>
      <span class="expand-icon">{isExpanded ? '▼' : '▶'}</span>
    </div>
  </div>

  {#if isExpanded}
    <div class="action-body">
      <div class="form-row">
        <label>Action Type</label>
        <select bind:value={actionType} onchange={() => changeActionType(actionType)}>
          <option value="jump">Jump to Page</option>
          <option value="auto_jump">Auto Jump</option>
          <option value="focus">Focus Window</option>
          <option value="key">Send Keyboard Key</option>
          <option value="text">Send Text</option>
          <option value="wait">Wait (delay)</option>
          <option value="wait_for">Wait For Event</option>
          <option value="exec">Execute Command</option>
          <option value="macro">Call Macro</option>
          <option value="try">Try/Else</option>
          <option value="return">Return</option>
          <option value="fail">Fail</option>
          <option value="and">And (all must succeed)</option>
          <option value="or">Or (any succeeds)</option>
          <option value="not">Not (invert)</option>
          <option value="refresh">Refresh Buttons</option>
        </select>
      </div>

      {#if actionType === 'exec'}
        <div class="form-row">
          <label>Command</label>
          <input
            type="text"
            value={action.exec || ''}
            oninput={(e) => onUpdate({ ...action, exec: e.currentTarget.value })}
            placeholder="bash command to execute"
          />
        </div>
        <div class="form-row checkbox">
          <label>
            <input
              type="checkbox"
              checked={action.wait || false}
              onchange={(e) => onUpdate({ ...action, wait: e.currentTarget.checked })}
            />
            Wait for completion
          </label>
        </div>

      {:else if actionType === 'jump'}
        <div class="form-row">
          <label>Page Name</label>
          {#if availablePages.length > 0}
            <select
              value={action.jump || ''}
              onchange={(e) => onUpdate({ ...action, jump: e.currentTarget.value })}
            >
              <option value="">Select a page</option>
              {#each availablePages as page}
                <option value={page}>{page}</option>
              {/each}
            </select>
          {:else}
            <input
              type="text"
              value={action.jump || ''}
              oninput={(e) => onUpdate({ ...action, jump: e.currentTarget.value })}
              placeholder="page name"
            />
          {/if}
        </div>

      {:else if actionType === 'focus'}
        <div class="form-row">
          <label>Window Class</label>
          <input
            type="text"
            value={action.focus || ''}
            oninput={(e) => onUpdate({ ...action, focus: e.currentTarget.value })}
            placeholder="window class name"
          />
        </div>

      {:else if actionType === 'wait_for'}
        <div class="form-row">
          <label>Event Type</label>
          <select
            value={action.wait_for || ''}
            onchange={(e) => onUpdate({ ...action, wait_for: e.currentTarget.value })}
          >
            <option value="">Select event type</option>
            <option value="focus">Focus (window focus changed)</option>
            <option value="page">Page (page changed)</option>
            <option value="tick">Tick (timer tick)</option>
            <option value="sleep">Sleep (system sleep/wake)</option>
            <option value="newdevice">New Device (device connected)</option>
            <option value="removeddevice">Removed Device (device disconnected)</option>
            <option value="timer">Timer (timer completed)</option>
          </select>
        </div>
        <div class="form-row">
          <label>Timeout (seconds)</label>
          <input
            type="number"
            value={action.timeout !== undefined ? action.timeout : ''}
            oninput={(e) => {
              const val = e.currentTarget.value;
              if (val === '' || parseFloat(val) === 1.0) {
                // Remove timeout if empty or default value
                const { timeout, ...rest } = action;
                onUpdate(rest);
              } else {
                onUpdate({ ...action, timeout: parseFloat(val) });
              }
            }}
            placeholder="1.0 (default)"
            step="0.1"
            min="0.1"
          />
        </div>

      {:else if actionType === 'key'}
        <div class="form-row">
          <label>Keyboard Shortcut</label>
          <input
            type="text"
            value={action.key || ''}
            oninput={(e) => onUpdate({ ...action, key: e.currentTarget.value })}
            placeholder="e.g., LCtrl+LShift+z or F12"
          />
        </div>

      {:else if actionType === 'text'}
        <div class="form-row">
          <label>Text to Type</label>
          <input
            type="text"
            value={action.text || ''}
            oninput={(e) => onUpdate({ ...action, text: e.currentTarget.value })}
            placeholder="text to send as keystrokes"
          />
        </div>

      {:else if actionType === 'wait'}
        <div class="form-row">
          <label>Wait Time (seconds)</label>
          <input
            type="number"
            value={action.wait || 0.5}
            oninput={(e) => onUpdate({ ...action, wait: parseFloat(e.currentTarget.value) })}
            step="0.1"
            min="0"
          />
        </div>

      {:else if actionType === 'macro'}
        <div class="form-row">
          <label>Macro Name</label>
          {#if availableMacros.length > 0}
            <select
              value={typeof action.macro === 'string' ? action.macro : (action.macro?.name || '')}
              onchange={(e) => {
                const macroName = e.currentTarget.value;
                const params = getMacroParams(macroName);
                if (params.length > 0) {
                  // Initialize with default params from macro definition
                  const macro = config.macros[macroName];
                  onUpdate({ macro: { name: macroName, params: { ...macro.params } } });
                } else {
                  onUpdate({ macro: macroName });
                }
              }}
            >
              <option value="">Select a macro</option>
              {#each availableMacros as macroName}
                <option value={macroName}>{macroName}</option>
              {/each}
            </select>
          {:else}
            <input
              type="text"
              value={typeof action.macro === 'string' ? action.macro : (action.macro?.name || '')}
              oninput={(e) => onUpdate({ ...action, macro: e.currentTarget.value })}
              placeholder="macro name"
            />
          {/if}
        </div>

        {#if action.macro}
          {@const macroName = typeof action.macro === 'string' ? action.macro : action.macro?.name}
          {@const macroParams = getMacroParams(macroName)}
          {#if macroParams.length > 0}
            <div class="param-section">
              <label class="param-section-label">Parameters</label>
              <div class="param-list">
                {#each macroParams as paramName}
                  <div class="param-item">
                    <div class="param-info">
                      <span class="param-name">{paramName}</span>
                      <input
                        type="text"
                        class="param-value"
                        value={action.macro?.params?.[paramName] || ''}
                        oninput={(e) => {
                          const currentMacro = typeof action.macro === 'string'
                            ? { name: action.macro, params: {} }
                            : action.macro;
                          onUpdate({
                            macro: {
                              ...currentMacro,
                              params: {
                                ...(currentMacro.params || {}),
                                [paramName]: e.currentTarget.value
                              }
                            }
                          });
                        }}
                        placeholder="Enter value"
                      />
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        {/if}

      {:else if actionType === 'refresh'}
        <div class="form-row">
          <label>Button(s) to Refresh</label>
          <input
            type="text"
            value={action.refresh || ''}
            oninput={(e) => {
              const val = e.currentTarget.value.trim();
              if (val === '') {
                onUpdate({ refresh: null });
              } else if (val.includes(',')) {
                onUpdate({ refresh: val.split(',').map(s => parseInt(s.trim())).filter(n => !isNaN(n)) });
              } else {
                const num = parseInt(val);
                onUpdate({ refresh: isNaN(num) ? null : num });
              }
            }}
            placeholder="leave empty for all dynamic, or button number(s), e.g., 1 or 1,2,3"
          />
        </div>

      {:else if actionType === 'try'}
        <div class="nested-actions">
          <div class="nested-section">
            <label class="nested-label">Try Actions:</label>
            <div class="nested-list">
              {#if action.try && action.try.length > 0}
                {#each action.try as tryAction, i}
                  <ActionEditor
                    action={tryAction}
                    index={i}
                    depth={depth + 1}
                    {config}
                    {deviceSerial}
                    onUpdate={(newAction) => {
                      action.try[i] = newAction;
                      onUpdate(action);
                    }}
                    onDelete={() => {
                      action.try.splice(i, 1);
                      onUpdate(action);
                    }}
                  />
                {/each}
              {:else}
                <p class="empty-nested">No try actions</p>
              {/if}
            </div>
            <button class="btn-add-nested" onclick={() => {
              if (!action.try) action.try = [];
              action.try.push({ exec: '' });
              onUpdate(action);
            }}>+ Add Try Action</button>
          </div>

          <div class="nested-section">
            <label class="nested-label">Else Actions:</label>
            <div class="nested-list">
              {#if action.else && action.else.length > 0}
                {#each action.else as elseAction, i}
                  <ActionEditor
                    action={elseAction}
                    index={i}
                    depth={depth + 1}
                    {config}
                    {deviceSerial}
                    onUpdate={(newAction) => {
                      action.else[i] = newAction;
                      onUpdate(action);
                    }}
                    onDelete={() => {
                      action.else.splice(i, 1);
                      onUpdate(action);
                    }}
                  />
                {/each}
              {:else}
                <p class="empty-nested">No else actions</p>
              {/if}
            </div>
            <button class="btn-add-nested" onclick={() => {
              if (!action.else) action.else = [];
              action.else.push({ exec: '' });
              onUpdate(action);
            }}>+ Add Else Action</button>
          </div>
        </div>

      {:else if actionType === 'and'}
        <div class="nested-actions">
          <div class="nested-section">
            <label class="nested-label">And Actions (all must succeed):</label>
            <div class="nested-list">
              {#if action.and && action.and.length > 0}
                {#each action.and as andAction, i}
                  <ActionEditor
                    action={andAction}
                    index={i}
                    depth={depth + 1}
                    {config}
                    {deviceSerial}
                    onUpdate={(newAction) => {
                      action.and[i] = newAction;
                      onUpdate(action);
                    }}
                    onDelete={() => {
                      action.and.splice(i, 1);
                      onUpdate(action);
                    }}
                  />
                {/each}
              {:else}
                <p class="empty-nested">No actions</p>
              {/if}
            </div>
            <button class="btn-add-nested" onclick={() => {
              if (!action.and) action.and = [];
              action.and.push({ exec: '' });
              onUpdate(action);
            }}>+ Add Action</button>
          </div>
        </div>

      {:else if actionType === 'or'}
        <div class="nested-actions">
          <div class="nested-section">
            <label class="nested-label">Or Actions (any must succeed):</label>
            <div class="nested-list">
              {#if action.or && action.or.length > 0}
                {#each action.or as orAction, i}
                  <ActionEditor
                    action={orAction}
                    index={i}
                    depth={depth + 1}
                    {config}
                    {deviceSerial}
                    onUpdate={(newAction) => {
                      action.or[i] = newAction;
                      onUpdate(action);
                    }}
                    onDelete={() => {
                      action.or.splice(i, 1);
                      onUpdate(action);
                    }}
                  />
                {/each}
              {:else}
                <p class="empty-nested">No actions</p>
              {/if}
            </div>
            <button class="btn-add-nested" onclick={() => {
              if (!action.or) action.or = [];
              action.or.push({ exec: '' });
              onUpdate(action);
            }}>+ Add Action</button>
          </div>
        </div>

      {:else if actionType === 'not'}
        <div class="nested-actions">
          <div class="nested-section">
            <label class="nested-label">Not Action (invert result):</label>
            <div class="nested-list">
              {#if action.not}
                <ActionEditor
                  action={action.not}
                  index={0}
                  depth={depth + 1}
                  {config}
                  {deviceSerial}
                  onUpdate={(newAction) => {
                    action.not = newAction;
                    onUpdate(action);
                  }}
                  onDelete={() => {
                    action.not = { exec: '' };
                    onUpdate(action);
                  }}
                />
              {/if}
            </div>
          </div>
        </div>

      {/if}
    </div>
  {/if}
</div>

<style>
  .action-editor {
    background-color: #3c3c3c;
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 6px;
    margin-left: 0;
  }

  .action-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    cursor: pointer;
    user-select: none;
  }

  .action-header:hover {
    background-color: #444;
  }

  .action-index {
    font-size: 11px;
    color: #888;
    font-weight: 600;
    min-width: 24px;
  }

  .action-summary {
    flex: 1;
    font-size: 12px;
    color: #cccccc;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .action-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-delete {
    background: none;
    border: none;
    color: #f48771;
    cursor: pointer;
    font-size: 18px;
    padding: 0 4px;
    line-height: 1;
  }

  .btn-delete:hover {
    color: #ff6b6b;
  }

  .expand-icon {
    font-size: 10px;
    color: #888;
  }

  .action-body {
    padding: 12px;
    border-top: 1px solid #555;
    background-color: #333;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }

  .form-row.checkbox {
    flex-direction: row;
    align-items: center;
  }

  .form-row.checkbox label {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    text-transform: none;
  }

  .form-row.checkbox input[type="checkbox"] {
    width: auto;
    margin: 0;
  }

  .param-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 12px;
  }

  .param-section-label {
    font-size: 11px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
  }

  .param-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .param-item {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    padding: 8px;
    background-color: #3c3c3c;
    border: 1px solid #555;
    border-radius: 4px;
    gap: 8px;
  }

  .param-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .param-name {
    font-size: 12px;
    font-weight: 600;
    color: #cccccc;
  }

  .param-value {
    padding: 6px 8px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 3px;
    font-size: 12px;
  }

  .param-value:focus {
    outline: none;
    border-color: #0e639c;
  }

  label {
    font-size: 11px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
    display: flex;
    align-items: center;
  }

  input, select {
    padding: 6px 8px;
    background-color: #2a2a2a;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 3px;
    font-size: 12px;
  }

  input:focus, select:focus {
    outline: none;
    border-color: #0e639c;
  }

  .nested-actions {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-left: -11px;
    margin-right: -12px;
  }

  .nested-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0;
    margin-left: 3px;
    border-left: 2px solid #4ec9b0;
  }

  .nested-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-left: 0;
    margin-left: 3px;
  }

  .nested-label {
    font-size: 12px;
    font-weight: 600;
    color: #4ec9b0;
    text-transform: none;
    margin: 0;
    padding-left: 6px;
  }

  .empty-nested {
    color: #666;
    font-size: 11px;
    font-style: italic;
    margin: 4px 0 4px 6px;
  }

  .btn-add-nested {
    padding: 6px 8px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 11px;
    align-self: flex-start;
    margin-left: 6px;
  }

  .btn-add-nested:hover {
    background-color: #1177bb;
  }
</style>
