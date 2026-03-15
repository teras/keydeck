# Device JSON Format Documentation

## Overview

Each device is defined in a single JSON file that contains all device-specific information, quirks, and protocol parameters. The JSON format is designed to completely eliminate hardcoded device logic from the protocol implementation.

## File Naming Convention

Files are named using the pattern: `Manufacturer-Model.json`

**Examples:**
- `Ajazz-AKP153R.json`
- `Mirabox-HSV293S.json`
- `Mars-Gaming-MSD-ONE.json`

**Conflicts:** When multiple devices share the same manufacturer and model name but have different Product IDs, append the PID:
- `Ajazz-AKP153E-0x1010.json` (v1 protocol)
- `Ajazz-AKP153E-0x3010.json` (v3 protocol - revision 2)

## Field Reference

### `hardware` (required)

Hardware USB identifiers for device detection.

---

#### `vendor_id` (string, required)

USB Vendor ID in hexadecimal format used to identify the device manufacturer.

- **Format:** `"0xXXXX"` (always 4 hex digits with `0x` prefix)
- **Common values:**
  - `"0x0300"` - Ajazz devices
  - `"0x5548"` - Mirabox (older models)
  - `"0x6602"` - Mirabox N3
  - `"0x6603"` - Mirabox (newer, v3 protocol)
  - `"0x0b00"` - Mars Gaming
  - `"0x0c00"` - Mad Dog
  - `"0x0a00"` - Risemode
  - `"0x0500"` - TMICE
  - `"0x1500"` - Soomfon
  - `"0x0200"` - Redragon

---

#### `product_id` (string, required)

USB Product ID in hexadecimal format used to identify the specific device model.

- **Format:** `"0xXXXX"` (always 4 hex digits with `0x` prefix)
- **Examples:**
  - `"0x1020"` - Ajazz AKP153R
  - `"0x1001"` - Ajazz AKP03
  - `"0x6670"` - Mirabox HSV293S

**Note:** All Mirabox/Ajazz devices use the same HID Usage Page (`65440` / 0xFF00) and Usage ID (`1`). These values are hardcoded as constants and do not need to be specified in JSON files.

---

### `info` (required)

Human-readable device information and metadata.

---

#### `human_name` (string, required)

Display name shown to users in UI and logs.

- **Examples:**
  - `"Ajazz AKP153R"`
  - `"Mirabox HSV293S"`
  - `"Mars Gaming MSD-ONE"`
  - `"Ajazz AKP03E (rev. 2)"`
- **Guidelines:**
  - Include manufacturer and model
  - Add revision info for variants (e.g., `"(rev. 2)"`)
  - Keep concise but descriptive

---

#### `device_namespace` (string, required)

Two-character identifier for the device family. This is a legacy field from the OpenDeck plugin system that groups devices with similar layouts together.

- **Format:** Exactly 2 ASCII characters
- **Values:**
  - `"99"` - AKP153 family (18 buttons, 3×6 grid, no encoders)
  - `"n3"` - AKP03 family (9 buttons, 3×3 grid, 3 encoders)
- **Purpose:** Originally used by OpenDeck plugins to identify device families. The value must match the `DeviceNamespace` field in OpenDeck plugin manifests.
- **Note:** For keydeck, this field is informational only and not used for device detection or behavior.

---

#### `manufacturer` (string, optional)

Manufacturer name for reference and display purposes.

- **Examples:**
  - `"Ajazz"`
  - `"Mirabox"`
  - `"Mars Gaming"`
  - `"Mad Dog"`
  - `"Redragon"`

---

#### `model` (string, optional)

Model identifier for reference purposes.

- **Examples:**
  - `"AKP153R"`
  - `"HSV293S"`
  - `"N3EN"`
  - `"SS-551"`

---

### `protocol` (required)

Protocol version and communication parameters.

---

#### `protocol_version` (integer, required)

