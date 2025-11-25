<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2025 Panayotis Katsaloulis -->

<script lang="ts">
  import ButtonEditor from './ButtonEditor.svelte';
  import { untrack } from 'svelte';

  interface Props {
    config: any;
    buttonDefName: string;
  }

  let { config, buttonDefName }: Props = $props();

  // Create a virtual config that ButtonEditor can work with
  // We'll create a fake template with a single button that mirrors the button definition
  const VIRTUAL_PAGE = '__buttondef_edit__';
  const VIRTUAL_SERIAL = '__virtual__';
  const BUTTON_INDEX = 0;

  // Create a virtual config that maps button definition to a template structure
  // Use $state so it's mutable - ButtonEditor needs to update it
  let virtualConfig = $state<any>({});

  // Initialize/update virtualConfig when buttonDefName or the button def changes
  $effect(() => {
    const buttonDef = config.buttons?.[buttonDefName] || {};

    virtualConfig = {
      ...config,
      templates: {
        ...config.templates,
        [VIRTUAL_PAGE]: {
          [`button${BUTTON_INDEX}`]: buttonDef
        }
      }
    };
  });

  // Sync changes back from virtual template to button definition
  // Track the current button def name to detect when we switch between definitions
  let previousButtonDefName = $state<string>(buttonDefName);
  let previousVirtualButton = $state<any>(undefined);

  $effect(() => {
    const virtualButton = virtualConfig.templates?.[VIRTUAL_PAGE]?.[`button${BUTTON_INDEX}`];

    // If we switched to a different button definition, reset tracking
    if (buttonDefName !== previousButtonDefName) {
      previousButtonDefName = buttonDefName;
      previousVirtualButton = virtualButton;
      return;
    }

    // Skip initial sync - only sync on actual changes after first load
    if (previousVirtualButton === undefined) {
      previousVirtualButton = virtualButton;
      return;
    }

    if (virtualButton && virtualButton !== previousVirtualButton) {
      // Ensure buttons object exists before syncing
      if (!config.buttons) {
        config.buttons = {};
      }

      // Only sync if there's an actual change
      if (config.buttons[buttonDefName] !== virtualButton) {
        config.buttons[buttonDefName] = virtualButton;
      }

      previousVirtualButton = virtualButton;
    }
  });
</script>

<ButtonEditor
  config={virtualConfig}
  currentPage={VIRTUAL_PAGE}
  currentTemplate={VIRTUAL_PAGE}
  buttonIndex={BUTTON_INDEX}
  deviceSerial={VIRTUAL_SERIAL}
  isTemplate={true}
  isButtonDef={true}
  customTitle={buttonDefName}
/>
