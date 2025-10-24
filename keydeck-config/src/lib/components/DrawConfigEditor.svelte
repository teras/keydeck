<script lang="ts">
  import ColorPicker from './ColorPicker.svelte';

  interface Props {
    drawConfig: any;
    onUpdate: (drawConfig: any) => void;
    disabled?: boolean;
  }

  let { drawConfig, onUpdate, disabled = false }: Props = $props();

  function updateDraw(updates: any) {
    onUpdate({ ...drawConfig, ...updates });
  }

  function removeField(fieldName: string) {
    const { [fieldName]: _, ...rest } = drawConfig;
    onUpdate(rest);
  }
</script>

<div class="draw-config-editor">
  <!-- Graphic Type -->
  <div class="form-row">
    <label>Type</label>
    <select
      value={drawConfig.type || 'gauge'}
      onchange={(e) => updateDraw({ type: e.currentTarget.value })}
      disabled={disabled}
    >
      <option value="gauge">Gauge</option>
      <option value="bar">Bar</option>
      <option value="multi_bar">Multi-Bar</option>
    </select>
  </div>

  <!-- Value (data source) -->
  <div class="form-row">
    <label>Value</label>
    <input
      type="text"
      value={drawConfig.value || ''}
      oninput={(e) => updateDraw({ value: e.currentTarget.value })}
      placeholder="${'service:name'} or ${'{env:VAR}'}"
      disabled={disabled}
    />
    <p class="field-help">Data source expression (e.g., ${'service:cpu'}). For multi-bar types, returns space-separated numbers.</p>
  </div>

  <!-- Range [min, max] -->
  <div class="form-row">
    <label>Range [min, max]</label>
    <div class="range-inputs">
      <input
        type="number"
        value={drawConfig.range?.[0] ?? 0}
        oninput={(e) => {
          const range = drawConfig.range || [0, 100];
          updateDraw({ range: [parseFloat(e.currentTarget.value), range[1]] });
        }}
        placeholder="Min"
        step="any"
        disabled={disabled}
      />
      <span class="range-separator">to</span>
      <input
        type="number"
        value={drawConfig.range?.[1] ?? 100}
        oninput={(e) => {
          const range = drawConfig.range || [0, 100];
          updateDraw({ range: [range[0], parseFloat(e.currentTarget.value)] });
        }}
        placeholder="Max"
        step="any"
        disabled={disabled}
      />
    </div>
  </div>

  <!-- Color (single) or Color Map (gradient) -->
  <div class="form-row">
    <label>Color Mode</label>
    <select
      value={drawConfig.color_map ? 'gradient' : 'solid'}
      onchange={(e) => {
        if (e.currentTarget.value === 'solid') {
          const { color_map, ...rest } = drawConfig;
          onUpdate({ ...rest, color: drawConfig.color || '#00ff00' });
        } else {
          const { color, ...rest } = drawConfig;
          onUpdate({ ...rest, color_map: drawConfig.color_map || [[0, '#ff0000'], [100, '#00ff00']] });
        }
      }}
      disabled={disabled}
    >
      <option value="solid">Solid Color</option>
      <option value="gradient">Color Gradient</option>
    </select>
  </div>

  {#if drawConfig.color_map}
    <!-- Color Map Editor -->
    <div class="form-row">
      <label>Color Gradient Map</label>
      <div class="color-map-list">
        {#each drawConfig.color_map as entry, i}
          <div class="color-map-entry">
            <input
              type="number"
              class="threshold-input"
              value={Array.isArray(entry) ? entry[0] : 0}
              oninput={(e) => {
                const colorMap = [...(drawConfig.color_map || [])];
                const currentEntry = colorMap[i];
                const color = Array.isArray(currentEntry) ? currentEntry[1] : '#000000';
                colorMap[i] = [parseFloat(e.currentTarget.value), color];
                updateDraw({ color_map: colorMap });
              }}
              placeholder="Threshold"
              step="any"
              disabled={disabled}
            />
            <ColorPicker
              color={Array.isArray(entry) ? entry[1] : '#000000'}
              onColorChange={(newColor) => {
                const colorMap = [...(drawConfig.color_map || [])];
                const currentEntry = colorMap[i];
                const threshold = Array.isArray(currentEntry) ? currentEntry[0] : 0;
                colorMap[i] = [threshold, newColor];
                updateDraw({ color_map: colorMap });
              }}
              disabled={disabled}
            />
            {#if !disabled}
              <button
                class="btn-remove-color-map"
                onclick={() => {
                  const colorMap = [...(drawConfig.color_map || [])];
                  colorMap.splice(i, 1);
                  updateDraw({ color_map: colorMap });
                }}
                title="Remove"
              >×</button>
            {/if}
          </div>
        {/each}
        {#if !disabled}
          <button
            class="btn-add-color-map"
            onclick={() => {
              const colorMap = [...(drawConfig.color_map || [])];
              colorMap.push([0, '#ffffff']);
              updateDraw({ color_map: colorMap });
            }}
          >+ Add Color Stop</button>
        {/if}
      </div>
      <p class="field-help">Format: [[threshold, color], ...]. Values are smoothly interpolated.</p>
    </div>
  {:else}
    <!-- Single Color -->
    <div class="form-row">
      <label>Color</label>
      <ColorPicker
        color={drawConfig.color || '#00ff00'}
        onColorChange={(newColor) => updateDraw({ color: newColor })}
        disabled={disabled}
      />
    </div>
  {/if}

  <!-- Optional: Position [x, y] -->
  <div class="form-row">
    <label>Position [x, y]</label>
    <div class="position-inputs">
      <input
        type="number"
        value={drawConfig.position?.[0] ?? ''}
        oninput={(e) => {
          const val = e.currentTarget.value;
          if (val === '' && !drawConfig.position?.[1]) {
            removeField('position');
          } else {
            const position = drawConfig.position || [0, 0];
            updateDraw({ position: [parseInt(val) || 0, position[1]] });
          }
        }}
        placeholder="X"
        disabled={disabled}
      />
      <span>,</span>
      <input
        type="number"
        value={drawConfig.position?.[1] ?? ''}
        oninput={(e) => {
          const val = e.currentTarget.value;
          if (val === '' && !drawConfig.position?.[0]) {
            removeField('position');
          } else {
            const position = drawConfig.position || [0, 0];
            updateDraw({ position: [position[0], parseInt(val) || 0] });
          }
        }}
        placeholder="Y"
        disabled={disabled}
      />
    </div>
    <p class="field-help">Position from top-left corner. Leave empty for centered.</p>
  </div>

  <!-- Optional: Size [W × H] -->
  <div class="form-row">
    <label>Size [W × H] (pixels)</label>
    <div class="dimension-inputs">
      <input
        type="number"
        value={drawConfig.width ?? ''}
        oninput={(e) => {
          const val = e.currentTarget.value;
          if (val === '') {
            removeField('width');
          } else {
            updateDraw({ width: parseInt(val) });
          }
        }}
        placeholder="W"
        min="1"
        disabled={disabled}
      />
      <span>×</span>
      <input
        type="number"
        value={drawConfig.height ?? ''}
        oninput={(e) => {
          const val = e.currentTarget.value;
          if (val === '') {
            removeField('height');
          } else {
            updateDraw({ height: parseInt(val) });
          }
        }}
        placeholder="H"
        min="1"
        disabled={disabled}
      />
    </div>
    <p class="field-help">Graphic dimensions. Leave empty for auto-sizing (button size - 2*padding).</p>
  </div>

  <!-- Optional: Padding -->
  <div class="form-row">
    <label>Padding (pixels)</label>
    <input
      type="number"
      value={drawConfig.padding ?? ''}
      oninput={(e) => {
        const val = e.currentTarget.value;
        if (val === '') {
          removeField('padding');
        } else {
          updateDraw({ padding: parseInt(val) });
        }
      }}
      placeholder="5 (default)"
      min="0"
      disabled={disabled}
    />
  </div>

  <!-- Optional: Direction (for bar and multi_bar types) -->
  {#if drawConfig.type === 'bar' || drawConfig.type === 'multi_bar'}
    <div class="form-row">
      <label>Direction</label>
      <select
        value={drawConfig.direction || ''}
        onchange={(e) => {
          const val = e.currentTarget.value;
          if (val === '') {
            removeField('direction');
          } else {
            updateDraw({ direction: val });
          }
        }}
        disabled={disabled}
      >
        <option value="">Auto (default: bottom to top)</option>
        <option value="left_to_right">Left to Right</option>
        <option value="right_to_left">Right to Left</option>
        <option value="top_to_bottom">Top to Bottom</option>
        <option value="bottom_to_top">Bottom to Top</option>
      </select>
      <p class="field-help">Fill direction. Default: bottom_to_top.</p>
    </div>
  {/if}

  <!-- Optional: Segments (for bar and multi_bar types) -->
  {#if drawConfig.type === 'bar' || drawConfig.type === 'multi_bar'}
    <div class="form-row">
      <label>Segments</label>
      <input
        type="number"
        value={drawConfig.segments ?? ''}
        oninput={(e) => {
          const val = e.currentTarget.value;
          if (val === '') {
            removeField('segments');
          } else {
            updateDraw({ segments: parseInt(val) });
          }
        }}
        placeholder="Continuous (default)"
        min="1"
        disabled={disabled}
      />
      <p class="field-help">Number of discrete blocks for segmented display (VU meter style). Leave empty for continuous fill.</p>
    </div>
  {/if}

  <!-- Optional: Bar Spacing (for multi_bar type) -->
  {#if drawConfig.type === 'multi_bar'}
    <div class="form-row">
      <label>Bar Spacing (pixels)</label>
      <input
        type="number"
        value={drawConfig.bar_spacing ?? ''}
        oninput={(e) => {
          const val = e.currentTarget.value;
          if (val === '') {
            removeField('bar_spacing');
          } else {
            updateDraw({ bar_spacing: parseInt(val) });
          }
        }}
        placeholder="2 (default)"
        min="0"
        disabled={disabled}
      />
    </div>
  {/if}
</div>

<style>
  .draw-config-editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-row label {
    font-size: 11px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
  }

  .form-row input,
  .form-row select {
    padding: 6px 8px;
    background-color: #3c3c3c;
    color: #cccccc;
    border: 1px solid #555;
    border-radius: 3px;
    font-size: 12px;
  }

  .form-row input:focus,
  .form-row select:focus {
    outline: none;
    border-color: #0e639c;
  }

  .field-help {
    font-size: 11px;
    color: #888;
    margin: 2px 0 0 0;
    font-style: italic;
  }

  .range-inputs {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .range-inputs input {
    flex: 1;
    min-width: 0;
  }

  .range-separator {
    color: #888;
    font-size: 12px;
    flex-shrink: 0;
  }

  .position-inputs {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .position-inputs input {
    flex: 1;
    min-width: 0;
  }

  .position-inputs span {
    color: #888;
    font-size: 12px;
    flex-shrink: 0;
  }

  .dimension-inputs {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dimension-inputs input {
    flex: 1;
    min-width: 0;
  }

  .dimension-inputs span {
    color: #888;
    font-size: 12px;
    flex-shrink: 0;
  }

  .color-map-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .color-map-entry {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px;
    background-color: #333;
    border: 1px solid #555;
    border-radius: 4px;
  }

  .threshold-input {
    flex: 1;
    min-width: 80px;
  }

  .btn-remove-color-map {
    background: none;
    border: none;
    color: #f48771;
    cursor: pointer;
    font-size: 18px;
    padding: 0 4px;
    line-height: 1;
  }

  .btn-remove-color-map:hover {
    color: #ff6b6b;
  }

  .btn-add-color-map {
    padding: 6px 12px;
    background-color: #0e639c;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
  }

  .btn-add-color-map:hover {
    background-color: #1177bb;
  }
</style>