The Mirabox/Ajazz protocol generation determines packet sizes, features, and initialization sequences.

- **Values:**
  - `0` - Legacy (512-byte packets, no serial, firmware "1.0.0.0")
  - `1` - Standard (512-byte packets, ACK headers, serial support)
  - `2` - Modern (1024-byte packets, unique serials, requires STP command)
  - `3` - Latest (1024-byte packets, dual keypress states, GIF support, long press)

- **Protocol comparison:**

  | Version | Packet Size | Serial Number | ACK Headers | Dual States | GIF Support |
  |---------|-------------|---------------|-------------|-------------|-------------|
  | 0       | 512 bytes   | No            | No          | No          | No          |
  | 1       | 512 bytes   | Yes           | Yes         | No          | No          |
  | 2       | 1024 bytes  | Yes           | Yes         | No          | No          |
  | 3       | 1024 bytes  | Yes           | Yes         | Yes         | Yes         |

- **Impact:**
  - Packet size: 512 bytes for v0/v1, 1024 bytes for v2/v3
  - Initialization: v2+ requires explicit STP command after screen clear
  - Button states: v3 reports both press and release events
  - Animation: v3 supports GIF images on buttons

---

#### `device_mode` (integer, optional)

Device mode to set on initialization for multimodal devices. Sends a MOD command with this value before any other commands.

- **Values:** `0`-`255` (device-specific meaning)
- **Known values:**
  - `2` - StreamDock N1 (software mode, sent via `switchMode()` on open)
- **When to use:** Some devices support multiple operational modes (e.g., macropad vs mixer). The mode must be set before the device will accept other commands.
- **Default:** Not set (no MOD command sent)

---

#### `report_id` (integer, optional)

HID report ID override. Most devices use the default `0x00`. The K1Pro uses `0x04`.

- **Values:** `0`-`255`
- **Default:** `0` (standard HID report ID)
- **Impact:** When non-zero, all byte offsets in HID reads and writes shift by 1 to account for the report ID byte. This affects CRT command buffers, image data transfers, and input event parsing.
- **Known values:**
  - `4` - StreamDock K1Pro

---

### `layout` (required)

Physical button and encoder layout configuration.

---

#### `rows` (integer, required)

Number of button rows in the grid layout.

- **Common values:**
  - `3` - All current AKP153 and AKP03 devices

---

#### `cols` (integer, required)

Number of button columns in the grid layout.

- **Common values:**
  - `6` - AKP153 family (3×6 grid = 18 buttons)
  - `3` - AKP03 family (3×3 grid = 9 buttons)

**Note:** Total key count is automatically calculated as `rows × cols` and does not need to be specified in the JSON.

---

#### `encoder_count` (integer, optional, default: 0)

Number of rotary encoders (twist knobs with press buttons).

- **Common values:**
  - `0` - AKP153 family (no encoders)
  - `3` - AKP03 family (3 encoders with press buttons)
- **Note:** Encoder press buttons are separate from the key grid and not included in the calculated key count.

---

### `image_format` (required)

Button image specifications.

#### `mode` (string, required)
Image encoding format.

**Values:**
- `"JPEG"` - JPEG compressed (all current devices)
- `"BMP"` - Bitmap uncompressed (legacy, not used)

#### `default_size` (array[2], required)
Default button image dimensions [width, height] in pixels.

**Format:** `[width, height]`

**Common values:**
- `[85, 85]` - v1 protocol devices (AKP153 family)
- `[95, 95]` - v3 protocol devices (AKP153E rev.2)
- `[60, 60]` - AKP03 family

**Note:** Individual buttons can override this via `per_button_overrides`.

#### `rotation` (string, required)
Image rotation before sending to device.

**Values:**
- `"Rot0"` - No rotation
- `"Rot90"` - Rotate 90° clockwise
- `"Rot180"` - Rotate 180°
- `"Rot270"` - Rotate 270° clockwise (90° counter-clockwise)

