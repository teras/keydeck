# CRT Wire Protocol Reference

This document describes the HID protocol used by Mirabox StreamDock (and compatible) devices,
reverse-engineered from the StreamDock Device SDK's `libtransport.so` binary and Python/C++ SDK sources.

## Overview

Communication happens over USB HID:

- **Output reports** (host → device): CRT commands and image data
- **Input reports** (device → host): button/encoder/touch events
- **Packet sizes**: `512 + 1` bytes for protocol v0/v1, `1024 + 1` bytes for protocol v2/v3
  - The `+1` is the HID report ID byte prepended to every report
- **Report ID**: `0x00` for most devices, `0x04` for K1Pro. When report ID is non-zero,
  all byte offsets in reads/writes shift by 1.

## CRT Command Structure

All commands share a common header:

```
Byte:    0         1  2  3     4  5      6...         10...
Field:  [report_id] [C  R  T] [0  0]   [CMD_NAME]   [params]
Hex:     0x00/0x04  43 52 54  00 00     variable      variable
```

### Offset rule

**Parameters always start at data offset 10** (byte index 11 counting report_id).
The padding between the command name and parameters varies to maintain this alignment:

| Command length | Padding after name | Examples |
|---------------:|-------------------:|----------|
| 3 bytes | 2 × `0x00` | DIS, HAN, STP, LIG, CLE, BAT, LOG, MOD |
| 4 bytes | 1 × `0x00` | LLUM, LMOD, CPOS |
| 5 bytes | none | LBLIG, COLOR, SETLB, DELED, QUCMD, BGPIC, BGCLE |
| 7 bytes | none | CONNECT |

The entire buffer is zero-padded to `packet_size` (512 or 1024) by the transport layer.

## Command Reference

### Screen control

| CRT Name | Hex Bytes | Purpose | Params (at offset 10+) |
|----------|-----------|---------|------------------------|
| DIS | `44 49 53` | Wake screen from sleep | none |
| HAN | `48 41 4e` | Enter sleep/standby | none |
| STP | `53 54 50` | Flush — apply pending image changes | none |
| LIG | `4c 49 47` | Set screen brightness | `[0x00, 0x00, percent]` (0–100) |

### Button images

| CRT Name | Hex Bytes | Purpose | Params (at offset 10+) |
|----------|-----------|---------|------------------------|
| BAT | `42 41 54` | Button image header | `[uint32_be(data_len), key_index]` |
| CLE | `43 4c 45` | Clear button image | `[0x00, 0x00, 0x00, key]` or `key=0xFF` for all |
| LOG | `4c 4f 47` | Boot logo (persistent to flash) | `[uint32_be(data_len), target]` target=0x01 |

### Background / touchscreen

| CRT Name | Hex Bytes | Purpose | Params (at offset 10+) |
|----------|-----------|---------|------------------------|
| BGPIC | `42 47 50 49 43` | Runtime background overlay | `[u32_be(len), u16_be(x), u16_be(y), u16_be(w), u16_be(h), 0x00, fb_layer]` |
| BGCLE | `42 47 43 4c 45` | Clear background layer | `[position]` — 0x01=keys, 0x02=touchscreen, 0x03=all |

### Device control

| CRT Name | Hex Bytes | Purpose | Params (at offset 10+) |
|----------|-----------|---------|------------------------|
| MOD | `4d 4f 44` | Switch device mode | `[0x00, mode + 0x30]` |
| CONNECT | `43 4f 4e 4e 45 43 54` | Keep-alive heartbeat | none |
| QUCMD | `51 55 43 4d 44` | Device config flags | `[flag1, flag2, ...]` (see below) |

QUCMD config payload (6 bytes, each `0x11`=on, `0xFF`=off, `0x1F`=follow):
`[LedFollowKeyLight, KeyLightOnDisconnect, CheckUsbPower, EnableVibration, ResetUsbReport, EnableBootVideo]`

### RGB LED strip (N4Pro, XL, M18)

| CRT Name | Hex Bytes | Purpose | Params (at offset 10+) |
|----------|-----------|---------|------------------------|
| LBLIG | `4c 42 4c 49 47` | LED strip brightness | `[brightness]` (0–100) |
| SETLB | `53 45 54 4c 42` | Set LED colors | `[R,G,B, R,G,B, ...]` × LED count |
| DELED | `44 45 4c 45 44` | Reset LEDs to default | none |

### Keyboard controls (K1Pro only)

| CRT Name | Hex Bytes | Purpose | Params (at offset 10+) |
|----------|-----------|---------|------------------------|
| LLUM | `4c 4c 55 4d` | Keyboard backlight brightness | `[brightness]` (0–6) |
| LMOD | `4c 4d 4f 44` | Keyboard lighting effect/speed | `[value]` (0–9 effects, 0–7 speed) |
| COLOR | `43 4f 4c 4f 52` | Keyboard RGB color | `[R, G, B]` |
| CPOS | `43 50 4f 53` | Keyboard OS mode | `[mode]` — `0x57`='W' Windows, `0x4d`='M' macOS |

