<script lang="ts">
  interface Props {
    config: any;
    deviceSerial: string;
    currentPage: string;
    onPageSelected: (pageName: string) => void;
  }

  let { config, deviceSerial, currentPage, onPageSelected }: Props = $props();

  let pageGroup = $derived(config.page_groups?.[deviceSerial] || config.page_groups?.default || {});

  // Pages are flattened at the same level as main_page and restore_mode
  // Filter out known page group fields to get actual pages
  let pages = $derived.by(() => {
    if (!pageGroup) return [];
    const knownFields = ['main_page', 'restore_mode', 'on_tick'];
    return Object.keys(pageGroup).filter(key => !knownFields.includes(key));
  });

  let newPageName = $state("");
  let showPageMenu = $state<string | null>(null);
  let draggedPage = $state<string | null>(null);
  let dragOverPage = $state<string | null>(null);
  let dropPosition = $state<'before' | 'after' | null>(null);

  // Close menu when clicking outside
  $effect(() => {
    if (showPageMenu !== null) {
      const handleClickOutside = (event: MouseEvent) => {
        const target = event.target as HTMLElement;
        if (!target.closest('.page-menu') && !target.closest('.page-menu-btn')) {
          showPageMenu = null;
        }
      };
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });

  function getGroupKey(): string {
    if (!config.page_groups) config.page_groups = {};
    return config.page_groups[deviceSerial] ? deviceSerial : 'default';
  }

  function ensurePageGroup() {
    const groupKey = getGroupKey();
    if (!config.page_groups[groupKey]) {
      config.page_groups[groupKey] = { restore_mode: "main" };
    }
    return groupKey;
  }

  function addPage() {
    if (!newPageName.trim()) return;
    const groupKey = ensurePageGroup();

    // Check if page already exists (pages are flattened)
    if (config.page_groups[groupKey][newPageName]) {
      alert(`Page "${newPageName}" already exists!`);
      return;
    }

    // Pages are flattened at the same level as restore_mode
    // Buttons are also flattened on the page (no .buttons object)
    config.page_groups[groupKey][newPageName] = {};
    // Also update root level since it's flattened in the actual config
    config[groupKey][newPageName] = {};
    onPageSelected(newPageName);
    newPageName = "";
  }

  function deletePage(pageName: string) {
    if (!confirm(`Are you sure you want to delete page "${pageName}"?`)) return;

    const groupKey = getGroupKey();
    const knownFields = ['main_page', 'restore_mode', 'on_tick'];

    // Delete from page_groups
    delete config.page_groups[groupKey][pageName];
    // Also delete from root level
    delete config[groupKey][pageName];

    // Select first available page or create main if none exist
    const remainingPages = Object.keys(config.page_groups[groupKey] || {})
      .filter(key => !knownFields.includes(key));

    if (remainingPages.length > 0) {
      onPageSelected(remainingPages[0]);
    } else {
      config.page_groups[groupKey]['main'] = {};
      config[groupKey]['main'] = {};
      onPageSelected('main');
    }

    showPageMenu = null;
  }

  function duplicatePage(pageName: string) {
    const groupKey = ensurePageGroup();
    const originalPage = config.page_groups[groupKey][pageName];

    let newName = `${pageName}_copy`;
    let counter = 1;
    while (config.page_groups[groupKey][newName]) {
      newName = `${pageName}_copy${counter}`;
      counter++;
    }

    // Deep clone the page
    const clonedPage = JSON.parse(JSON.stringify(originalPage));
    config.page_groups[groupKey][newName] = clonedPage;
    config[groupKey][newName] = clonedPage;
    onPageSelected(newName);
    showPageMenu = null;
  }

  function renamePage(pageName: string) {
    const newName = prompt(`Rename page "${pageName}" to:`, pageName);
    if (!newName || newName === pageName) return;

    const groupKey = getGroupKey();
    if (config.page_groups[groupKey][newName]) {
      alert(`Page "${newName}" already exists!`);
      return;
    }

    config.page_groups[groupKey][newName] = config.page_groups[groupKey][pageName];
    config[groupKey][newName] = config[groupKey][pageName];
    delete config.page_groups[groupKey][pageName];
    delete config[groupKey][pageName];
    onPageSelected(newName);
    showPageMenu = null;
  }

  function handleDragStart(e: DragEvent, page: string) {
    draggedPage = page;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', page);
    }
  }

  function handleDragOver(e: DragEvent, page: string) {
    if (!draggedPage || draggedPage === page) return;

    e.preventDefault();
    e.stopPropagation();

    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move';
    }

    // Determine if we should insert before or after based on mouse position
    const element = e.currentTarget as HTMLElement;
    const rect = element.getBoundingClientRect();
    const midpoint = rect.top + rect.height / 2;

    dragOverPage = page;
    dropPosition = e.clientY < midpoint ? 'before' : 'after';
  }

  function handleDragLeave() {
    dragOverPage = null;
    dropPosition = null;
  }

  function handleDrop(e: DragEvent, targetPage: string) {
    e.preventDefault();
    e.stopPropagation();

    if (!draggedPage || draggedPage === targetPage) {
      draggedPage = null;
      dragOverPage = null;
      dropPosition = null;
      return;
    }

    const groupKey = getGroupKey();
    const pageList = pages.slice();
    const draggedIndex = pageList.indexOf(draggedPage);
    let targetIndex = pageList.indexOf(targetPage);

    // Remove dragged page first
    pageList.splice(draggedIndex, 1);

    // Recalculate target index after removal
    targetIndex = pageList.indexOf(targetPage);

    // Insert based on drop position
    if (dropPosition === 'after') {
      pageList.splice(targetIndex + 1, 0, draggedPage);
    } else {
      pageList.splice(targetIndex, 0, draggedPage);
    }

    // Rebuild the page group object in the new order
    const knownFields = ['main_page', 'restore_mode', 'on_tick'];
    const newPageGroup: any = {};

    // Keep known fields at the beginning
    for (const field of knownFields) {
      if (config.page_groups[groupKey][field] !== undefined) {
        newPageGroup[field] = config.page_groups[groupKey][field];
      }
    }

    // Add pages in new order
    for (const pageName of pageList) {
      newPageGroup[pageName] = config.page_groups[groupKey][pageName];
    }

    config.page_groups[groupKey] = newPageGroup;
    config[groupKey] = newPageGroup;

    // Select the dragged page after dropping
    onPageSelected(draggedPage);

    draggedPage = null;
    dragOverPage = null;
    dropPosition = null;
  }

  function handleDragEnd() {
    draggedPage = null;
    dragOverPage = null;
    dropPosition = null;
  }