**Common values:**
- `"Rot90"` - Most AKP153 devices, v3 AKP03 devices
- `"Rot0"` - v2 AKP03 devices

**Why needed:** Device screens may be physically rotated relative to the expected orientation.

#### `mirror` (string, required)
Image mirroring before sending to device.

**Values:**
- `"None"` - No mirroring
- `"X"` - Flip horizontally
- `"Y"` - Flip vertically
- `"Both"` - Flip both horizontally and vertically

**Common values:**
- `"Both"` - AKP153 family
- `"None"` - AKP03 family

**Why needed:** Device screens may be mirrored relative to the expected orientation.

#### `per_button_overrides` (object, optional)
Per-button image format overrides.

**Format:** Object with button indices (as strings) as keys, override objects as values.

**Example:**
```json
{
  "5": {"size": [82, 82]},
  "11": {"size": [82, 82]},
  "17": {"size": [82, 82]}
}
```

**Override fields:**
- `size` (array[2], optional) - Override image size for this button
- `rotation` (string, optional) - Override rotation for this button
- `mirror` (string, optional) - Override mirroring for this button

**Use case:** Some v3 devices (e.g., Ajazz AKP153E rev.2) have edge buttons with slightly different sizes (82×82) while center buttons use 95×95.

---

### `input_mapping` (required)

Input event mapping and quirks.

#### `button_remap` (array[integer] or null, optional)
Button index remapping array for devices with non-sequential button IDs.

**Format:** Array mapping logical button indices to physical button IDs, or `null` if no remapping needed.

**Length:** Must equal `key_count`

**Example:**
```json
[12, 9, 6, 3, 0, 15, 13, 10, 7, 4, 1, 16, 14, 11, 8, 5, 2, 17]
```

**How it works:**
- Array index = logical button number (0-based, left-to-right, top-to-bottom)
- Array value = physical device button ID

**Example:** Button at logical position 0 (top-left) is actually device button ID 12.

**When to use:**
- AKP153 family devices report non-sequential button IDs
- AKP03 family devices have sequential IDs (use `null`)

**Why needed:** Devices may report button presses using arbitrary IDs that don't match the physical layout. This array maps logical positions to device IDs.

#### `encoder_twist_map` (object, optional)
Maps raw HID input bytes to encoder twist events.

**Format:** Object with raw byte values (as strings) as keys, twist event objects as values.

**Example:**
```json
{
  "144": {"encoder": 0, "direction": -1},
  "145": {"encoder": 0, "direction": 1},
  "80": {"encoder": 1, "direction": -1},
  "81": {"encoder": 1, "direction": 1}
}
```

**Fields:**
- `encoder` (integer) - Encoder index (0-based)
- `direction` (integer) - Twist direction: `-1` for CCW, `1` for CW

**When to use:**
- AKP03 family devices with encoders
- Empty object `{}` for devices without encoders

**Why needed:** Devices may use custom byte values to represent encoder events that need translation.

#### `encoder_press_map` (object, optional)
Maps raw HID input bytes to encoder button press events.

**Format:** Object with raw byte values (as strings) as keys, encoder indices as values.

**Example:**
```json
{
  "51": 0,
  "53": 1,
  "52": 2
}
```

**When to use:**
- AKP03 family devices with encoders
- Empty object `{}` for devices without encoders

**Why needed:** Encoder button presses may be reported using custom byte values.

#### `non_display_buttons` (array[integer], optional)
List of button indices that don't have displays.

**Format:** Array of button indices (0-based)

**Example:**
```json
[7, 8, 9]
```

**When to use:**
- AKP03 family: buttons 7, 8, 9 are encoder buttons without displays
- AKP153 family: empty array `[]` (all buttons have displays)

**Why needed:** Prevents trying to send images to buttons without screens, which would cause errors.

---

### `background` (optional)