Note: LMOD is used for both `set_keyboard_lighting_effects` and `set_keyboard_lighting_speed`
in the SDK — the same CRT command, different value semantics.

## Image Transfer Protocol

Sending an image to a button is a multi-step process:

1. **Header**: Send CRT `BAT` command with `uint32_be(image_data_length)` and `key_index`
2. **Data**: Chunk the JPEG/PNG data into `packet_size`-byte reports:
   ```
   [report_id] [image_data_chunk... zero-padded to packet_size]
   ```
3. **Flush** (v2+ only): Send CRT `STP` to apply changes

Boot logo (`LOG`) and background image (`BGPIC`) follow the same chunked transfer,
each followed by `STP`.

## Input Report Format

```
Byte:  0           1    2    3              9          10
      [report_id?] [ACK header...]         [event_code] [state]
                    41   43   4b ("ACK")
```

- **Protocol v0**: No ACK prefix, raw event data
- **Protocol v1+**: ACK header bytes `0x41 0x43 0x4b` ("ACK") at the start; event data at offsets 9–10
- **Protocol v3**: `state` byte distinguishes press (`0x01`) from release (`0x02`)
- **K1Pro** (`report_id=0x04`): All offsets shift by +1 (event at 10, state at 11)

### Event code interpretation

The raw `event_code` byte is device-specific. It may represent:
- A **button press/release** (regular display button or non-display button)
- An **encoder twist** (mapped via `encoder_twist_map` in device definition)
- An **encoder press** (mapped via `encoder_press_map`)
- A **swipe** or **toggle** event (mapped via encoder maps)

The device JSON definition's `input_mapping` section routes these raw bytes to semantic events.

## Input Event Hardware Codes

### N4Pro (5548:1008, 5548:1021)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–10 | 0x01–0x0A | Main screen buttons |
| Button | 64–67 | 0x40–0x43 | Secondary screen buttons |
| Knob twist | 160, 161 | 0xA0, 0xA1 | Knob 1 CCW/CW |
| Knob twist | 80, 81 | 0x50, 0x51 | Knob 2 CCW/CW |
| Knob twist | 144, 145 | 0x90, 0x91 | Knob 3 CCW/CW |
| Knob twist | 112, 113 | 0x70, 0x71 | Knob 4 CCW/CW |
| Knob press | 55 | 0x37 | Knob 1 |
| Knob press | 53 | 0x35 | Knob 2 |
| Knob press | 51 | 0x33 | Knob 3 |
| Knob press | 54 | 0x36 | Knob 4 |
| Swipe | 56, 57 | 0x38, 0x39 | Swipe left/right |

### N4 (6602:1001, 6603:1007)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–10 | 0x01–0x0A | Main screen buttons |
| Button | 1–4 (secondary) | 0x01–0x04 | Secondary screen (mapped to logical 11–14) |

Note: The Python SDK class for N4 does not implement knob/encoder handling.
The physical hardware may still have knobs (same PCB as N4Pro without RGB LEDs) —
our JSON definitions include encoder mappings matching the N4Pro pattern.

### XL (5548:1028, 5548:1031)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–32 | 0x01–0x20 | Display buttons |
| Toggle | 33 | 0x21 | Toggle 1 up |
| Toggle | 35 | 0x23 | Toggle 1 down |
| Toggle | 36 | 0x24 | Toggle 2 up |
| Toggle | 38 | 0x26 | Toggle 2 down |

### M18 (6603:1009, 6603:1012)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–15 | 0x01–0x0F | Display buttons |
| Button | 37 | 0x25 | Non-display button 1 |
| Button | 48 | 0x30 | Non-display button 2 |
| Button | 49 | 0x31 | Non-display button 3 |

### K1Pro (6603:1015, 6603:1019)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 5, 3, 1 | 0x05, 0x03, 0x01 | Top row (left to right) |
| Button | 6, 4, 2 | 0x06, 0x04, 0x02 | Bottom row (left to right) |
| Knob twist | 80, 81 | 0x50, 0x51 | Knob 1 CCW/CW |
| Knob twist | 96, 97 | 0x60, 0x61 | Knob 2 CCW/CW |
| Knob twist | 144, 145 | 0x90, 0x91 | Knob 3 CCW/CW |
| Knob press | 37 | 0x25 | Knob 1 |
| Knob press | 48 | 0x30 | Knob 2 |
| Knob press | 49 | 0x31 | Knob 3 |

Note: K1Pro uses HID report ID `0x04` (all other devices use `0x00`).

