<script lang="ts">
  import ButtonEditor from './ButtonEditor.svelte';

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

  // Ensure button definition exists
  function ensureButtonDef() {
    if (!config.buttons) {
      config.buttons = {};
    }
    if (!config.buttons[buttonDefName]) {
      config.buttons[buttonDefName] = {};
    }
  }

  // Create a virtual config that maps button definition to a template structure
  let virtualConfig = $derived.by(() => {
    ensureButtonDef();

    return {
      ...config,
      templates: {
        ...config.templates,
        [VIRTUAL_PAGE]: {
          [`button${BUTTON_INDEX}`]: config.buttons[buttonDefName]
        }
      }
    };
  });

  // Sync changes back from virtual template to button definition
  $effect(() => {
    const virtualButton = virtualConfig.templates?.[VIRTUAL_PAGE]?.[`button${BUTTON_INDEX}`];
    if (virtualButton && config.buttons[buttonDefName] !== virtualButton) {
      // Sync changes back
      config.buttons[buttonDefName] = virtualButton;
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
  customTitle={buttonDefName}
/>
