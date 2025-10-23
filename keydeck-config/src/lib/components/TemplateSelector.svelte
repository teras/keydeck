<script lang="ts">
  interface Props {
    config: any;
    selectedTemplates: string[];
    currentTemplateName?: string; // To exclude current template from the list
    onUpdate: (templates: string[]) => void;
  }

  let { config, selectedTemplates, currentTemplateName, onUpdate }: Props = $props();

  let showDropdown = $state(false);

  // Check if adding a template would create a cycle
  function wouldCreateCycle(templateToAdd: string): boolean {
    if (!currentTemplateName) return false;

    // Build a set of all templates that templateToAdd depends on (directly or indirectly)
    const visited = new Set<string>();
    const toVisit = [templateToAdd];

    while (toVisit.length > 0) {
      const current = toVisit.pop()!;

      // If we've reached the current template, it would create a cycle
      if (current === currentTemplateName) {
        return true;
      }

      // Skip if already visited
      if (visited.has(current)) {
        continue;
      }
      visited.add(current);

      // Add all templates that current inherits from
      const template = config.templates?.[current];
      if (template?.inherits) {
        const inherits = Array.isArray(template.inherits)
          ? template.inherits
          : [template.inherits];

        for (const inherited of inherits) {
          if (!visited.has(inherited)) {
            toVisit.push(inherited);
          }
        }
      }
    }

    return false;
  }

  let availableTemplates = $derived(
    Object.keys(config.templates || {})
      .filter(t => t !== currentTemplateName && !wouldCreateCycle(t))
  );

  // Close dropdown when clicking outside
  $effect(() => {
    if (showDropdown) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.template-selector-container')) {
          showDropdown = false;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  function toggleTemplate(templateName: string) {
    const index = selectedTemplates.indexOf(templateName);
    let newSelection: string[];

    if (index > -1) {
      // Remove template
      newSelection = selectedTemplates.filter(t => t !== templateName);
    } else {
      // Add template
      newSelection = [...selectedTemplates, templateName];
    }

    onUpdate(newSelection);
  }

  function removeTemplate(templateName: string, event: Event) {
    event.stopPropagation();
    const newSelection = selectedTemplates.filter(t => t !== templateName);
    onUpdate(newSelection);
  }

  function isSelected(templateName: string): boolean {
    return selectedTemplates.includes(templateName);
  }
</script>

<div class="template-selector-container">
  <button
    class="selector-trigger"
    onclick={() => showDropdown = !showDropdown}
  >
    <div class="selected-templates">
      {#if selectedTemplates.length > 0}
        <div class="template-chips">
          {#each selectedTemplates as template}
            <span class="template-chip">
              {template}
              <button
                class="remove-chip"
                onclick={(e) => removeTemplate(template, e)}
                title="Remove"
              >
                ✕
              </button>
            </span>
          {/each}
        </div>
      {:else}
        <span class="placeholder">Select templates...</span>
      {/if}
    </div>
    <span class="dropdown-arrow">▼</span>
  </button>

  {#if showDropdown}
    <div class="template-dropdown">
      {#if availableTemplates.length > 0}
        <div class="template-options">
          {#each availableTemplates as template}
            <button
              class="template-option"
              class:selected={isSelected(template)}
              onclick={() => toggleTemplate(template)}
            >
              <span class="checkbox">
                {#if isSelected(template)}
                  ✓
                {/if}
              </span>
              <span class="template-name">{template}</span>
            </button>
          {/each}
        </div>
      {:else}
        <div class="no-templates">
          <p>No templates available</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .template-selector-container {
    position: relative;
    width: 100%;
  }

  .selector-trigger {
    width: 100%;
    min-height: 38px;
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    justify-content: space-between;
    align-items: center;
    text-align: left;
    gap: 8px;
  }

  .selector-trigger:hover {
    background-color: #4a4a4a;
  }

  .selector-trigger:focus {
    outline: none;
    border-color: #0e639c;
  }

  .selected-templates {
    flex: 1;
    min-width: 0;
  }

  .template-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .template-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    background-color: #0e639c;
    color: white;
    border-radius: 3px;
    font-size: 12px;
    line-height: 1.4;
  }

  .remove-chip {
    background: none;
    border: none;
    color: white;
    cursor: pointer;
    padding: 0;
    margin: 0;
    font-size: 14px;
    line-height: 1;
    width: 14px;
    height: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
  }

  .remove-chip:hover {
    background-color: rgba(255, 255, 255, 0.2);
  }

  .placeholder {
    color: #888;
    font-style: italic;
    font-size: 13px;
  }

  .dropdown-arrow {
    font-size: 10px;
    color: #888;
    flex-shrink: 0;
  }

  .template-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    background-color: #2d2d30;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 1000;
    max-height: 300px;
    display: flex;
    flex-direction: column;
  }

  .template-options {
    overflow-y: auto;
    max-height: 300px;
  }

  .template-option {
    width: 100%;
    padding: 10px 12px;
    background: none;
    border: none;
    color: #cccccc;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    font-size: 13px;
  }

  .template-option:hover {
    background-color: #3c3c3c;
  }

  .template-option.selected {
    background-color: rgba(14, 99, 156, 0.2);
  }

  .template-option.selected:hover {
    background-color: rgba(14, 99, 156, 0.3);
  }

  .checkbox {
    width: 18px;
    height: 18px;
    border: 2px solid #555;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    flex-shrink: 0;
  }

  .template-option.selected .checkbox {
    background-color: #0e639c;
    border-color: #0e639c;
    color: white;
  }

  .template-name {
    flex: 1;
  }

  .no-templates {
    padding: 16px;
    text-align: center;
  }

  .no-templates p {
    margin: 0;
    color: #666;
    font-size: 12px;
    font-style: italic;
  }
</style>