### N1 (6603:1011, 6603:1000)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–15 | 0x01–0x0F | Display buttons |
| Button | 30, 31 | 0x1E, 0x1F | Secondary screen buttons (untested) |
| Knob twist | 50, 51 | 0x32, 0x33 | Knob 1 CCW/CW |
| Knob press | 35 | 0x23 | Knob 1 |

Note: N1 requires `switchMode(2)` on initialization (`device_mode: 2`). Portrait orientation.

### N3 (6602:1001 / existing devices)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–6 | 0x01–0x06 | Display buttons |
| Button | 37, 48, 49 | 0x25, 0x30, 0x31 | Encoder press buttons |
| Knob twist | 144, 145 | 0x90, 0x91 | Knob 1 CCW/CW |
| Knob twist | 96, 97 | 0x60, 0x61 | Knob 2 CCW/CW |
| Knob twist | 80, 81 | 0x50, 0x51 | Knob 3 CCW/CW |
| Knob press | 51 | 0x33 | Knob 1 |
| Knob press | 52 | 0x34 | Knob 2 |
| Knob press | 53 | 0x35 | Knob 3 |

### 293 / 293V3 / M3 (15-button devices, no encoders)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–15 | 0x01–0x0F | Display buttons |

### 293s / 293sV3 (15 + 3 secondary screen buttons)

| Type | HW Code (dec) | HW Code (hex) | Meaning |
|------|---------------|----------------|---------|
| Button | 1–15 | 0x01–0x0F | Main display buttons |
| Button | 16–18 | 0x10–0x12 | Secondary screen buttons |

## Knob Code Patterns

Knob rotation codes follow a consistent pattern: `base` for CCW, `base + 1` for CW.

| Base (hex) | Used by |
|-----------|---------|
| 0x50 | N4Pro knob 2, K1Pro knob 1, N3 knob 3 |
| 0x60 | K1Pro knob 2, N3 knob 2 |
| 0x70 | N4Pro knob 4 |
| 0x90 | N4Pro knob 3, K1Pro knob 3, N3 knob 1 |
| 0xA0 | N4Pro knob 1 |
| 0x32 | N1 knob 1 |
| 0x21/0x23 | XL toggle 1 (non-standard pattern) |
| 0x24/0x26 | XL toggle 2 (non-standard pattern) |

## Device Configuration Summary

| Device | Keys | Encoders | LEDs | Report ID | Image Size | Rotation | Background |
|--------|------|----------|------|-----------|-----------|----------|------------|
| N4Pro | 2×5 | 4 knobs | 4 | 0x00 | 112×112 | 180° | 800×480 |
| N4 | 2×5 | 4 knobs | — | 0x00 | 112×112 | 180° | 800×480 |
| XL | 4×8 | 2 toggles | 6 | 0x00 | 80×80 | 180° | 1024×600 |
| M18 | 3×5 | — | 24 | 0x00 | 64×64 | 0° | 480×272 |
| K1Pro | 2×3 | 3 knobs | — | 0x04 | 64×64 | 90° | 800×480 |
| N1 | 3×5 | 1 knob | — | 0x00 | 96×96 | 0° | 480×854 |
| N3 | 2×3 | 3 knobs | — | 0x00 | 64×64 | -90° | 320×240 |
| 293 | 3×5 | — | — | 0x00 | 100×100 | 180° | 800×480 |
| 293V3 | 3×5 | — | — | 0x00 | 112×112 | 180° | 800×480 |
| 293s | 3×5+3 | — | — | 0x00 | 85×85 | 90° | 854×480 |
| 293sV3 | 3×5+3 | — | — | 0x00 | 96×96 | 90° | 854×480 |
| M3 | 3×5 | — | — | 0x00 | 64×64 | 0° | 480×272 |

## SDK Sources

Findings were extracted from the [StreamDock Device SDK](https://github.com/AhmedMohamedAbdelHamworksead/StreamDock-Device-SDK):

| Source | Location | Method |
|--------|----------|--------|
| Python SDK device classes | `Python-SDK/src/StreamDock/Devices/*.py` | Direct reading |
| Python transport wrapper | `Python-SDK/src/StreamDock/Transport/LibUSBHIDAPI.py` | Direct reading |
| C++ SDK device classes | `CPP-SDK/src/HotspotDevice/*/` | Direct reading |
| CRT command templates | `libtransport.so` `.rodata` section | `objdump -s -j .rodata` |
| CRT dispatch logic | `libtransport.so` `.text` section | `objdump -d` + RIP-relative addressing |
| Product IDs | `Python-SDK/src/StreamDock/ProductIDs.py` | Direct reading |
| Feature options | `Python-SDK/src/StreamDock/FeatrueOption.py` | Direct reading |

### Key `.rodata` Offsets in `libtransport.so`

For future reverse engineering reference, CRT command templates are stored as
static byte arrays in `.rodata`. Each template is `CRT\0\0` + command name bytes.
The transport functions copy these templates to a buffer, set parameters at the
appropriate offsets, then call the HID write function.