Background/touchscreen image configuration. Present on devices with a touchscreen display
behind or around the button grid.

#### `resolution` (array[2], required)

Background image dimensions `[width, height]` in pixels.

- **Examples:**
  - `[800, 480]` - N4Pro, N4, 293, 293V3, K1Pro
  - `[1024, 600]` - XL
  - `[480, 272]` - M18, M3
  - `[480, 854]` - N1 (portrait orientation)
  - `[854, 480]` - 293s, 293sV3

#### `mode` (string, required)

Image encoding format for background images.

- **Values:** `"JPEG"` (all current devices)

#### `rotation` (string, required)

Image rotation for the background. Same values as `image_format.rotation`.

#### `mirror` (string, required)

Image mirroring for the background. Same values as `image_format.mirror`.

---

### `led` (optional)

RGB LED strip configuration for devices with ambient lighting around the button grid.

#### `count` (integer, required)

Number of individually addressable RGB LEDs.

- **Known values:**
  - `4` - StreamDock N4Pro
  - `6` - StreamDock XL
  - `24` - StreamDock M18

**Note:** When present, the device supports CRT commands LBLIG (brightness), SETLB (color),
and DELED (reset). See [PROTOCOL.md](PROTOCOL.md) for details.

---

### `quirks` (required)

Device-specific behavior flags.

#### `needs_button_remapping` (boolean, required)
Whether device requires button index remapping.

**Values:**
- `true` - Device uses non-sequential button IDs (use `button_remap` array)
- `false` - Device uses sequential button IDs (ignore `button_remap`)

**Common values:**
- `true` - All AKP153 family devices
- `false` - All AKP03 family devices

#### `has_non_display_buttons` (boolean, required)
Whether device has buttons without displays.

**Values:**
- `true` - Device has buttons that shouldn't receive image commands (check `non_display_buttons`)
- `false` - All buttons have displays

**Common values:**
- `false` - AKP153 family
- `true` - AKP03 family (encoder buttons)

#### `image_remap_only` (boolean, optional, default: false)
When `true`, `button_remap` is applied only to image slot addressing, not to input events.

Some devices number image slots and button presses in different physical orders
(e.g. image slots bottom-to-top but presses top-to-bottom). For these devices,
the remap fixes image positioning while input already reports correct indices.

**Values:**
- `true` - Remap images only (e.g. MagTran M3)
- `false` (default) - Remap both images and input (e.g. AKP153 family)

#### `force_encoder_toggle` (boolean, optional, default: false)
Forces encoder press events to toggle mode (synthesize press+release) even when the
protocol version (>2) would normally enable separate press/release state tracking.

Some devices report encoder press events unreliably in dual-state mode despite
supporting it for regular buttons. This quirk forces the encoder handling to
emit paired Down+Up events for every press, like protocol v1/v2 devices.

**Values:**
- `true` - Force toggle mode for encoders
- `false` (default) - Use protocol version to determine encoder state handling

---

## Complete Examples

### AKP153 Family Device (18 buttons, no encoders)

```json
{
  "hardware": {
    "vendor_id": "0x0300",
    "product_id": "0x1020"
  },
  "info": {
    "human_name": "Ajazz AKP153R",
    "device_namespace": "99",
    "manufacturer": "Ajazz",
    "model": "AKP153R"
  },
  "protocol": {
    "protocol_version": 1
  },
  "layout": {
    "rows": 3,
    "cols": 6,
    "encoder_count": 0
  },
  "image_format": {
    "mode": "JPEG",
    "default_size": [85, 85],
    "rotation": "Rot90",
    "mirror": "Both",
    "per_button_overrides": {}
  },
  "input_mapping": {
    "button_remap": [12, 9, 6, 3, 0, 15, 13, 10, 7, 4, 1, 16, 14, 11, 8, 5, 2, 17],
    "encoder_twist_map": {},
    "encoder_press_map": {},
    "non_display_buttons": []
  },
  "quirks": {
    "needs_button_remapping": true,
    "has_non_display_buttons": false
  }
}
```