</script>

<div class="page-list">
  <h3>Pages</h3>

  <div class="pages">
    {#each pages as page}
      <div
        class="page-item-wrapper"
        class:drag-over-before={dragOverPage === page && dropPosition === 'before'}
        class:drag-over-after={dragOverPage === page && dropPosition === 'after'}
        class:dragging={draggedPage === page}
        ondragover={(e) => handleDragOver(e, page)}
        ondragleave={handleDragLeave}
        ondrop={(e) => handleDrop(e, page)}
        ondragenter={(e) => e.preventDefault()}
      >
        <span
          class="drag-handle"
          draggable="true"
          ondragstart={(e) => handleDragStart(e, page)}
          ondragend={handleDragEnd}
        >
          ⋮⋮
        </span>
        <button
          class="page-item"
          class:active={page === currentPage}
          onclick={() => onPageSelected(page)}
        >
          {page}
        </button>
        <button
          class="page-menu-btn"
          onclick={(e) => {
            e.stopPropagation();
            showPageMenu = showPageMenu === page ? null : page;
          }}
        >
          ⋮
        </button>
        {#if showPageMenu === page}
          <div class="page-menu">
            <button onclick={() => renamePage(page)}>✏️ Rename</button>
            <button onclick={() => duplicatePage(page)}>📋 Duplicate</button>
            <button class="danger" onclick={() => deletePage(page)}>🗑️ Delete</button>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="add-page">
    <input
      type="text"
      placeholder="New page name"
      bind:value={newPageName}
      onkeydown={(e) => e.key === 'Enter' && addPage()}
    />
    <button onclick={addPage}>+ Add</button>
  </div>
</div>

<style>
  .page-list {
    margin-top: 20px;
  }

  h3 {
    margin: 0 0 12px 0;
    font-size: 14px;
    font-weight: 600;
    color: #cccccc;
  }

  .pages {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
  }

  .page-item-wrapper {
    position: relative;
    display: flex;
    gap: 4px;
  }

  .page-item-wrapper.dragging {
    opacity: 0.4;
  }

  .page-item-wrapper.drag-over-before::before {
    content: '';
    position: absolute;
    top: -2px;
    left: 0;
    right: 0;
    height: 3px;
    background-color: #0e639c;
    border-radius: 2px;
    z-index: 10;
  }

  .page-item-wrapper.drag-over-after::after {
    content: '';
    position: absolute;
    bottom: -2px;
    left: 0;
    right: 0;
    height: 3px;
    background-color: #0e639c;
    border-radius: 2px;
    z-index: 10;
  }

  .drag-handle {
    display: flex;
    align-items: center;
    padding: 0 4px;
    cursor: grab;
    color: #666;
    font-size: 12px;
    user-select: none;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .drag-handle:hover {
    color: #999;
  }

  .page-item {
    flex: 1;
    padding: 8px 12px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    font-size: 13px;
  }

  .page-item:hover {
    background-color: #4a4a4a;
  }

  .page-item.active {
    background-color: #0e639c;
    border-color: #1177bb;
  }

  .page-menu-btn {
    padding: 8px 10px;
    background-color: #3c3c3c;
    color: #888;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
  }

  .page-menu-btn:hover {
    background-color: #4a4a4a;
    color: #cccccc;
  }

  .page-menu {
    position: absolute;
    right: 0;
    top: 100%;
    margin-top: 4px;
    background-color: #2d2d30;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    z-index: 100;
    min-width: 150px;
    overflow: hidden;
  }

  .page-menu button {
    display: block;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: #cccccc;
    text-align: left;
    cursor: pointer;
    font-size: 13px;
  }

  .page-menu button:hover {
    background-color: #3c3c3c;
  }

  .page-menu button.danger {
    color: #f48771;
  }

  .page-menu button.danger:hover {
    background-color: #5a1d1d;
  }

  .add-page {
    display: flex;
    gap: 4px;
  }

  input {
    flex: 1;
    padding: 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 4px;
    font-size: 13px;
  }

  .add-page button {
    padding: 8px 12px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 13px;
  }

  .add-page button:hover {
    background-color: #1177bb;
  }
</style>