### AKP03 Family Device (9 buttons, 3 encoders)

```json
{
  "hardware": {
    "vendor_id": "0x0300",
    "product_id": "0x1001"
  },
  "info": {
    "human_name": "Ajazz AKP03",
    "device_namespace": "n3",
    "manufacturer": "Ajazz",
    "model": "AKP03"
  },
  "protocol": {
    "protocol_version": 2
  },
  "layout": {
    "rows": 3,
    "cols": 3,
    "encoder_count": 3
  },
  "image_format": {
    "mode": "JPEG",
    "default_size": [60, 60],
    "rotation": "Rot0",
    "mirror": "None",
    "per_button_overrides": {}
  },
  "input_mapping": {
    "button_remap": null,
    "encoder_twist_map": {
      "144": {"encoder": 0, "direction": -1},
      "145": {"encoder": 0, "direction": 1},
      "80": {"encoder": 1, "direction": -1},
      "81": {"encoder": 1, "direction": 1},
      "96": {"encoder": 2, "direction": -1},
      "97": {"encoder": 2, "direction": 1}
    },
    "encoder_press_map": {
      "51": 0,
      "53": 1,
      "52": 2
    },
    "non_display_buttons": [7, 8, 9]
  },
  "quirks": {
    "needs_button_remapping": false,
    "has_non_display_buttons": true
  }
}
```

### v3 Device with Per-Button Overrides

```json
{
  "hardware": {
    "vendor_id": "0x0300",
    "product_id": "0x3010"
  },
  "info": {
    "human_name": "Ajazz AKP153E (rev. 2)",
    "device_namespace": "99",
    "manufacturer": "Ajazz",
    "model": "AKP153E"
  },
  "protocol": {
    "protocol_version": 3
  },
  "layout": {
    "rows": 3,
    "cols": 6,
    "encoder_count": 0
  },
  "image_format": {
    "mode": "JPEG",
    "default_size": [95, 95],
    "rotation": "Rot90",
    "mirror": "Both",
    "per_button_overrides": {
      "5": {"size": [82, 82]},
      "11": {"size": [82, 82]},
      "17": {"size": [82, 82]}
    }
  },
  "input_mapping": {
    "button_remap": [12, 9, 6, 3, 0, 15, 13, 10, 7, 4, 1, 16, 14, 11, 8, 5, 2, 17],
    "encoder_twist_map": {},
    "encoder_press_map": {},
    "non_display_buttons": []
  },
  "quirks": {
    "needs_button_remapping": true,
    "has_non_display_buttons": false
  }
}
```

### StreamDock Device with LEDs, Background, and Encoders

```json
{
  "hardware": {
    "vendor_id": "0x5548",
    "product_id": "0x1008"
  },
  "info": {
    "human_name": "StreamDock N4Pro",
    "device_namespace": "np",
    "manufacturer": "Mirabox",
    "model": "StreamDock N4Pro"
  },
  "protocol": {
    "protocol_version": 3
  },
  "layout": {
    "rows": 2,
    "cols": 5,
    "encoder_count": 4
  },
  "image_format": {
    "mode": "JPEG",
    "default_size": [112, 112],
    "rotation": "Rot180",
    "mirror": "None",
    "per_button_overrides": {}
  },
  "input_mapping": {
    "button_remap": [5, 6, 7, 8, 9, 0, 1, 2, 3, 4],
    "encoder_twist_map": {
      "160": { "encoder": 0, "direction": -1 },
      "161": { "encoder": 0, "direction": 1 },
      "80": { "encoder": 1, "direction": -1 },
      "81": { "encoder": 1, "direction": 1 },
      "144": { "encoder": 2, "direction": -1 },
      "145": { "encoder": 2, "direction": 1 },
      "112": { "encoder": 3, "direction": -1 },
      "113": { "encoder": 3, "direction": 1 }
    },
    "encoder_press_map": {
      "55": 0,
      "53": 1,
      "51": 2,
      "54": 3
    },
    "non_display_buttons": []
  },
  "background": {
    "resolution": [800, 480],
    "mode": "JPEG",
    "rotation": "Rot180",
    "mirror": "None"
  },
  "quirks": {
    "needs_button_remapping": true
  },
  "led": {
    "count": 4
  }
}
```

### Device with Custom HID Report ID

```json
{
  "hardware": {
    "vendor_id": "0x6603",
    "product_id": "0x1015"
  },
  "info": {
    "human_name": "StreamDock K1Pro",
    "device_namespace": "kp",
    "manufacturer": "Mirabox",
    "model": "StreamDock K1Pro"
  },
  "protocol": {
    "protocol_version": 3,
    "report_id": 4
  },
  "layout": {
    "rows": 2,
    "cols": 3,
    "encoder_count": 3
  },
  "image_format": {
    "mode": "JPEG",
    "default_size": [64, 64],
    "rotation": "Rot90",
    "mirror": "None",
    "per_button_overrides": {}
  },
  "input_mapping": {
    "button_remap": [4, 2, 0, 5, 3, 1],
    "encoder_twist_map": {
      "80": { "encoder": 0, "direction": -1 },
      "81": { "encoder": 0, "direction": 1 },
      "96": { "encoder": 1, "direction": -1 },
      "97": { "encoder": 1, "direction": 1 },
      "144": { "encoder": 2, "direction": -1 },
      "145": { "encoder": 2, "direction": 1 }
    },
    "encoder_press_map": {
      "37": 0,
      "48": 1,
      "49": 2
    },
    "non_display_buttons": []
  },
  "quirks": {
    "needs_button_remapping": true
  }
}
```

---

## Adding New Devices

To add support for a new device:

1. **Create a new JSON file** in the `devices/` directory
2. **Name it** using the `Manufacturer-Model.json` convention
3. **Fill in all required fields** based on device specifications
4. **Test the device** to verify correct operation
5. **No code changes needed!** The registry automatically loads new devices

## Validation

The registry validates:
- ✅ JSON syntax correctness
- ✅ VID/PID hex format (`0xXXXX`)
- ✅ Required fields present
- ✅ No duplicate VID/PID combinations
- ✅ Enum values (mode, rotation, mirror) are valid
- ✅ Array lengths match constraints

**Note:** The registry does NOT validate:
- Button remap array correctness
- Encoder map byte values
- Image sizes (devices accept various sizes)

Testing with actual hardware is required to verify correct operation.

---

## Design Principles

1. **All quirks in JSON, not code** - Device-specific behavior is data-driven
2. **Protocol-only core** - Code implements protocol, JSON defines devices
3. **Zero hardcoded logic** - Adding devices requires no code changes
4. **Single source of truth** - JSON drives all device decisions
5. **Self-documenting** - JSON format is the specification

---

## Loading Device Definitions

The `mirajazz-json` library is **fully parametric** - it does not hardcode any paths. Applications using this library must provide their own search paths.

### Example Usage

```rust
use mirajazz_json::DeviceRegistry;
use std::path::PathBuf;

// Define your application's search paths
let search_paths = vec![
    PathBuf::from("/usr/share/myapp/devices"),       // System-wide
    dirs::home_dir()?.join(".config/myapp/devices"), // User config
    PathBuf::from("./devices"),                      // Local development
];

// Load from multiple paths - later paths override earlier ones
let registry = DeviceRegistry::load_from_paths(&search_paths)?;
```

This design allows:
- **Reusability**: Any application can use the library with its own paths
- **Flexibility**: Applications control path priority and search order
- **No hardcoded assumptions**: Library doesn't assume "keydeck" or any specific app name
- **Testing**: Easy to provide custom paths for development/testing
