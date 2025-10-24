# KeyDeck YAML Configuration Guide

This document provides a comprehensive guide to the YAML configuration format used by the `keydeck` application. The configuration file, located at `~/.config/keydeck/keydeck.yaml`, allows you to customize the setup for StreamDeck devices and similar hardware.

Each device is identified by its serial number or can use a generic `default` configuration.

## Table of Contents

- [Overview](#overview)
- [Configuration Structure](#configuration-structure)
- [Runtime Operations](#runtime-operations)
- [Detailed Configuration](#detailed-configuration)
  - [Global Fields](#global-fields)
  - [Device-Specific Configuration](#device-specific-configuration)
  - [Page Configuration](#page-configuration)
  - [Button Structure](#button-structure)
  - [Templates](#templates)
  - [Template Inheritance](#template-inheritance)
  - [Buttons](#buttons)
  - [Macros](#macros)
  - [Logical Operators](#logical-operators)

## Overview

The KeyDeck configuration file defines:

- Global settings and service commands
- Individual device configurations identified by their serial numbers
- Page and button templates, macros, and named colors

## Configuration Structure

The file has five main sections:

1. **Global Fields**: Define general settings, like image paths and colors.
2. **Device Configuration**: Define settings for each device using the device serial number.
3. **Page Templates**: Define reusable button layouts or actions.
4. **Buttons**: Specify reusable buttons with actions.
5. **Macros**: Define reusable action sequences with parameters.

## Runtime Operations

### Configuration Reload (SIGHUP)

KeyDeck supports live configuration reloading without restarting the server. This is useful for testing configuration changes or updating button layouts on the fly.

**To reload the configuration:**

```bash
# Find the keydeck process ID
pgrep keydeck

# Send SIGHUP signal to reload configuration
kill -HUP <pid>

# Or in one command:
pkill -HUP keydeck
```

**Behavior:**
- The configuration file (`~/.config/keydeck/keydeck.yaml`) is re-read from disk
- All devices are reinitialized with the new configuration
- Current page states are reset to the main page (or first page if no main_page is defined)
- Background services are restarted with new settings
- No device reconnection is required

**Use cases:**
- Rapid iteration during configuration development
- Updating button icons or text without downtime
- Modifying macros or templates
- Changing service commands or intervals

**Example workflow:**
```bash
# Edit your configuration
vim ~/.config/keydeck/keydeck.yaml

# Reload without restarting
pkill -HUP keydeck

# Check logs for any configuration errors
journalctl -u keydeck -f
```

### Device Information Query

KeyDeck provides a command-line tool to query detailed information about connected StreamDeck devices. This is useful for writing configurations, debugging hardware issues, or verifying device capabilities.

**Command:**

```bash
keydeck --info
```

**Output:**

Returns YAML-formatted device information for all connected devices:

```yaml
devices:
  - serial: "CL12345678"
    kind: "Stream Deck +"
    buttons:
      count: 8
      layout:
        columns: 4
        rows: 2
        column_major: true  # Button indices increment down columns first
    encoders:
      count: 4
      positions: [0, 1, 2, 3]  # Physical positions
    touchstrip:
      present: true
      length: 400  # Width in pixels
    lcd:
      present: false

  - serial: "AL87654321"
    kind: "Stream Deck XL"
    buttons:
      count: 32
      layout:
        columns: 8
        rows: 4
        column_major: true
    encoders:
      count: 0
    touchstrip:
      present: false
    lcd:
      present: false
```

**Device Information Fields:**

- **serial**: Unique device serial number (used as key in configuration)
- **kind**: Human-readable device model name
- **buttons.count**: Total number of physical buttons
- **buttons.layout**: Grid layout for button arrangement
  - `columns`: Number of button columns
  - `rows`: Number of button rows
  - `column_major`: If true, button indices increment down columns; if false, across rows
- **encoders.count**: Number of rotary encoders (knobs)
- **encoders.positions**: Physical positions of encoders on the device
- **touchstrip.present**: Whether device has an LCD touchstrip
- **touchstrip.length**: Touchstrip width in pixels (if present)
- **lcd.present**: Whether device has additional LCD displays

**Use cases:**
- **Configuration planning**: Determine available buttons before writing configs
- **Button mapping**: Understand button index layout (column-major vs row-major)
- **Hardware verification**: Check if device is detected correctly
- **Multi-device setups**: Get serial numbers for device-specific configurations
- **Feature detection**: Check for encoders, touchstrips, or LCD capabilities

**Example usage:**

```bash
# Get device info and save to file for reference
keydeck --info > my_devices.yaml

# Quick check of connected devices
keydeck --info | grep -E 'serial|kind|count'
```

## Detailed Configuration

### Global Fields

Global fields are configurations that apply universally across devices. Available options include:

- `image_dir`: *(optional)* A string path to the directory containing button images. If unspecified, KeyDeck uses the current working directory.
- `colors`: A dictionary of named colors, specified in hexadecimal format (`0xRRGGBB` or `0xAARRGGBB`).
- `tick_time`: *(optional)* Global tick interval in seconds. Controls how often the tick event fires for all devices. Must be between 1 and 60 seconds. Default: 2 seconds.
- `services`: *(optional)* A dictionary of background services that execute commands periodically and cache their results. Services provide data that can be referenced in button text via `${service:name}` syntax. See [Services](#services) for details.

#### Example

```yaml
image_dir: /home/teras/Works/System/Drivers/StreamDeck
tick_time: 2
colors:
  background: 0x40FFFFFF

services:
  cpu: "top -bn1 | grep 'Cpu' | awk '{print $2}'"
  weather:
    exec: "curl -s wttr.in/?format=%t"
    interval: 600
    timeout: 10
```

### Services

Services are background threads that execute commands periodically and cache their results. They enable dynamic button content that updates in real-time without blocking the main thread.

#### Service Configuration

Services can be defined in two forms:

**Simple Form** (uses defaults):
```yaml
services:
  cpu: "top -bn1 | grep 'Cpu' | awk '{print $2}'"
  memory: "free -m | awk 'NR==2{printf \"%.0f%%\", $3*100/$2}'"
```
- Command executes via bash every 1 second (default interval)
- Timeout of 5 seconds (default)

**Detailed Form** (explicit configuration):
```yaml
services:
  weather:
    exec: "curl -s wttr.in/?format=%t"
    interval: 600  # Update every 10 minutes
    timeout: 10    # Allow 10 seconds for network request
```

#### Service Fields

- `exec`: *(required)* Shell command to execute via bash
- `interval`: *(optional)* Seconds between command executions (default: 1.0)
- `timeout`: *(optional)* Maximum seconds to wait for command (default: 5.0)

#### Service Behavior

- **Lazy startup**: Services start only when first referenced by a button
- **Background execution**: Each service runs in its own thread, never blocking the UI
- **Automatic retry**: If a command fails or times out, service shows "⚠" and retries on next interval
- **Cached results**: Results are cached and instantly available to all buttons

#### Referencing Services

Services are referenced in button text using `${service:name}` syntax:

```yaml
services:
  cpu: "top -bn1 | grep 'Cpu' | awk '{print $2}'"

pages:
  Main:
    on_tick:
      - refresh:  # Update all dynamic buttons every second

    button1:
      dynamic: true
      text: "CPU: ${service:cpu}%"
```

### Device-Specific Configuration

Each device is defined by its serial number. If a serial number is not found, the configuration falls back to `default`. Device-specific configurations allow you to specify pages and layouts for each connected StreamDeck device.

#### Structure

- Each device has special device field parameters.
- Pages can be defined within the device section.
- Each page can include button configurations.
- Application-specific pages can be activated based on window focus.

#### Device Fields
- **main_page**: *(optional)* Specifies the initial page to load for the device. If not defined, the first page listed will be used.
- **restore_mode**: *(optional)* Defines behavior for page changes on focus:
  - `keep`: Retains the current page on focus change.
  - `last`: Returns to the last viewed page.
  - `main`: Defaults to the main page.

#### Page Configuration

Pages are sections within each device configuration that define sets of buttons and their behavior for different contexts, applications, or layouts. Each page represents a layout displayed on the StreamDeck and can be customized with button mappings, background colors, templates, and associated window classes.

A device can have multiple pages, allowing dynamic switching based on application focus or specific user-defined actions. Each one has the following fields:

- **buttons**: A map of button configurations specific to the page.

- **inherits**: *(optional)* A list of templates this page inherits from. Templates can also inherit from other templates, enabling multi-level inheritance (e.g., page → layout → common_buttons). Buttons are merged in parent-first order, with child buttons overriding parent buttons. See [Template Inheritance](#template-inheritance) for details.

- **on_tick**: *(optional)* A list of actions to execute on each tick event (fires at the interval specified by the global `tick_time` setting, default 2 seconds). Useful for periodic updates, status checks, or time-based automations. Inherited from templates if not defined in the page. **Note:** If the page defines its own `on_tick`, it completely overrides (replaces) the inherited `on_tick` - actions are not merged. See [Available Actions for Buttons](#available-actions-for-buttons) for supported action types.

- **window_name**: *(optional)* Specifies a window name pattern that, when matched, automatically activates the page. Matches against both window class AND window title using case-insensitive substring matching with OR logic. This is useful for associating a page layout with a particular application.

- **lock**: *(optional)* A boolean value that, if `true`, prevents the page from automatically switching when focus changes. This is useful for pages that you want to remain active regardless of window focus changes (e.g., a numpad page). Note: locked pages can still be exited via manual actions like `jump` or `auto_jump`.

##### Example: Page with Tick Handler

```yaml
default:
  pages:
    Main:
      on_tick:
        - exec: "notify-send 'Tick' 'One second passed'"
      button1:
        text: "Hello"
        actions:
          - exec: "echo 'Button pressed'"
```

In this example, the `on_tick` handler executes at the configured tick interval (default 2 seconds) while the page is active, showing a notification. This is useful for:
- Updating time-sensitive button displays
- Polling system status
- Periodic health checks
- Automated tasks that run on a schedule

#### Button Structure

Each button is defined as "button#", where `#` is the button index inside the page. The button can either be based on a template or have its own configuration.

When it is based on a template, the name of the button template is used as a parameter. Otherwise, the button configuration is defined directly with the following fields:

- **icon**: *(optional)* Specifies the path to an image file for the button. This icon will be displayed on the button. If `image_dir` is specified in the global configuration, icons are looked up relative to this directory.
- **background**: *(optional)* Background color for the button, in hexadecimal format or referencing a named color.
- **draw**: *(optional)* Graphics configuration for rendering dynamic visualizations (single bars, gauges, multiple bars). Drawn after icon/background, before text. See [Graphics Rendering](#graphics-rendering).
- **text**: *(optional)* Text to display on the button. Supports dynamic parameters (see [Dynamic Parameters](#dynamic-parameters)).
- **dynamic**: *(optional)* Boolean flag to override automatic dynamic detection. When `true`, the button is always included in `refresh:` actions. When `false`, the button is excluded even if it contains dynamic parameters. When omitted (recommended), automatic detection is used based on the presence of `${provider:arg}` patterns in the button's properties. See [Automatic Dynamic Detection](#automatic-dynamic-detection) for details.
- **actions**: *(optional)* List of actions to execute when the button is pressed. Actions execute in sequence.

**Rendering Order**: When multiple visual elements are specified, they are layered in this order:
1. Background color (if specified)
2. Icon image (if specified)
3. Graphics (if `draw` is specified)
4. Text (if specified)

##### Available Actions for Buttons

Buttons support multiple actions, executed in sequence:
- **Exec**: Executes an external command. Useful for launching applications or running scripts.
  - By default, commands run asynchronously (fire-and-forget) and only fail if the command cannot be started.
  - Set `wait: true` to wait for the command to complete and check its exit status. When `wait: true`, the action fails if the command exits with non-zero status, making it compatible with `try`/`else` error handling.
  - **Example (async)**: `- exec: "firefox"`
  - **Example (sync with wait)**:
    ```yaml
    - exec: "test -f /tmp/myfile"
      wait: true
    ```
- **Jump**: Navigates to a specified page.
  - **Example**: `- jump: "Welcome"`
- **AutoJump**: Re-evaluates the current window focus and switches to the appropriate page for that application. This action bypasses page locks, making it useful as an "escape" button from locked pages.
  - **Use case**: On a locked page (like a numpad), add an auto_jump button to return to the context-appropriate page based on the currently focused window.
  - **Behavior**: Retrieves the current window class and title, then triggers the focus change logic with `force_change=true`, bypassing any lock on the current page.
  - **Example**: `- auto_jump:` (empty value)
  - **Example usage**:
    ```yaml
    pages:
      Numpad:
        lock: true  # Page won't auto-switch on focus changes
        button17:
          icon: back.png
          actions:
            - auto_jump:  # Manual "return to context" button
    ```
- **Focus**: Brings a specified application window to focus by its window class or title. Returns an error if the focus operation fails, which can be caught with try/else.
  - **Example**: `- focus: "firefox"`
  - **Example with guaranteed focus**:
    ```yaml
    - try:
        - focus: ferdium
      else:
        - wait_for: focus
        - focus: ferdium
    ```
- **Key**: Sends a keyboard shortcut or keypress.
  - **Format**: `"Ctrl+Shift+T"` for combinations or `"F12"` for function keys.
  - **Example**: `- key: "LCtrl+LShift+z"`
- **Text**: Types a string of text as individual keystrokes. Automatically handles Shift modifier for uppercase letters and special characters.
  - **Supported characters**: a-z, A-Z, 0-9, space, and common symbols (!, @, #, $, %, ^, &, *, etc.)
  - **Escape sequences**: `\n` (Enter), `\t` (Tab), `\r` (Enter), `\\` (backslash), `\e` (Escape)
  - **Example**: `- text: "Hello World!"` (automatically presses Shift for uppercase H and W, and for !)
  - **Example**: `- text: "user@example.com"` (automatically presses Shift for @)
  - **Example**: `- text: "Line 1\nLine 2"` (types two lines with Enter in between)
- **WaitFor**: Waits for a specific event type to occur before continuing. If the event doesn't occur within the timeout, returns an error (can be caught with try/else).

  This action pauses execution until the specified event type occurs in the system. The action queue is suspended and resumed automatically when any event of that type arrives.

  **Syntax:**
  - Simple form: `wait_for: focus` (waits for any focus event with default 1.0s timeout)
  - With timeout: `wait_for: focus` with `timeout: 2.0` (compact syntax, like exec/macro)

  **Supported Event Types:**
  - **focus**: Waits for any window focus change
  - **tick**: Waits for timer tick (fires at the global `tick_time` interval, default 2 seconds)
  - **sleep**: Waits for system sleep/wake event
  - **newdevice**: Waits for a new device to connect
  - **removeddevice**: Waits for a device to disconnect

  **Examples:**

  Simple wait for focus change (1 second timeout):
  ```yaml
  - wait_for: focus
  ```

  Wait for focus change with custom timeout:
  ```yaml
  - wait_for: focus
    timeout: 2.0
  ```

  **Guaranteed focus pattern**:
  ```yaml
  - try:
      - focus: ferdium
    else:
      - wait_for: focus
      - focus: ferdium
  ```

  **Behavior:**
  - Pauses action queue until the specified event type occurs
  - Resumes automatically when event arrives
  - Returns error on timeout (can be caught with try/else)
- **Wait**: Schedules a non-blocking pause for a specified duration (in seconds). The device remains responsive and other buttons can be pressed during the wait.
  - **Example**: `- wait: 0.5` (waits half a second, non-blocking)
- **Macro**: Calls a reusable macro with optional parameters. Parameters are substituted using `${param}` syntax.
  - **Example (simple)**: `- macro: my_macro_name`
  - **Example (with params)**:
    ```yaml
    - macro: focus_app
      app: firefox
      key: F5
    ```
- **Return**: Stops execution of the current action sequence successfully. Remaining actions are not executed, but no error is raised.
  - **Example**: `- return:`
- **Fail**: Stops execution of the current action sequence with an error. This triggers error handling in try/else blocks.
  - **Example**: `- fail:`
- **And**: Executes multiple actions sequentially. Returns success only if ALL actions succeed. Short-circuits on first failure.
  - **Example**:
    ```yaml
    - and:
        - focus: firefox
        - focus: thunderbird
    ```
- **Or**: Executes actions sequentially until one succeeds. Returns success on FIRST successful action. Returns failure if all actions fail.
  - **Example**:
    ```yaml
    - or:
        - focus: firefox
        - focus: chrome
        - focus: edge
    ```
- **Not**: Inverts the success/failure of a single action. Returns success if the action fails, failure if it succeeds.
  - **Example**:
    ```yaml
    - not:
        focus: unwanted_app
    ```
- **Refresh**: Updates button visual content by re-rendering buttons. Useful for dynamic buttons that display changing information (time, system stats, etc.).
  - **No parameter**: Refreshes all buttons marked with `dynamic: true` on the current page
  - **Single button**: `- refresh: 5` (refreshes button 5)
  - **Multiple buttons**: `- refresh: [1, 3, 7]` (refreshes buttons 1, 3, and 7)
  - Returns error if button number is invalid or button doesn't exist in configuration
  - **Example (auto-update)**:
    ```yaml
    pages:
      Dashboard:
        on_tick:
          - refresh:  # Auto-refresh all dynamic buttons every second
        button1:
          dynamic: true
          text: "${time:%H:%M:%S}"
    ```
  - **Example (manual refresh)**:
    ```yaml
    button10:
      actions:
        - exec: "update-data.sh"
          wait: true
        - refresh: [1, 2, 3]  # Refresh specific buttons after update
    ```

### Logical Operators

The `and`, `or`, and `not` actions provide boolean logic for creating complex conditions. They return success (`Ok`) or failure (`Err`) and are typically used within `try/else` blocks.

#### How They Work

- **`and:`** - ALL actions must succeed (short-circuits on first failure)
- **`or:`** - At least ONE action must succeed (short-circuits on first success)
- **`not:`** - Inverts the result of a single action (success↔failure)

All three can be nested and composed to create arbitrarily complex logic.

#### Sequential AND (Default Behavior)

Actions in a sequence implicitly use AND logic:

```yaml
actions:
  - focus: firefox    # Must succeed
  - key: F5           # Must succeed
  - text: "Done"      # Must succeed
# All three must succeed for sequence to complete
```

The explicit `and:` action is only needed when nesting conditions inside `or:` or `not:`.

#### Example 1: Browser Fallback (OR)

Try multiple browsers until one is found:

```yaml
buttons:
  button1:
    actions:
      - try:
          - or:
              - focus: firefox
              - focus: chrome
              - focus: edge
          - key: "Ctrl+T"  # Open new tab in whichever succeeded
        else:
          - text: "No browser running!"
```

#### Example 2: Guard Condition (NOT)

Ensure an app is NOT running before launching:

```yaml
buttons:
  button1:
    actions:
      - try:
          - not:
              focus: my_app
          - exec: "my_app"  # Launch only if not already running
        else:
          - focus: my_app   # Already running, just focus it
```

#### Example 2.5: Using exec with wait for Conditional Logic

Check if a file exists before processing:

```yaml
buttons:
  button1:
    actions:
      - try:
          - exec: "test -f /tmp/data.json"
            wait: true  # Wait for command to complete and check exit code
          - exec: "process-data.sh /tmp/data.json"
          - text: "Data processed!"
        else:
          - text: "No data file found"
```

Run a script and handle errors:

```yaml
buttons:
  button2:
    actions:
      - try:
          - exec: "my-backup-script.sh"
            wait: true  # Will fail if script returns non-zero exit code
          - text: "Backup complete!"
        else:
          - text: "Backup failed! Check logs"
```

#### Example 3: Complex Nested Logic

Express "(Firefox AND Thunderbird) OR Chrome":

```yaml
buttons:
  button1:
    actions:
      - try:
          - or:
              - and:
                  - focus: firefox
                  - focus: thunderbird
              - focus: chrome
          - text: "Condition met!"
        else:
          - text: "Condition failed"
```

#### Example 4: NOT with Composition

Check that NOT (both Firefox AND Chrome are running):

```yaml
buttons:
  button1:
    actions:
      - try:
          - not:
              and:
                - focus: firefox
                - focus: chrome
          - text: "At least one browser NOT running"
        else:
          - text: "Both browsers are running"
```

#### Example 5: Multi-Level OR Fallback

Try multiple strategies in order:

```yaml
macros:
  ensure_browser:
    actions:
      - or:
          - focus: firefox           # Try focusing existing
          - exec: "firefox"          # Try launching
          - text: "Install Firefox"  # Give up, show message

buttons:
  button1:
    actions:
      - macro: ensure_browser
      - key: "Ctrl+T"
```

#### Important Notes

- **Side Effects**: Actions execute and may have side effects even if the overall condition fails. This is expected behavior (same as other programming languages).
- **Short-Circuit Evaluation**:
  - `and:` stops on first failure (remaining actions don't execute)
  - `or:` stops on first success (remaining actions don't execute)
- **Error Messages**: Failed conditions return descriptive error messages that can be caught with `try/else`.
- **Nesting**: Logical operators can be nested arbitrarily deep to express complex conditions.

### Event System

The following events are dispatched by the system and can be waited for using `wait_for`:

| Event Type | Dispatched When | Dispatched From |
|------------|----------------|-----------------|
| `focus` | Window focus changes (class or title) | Central event loop in `server.rs` |
| `tick` | Timer fires (at global `tick_time` interval, default 2s) | Central event loop in `server.rs` |
| `sleep` | System enters/exits sleep mode | Central event loop in `server.rs` |
| `newdevice` | New device connected | Central event loop in `server.rs` |
| `removeddevice` | Device disconnected | Central event loop in `server.rs` |

**Event Behavior:**
- All events are dispatched through the central event loop in `server.rs`
- Event dispatching calls `check_pending_event()` on all devices
- If a device has a pending `wait_for` with matching event type, actions resume
- `wait_for` matches only on event type, not on specific details
- Timeout: if event doesn't occur within timeout period, returns error (catchable with try/else)
- User interactions (button press, encoder twist, touch events) cancel any pending action queue

#### Example

```
12345678:
  main_page: Welcome
  Welcome:
    button1:
      icon: kitty.png
      background: background
      actions:
        - focus: kitty
        - jump: Kitty
    button2:
      icon: thunderbird.png
      actions:
        - focus: thunderbird
        - jump: Thunderbird
    button3: button_from_termpate_1
    button4: button_from_termpate_2
   butt
  Kitty:
    window_name: kitty
# ...

8B840A19374D:
  restore_mode: last
  Main:
    inherits:
      - home_layout
  Firefox:
    button4:
      icon: firefox.png
      background: background
    window_name: firefox
```


### Templates

Templates are reusable layouts applied to pages. Templates include button configurations that can be applied in the `inherits` field of a page.

#### Template Structure

The structure resembles a page configuration, with each template containing buttons and their actions.

- `button#`: Each button configuration, similar to page buttons.
- `inherits`: *(optional)* Templates can inherit from other templates, enabling multi-level inheritance.
- `on_tick`: *(optional)* Tick actions that will be inherited by pages using this template.
- `lock`: *(optional)* Boolean value that will be inherited by pages using this template. If true, pages inheriting this template will be locked (preventing automatic page switching on focus changes) unless the page explicitly overrides this value.

#### Example

```yaml
templates:
  home_layout:
    button1:
      icon: kitty2.png
      actions:
        - focus: kitty
        - jump: Kitty
    button2:
      icon: thunderbird.png
      actions:
        - focus: thunderbird
        - jump: Thunderbird
```

### Template Inheritance

Template inheritance allows templates to inherit from other templates, and pages to inherit from templates. This enables powerful composition patterns where common elements (like status displays) can be shared across all pages.

#### How Inheritance Works

1. **Multi-level inheritance**: Templates can inherit from other templates, forming inheritance chains (e.g., `page → layout → common_buttons`)
2. **Parent-first merge order**: Buttons are merged starting from the most distant ancestor down to the child
3. **Child overrides parent**: Buttons defined in a child override identically-named buttons from parents
4. **on_tick override**: The `on_tick` field is **not merged** - a child's `on_tick` completely replaces the parent's
5. **lock override**: The `lock` field is **not merged** - a child's `lock` completely replaces the parent's. If not specified in the child, the parent's value is inherited

#### Inheritance Chain Example

```yaml
templates:
  # Base template - common right-side monitoring buttons
  common_right_buttons:
    on_tick:
      - refresh:  # Auto-update dynamic buttons
    button6:
      dynamic: true
      text: "${time:%d/%m}\n${time:%H:%M}"
      background: "0x1a1a1a"
      text_color: "0x00ff00"
    button12:
      dynamic: true
      text: "CPU\n${service:cpu}%"
      background: "0x1a1a1a"
    button18:
      dynamic: true
      text: "RAM\n${service:memory}%"
      background: "0x1a1a1a"

  # Navigation layout inherits common buttons
  home_layout:
    inherits:
      - common_right_buttons  # Gets buttons 6, 12, 18 and on_tick
    button1:
      icon: kitty.png
      actions: [focus: kitty, jump: Kitty]
    button2:
      icon: firefox.png
      actions: [focus: firefox, jump: Firefox]

pages:
  Main:
    inherits:
      - home_layout  # Gets all of home_layout + common_right_buttons
    # Automatically has buttons 1, 2, 6, 12, 18 and on_tick

  Firefox:
    inherits:
      - home_layout  # Gets all of home_layout + common_right_buttons
    button2:  # Override button2 from home_layout
      icon: firefox_custom.png
      background: "0xFF0000"
    window_name: firefox
```

**Result**: The `Main` page gets buttons 1, 2, 6, 12, 18 and on_tick. The `Firefox` page gets buttons 1, 6, 12, 18, on_tick, and its custom button2.

#### Multiple Inheritance

Pages and templates can inherit from multiple parents:

```yaml
templates:
  common_right_buttons:
    button12: { text: "CPU" }
    button18: { text: "RAM" }

  common_navigation:
    button1: { icon: home.png }
    button2: { icon: back.png }

pages:
  Main:
    inherits:
      - common_navigation  # Applied first
      - common_right_buttons  # Applied second (can override)
    # Gets all buttons from both templates
```

Parents are applied in order - later parents override earlier ones if there are conflicts.

#### Lock Inheritance Example

The `lock` field can be inherited from templates, useful for creating categories of "locked" pages:

```yaml
templates:
  # Base template for utility pages that should stay locked
  utility_page_base:
    lock: true
    button1:
      icon: back.png
      actions:
        - auto_jump:  # Manual "return to context" button

pages:
  # Numpad inherits lock: true from template
  Numpad:
    inherits:
      - utility_page_base
    # Gets lock: true automatically, plus back button
    button2: { text: "1", actions: [text: "1"] }
    button3: { text: "2", actions: [text: "2"] }
    # ... more numpad buttons

  # Calculator also inherits lock: true
  Calculator:
    inherits:
      - utility_page_base
    # Gets lock: true automatically, plus back button
    button2: { text: "+", actions: [text: "+"] }
    button3: { text: "-", actions: [text: "-"] }
    # ... more calculator buttons

  # Normal page that overrides the lock
  SpecialPage:
    inherits:
      - utility_page_base
    lock: false  # Override: this page can auto-switch
    # Gets back button but NOT locked
```

**Result**: Both `Numpad` and `Calculator` pages are locked (won't auto-switch on focus changes), while `SpecialPage` explicitly overrides to unlock itself.

#### Cycle Detection

The system detects circular inheritance and reports an error:

```yaml
templates:
  template_a:
    inherits: [template_b]
  template_b:
    inherits: [template_a]  # Error: Circular inheritance!
```

**Error message**: `Circular template inheritance detected: template_a → template_b → template_a`

#### Override Behavior

- **Buttons**: Child buttons completely replace parent buttons with the same name
- **on_tick**: Child's `on_tick` completely replaces parent's `on_tick` (not merged)
- **lock**: Child's `lock` completely replaces parent's `lock` (not merged). If child doesn't specify lock, parent's value is inherited
- **Other fields** (window_name): Page-specific, not inherited from templates

#### Best Practices

1. **Create a common base template** for elements that appear on all pages (date, CPU, RAM, etc.)
2. **Use inheritance chains** for logical groupings (base → layout → page)
3. **Keep templates focused** - each template should have a single responsibility
4. **Document inheritance chains** in comments to help future maintenance

### Buttons

The `buttons` section defines individual button configurations, which can be reused across device pages. Each button is defined by a unique name.

#### Example

```
buttons:
  num0:
    icon: 0.png
    actions:
    - key: "0"
```

### Macros

The `macros` section defines reusable action sequences with optional parameters. Macros help reduce configuration repetition and enable complex, parameterized behaviors.

#### Macro Structure

Each macro has:
- **params** *(optional)*: A map of parameter names to default values
- **actions** *(required)*: A list of actions to execute when the macro is called

Parameters can be referenced in actions using the `${parameter_name}` syntax.

#### Basic Syntax

```yaml
macros:
  macro_name:
    params:  # Optional default parameter values
      param1: default_value1
      param2: default_value2
    actions:
      - focus: ${param1}
      - wait: 0.1
      - key: ${param2}
```

#### Calling Macros

Macros are called using the `macro` action:

```yaml
# Simple call (no parameters, uses defaults)
- macro: macro_name

# Call with parameters
- macro: macro_name
  param1: custom_value1
  param2: custom_value2
```

#### Parameter Substitution

Parameters are substituted anywhere `${param}` appears in action strings:

```yaml
macros:
  send_greeting:
    params:
      name: World
    actions:
      - text: "Hello ${name}!"
      - exec: "echo 'Greeted ${name}' >> /tmp/log"

buttons:
  button1:
    actions:
      - macro: send_greeting
          name: Alice  # Types "Hello Alice!"
```

#### Example 1: Simple Macro (No Parameters)

```yaml
macros:
  home_button:
    actions:
      - auto_jump:

buttons:
  button15:
    icon: home.png
    actions:
      - macro: home_button
```

#### Example 2: Macro with Parameters and Defaults

```yaml
macros:
  focus_and_refresh:
    params:
      app: firefox
      key: F5
    actions:
      - focus: ${app}
      - wait: 0.1
      - key: ${key}

buttons:
  button1:
    actions:
      - macro: focus_and_refresh
          app: chrome
          key: "Ctrl+r"
```

#### Example 3: Nested Macros

Macros can call other macros:

```yaml
macros:
  safe_focus:
    params:
      app: firefox
    actions:
      - try:
          - focus: ${app}
        else:
          - return:  # Quietly stop if app not found
      - wait: 0.1

  focus_and_act:
    params:
      app: firefox
      action_key: F5
    actions:
      - macro: safe_focus
          app: ${app}
      - key: ${action_key}

buttons:
  button1:
    actions:
      - macro: focus_and_act
          app: thunderbird
          action_key: "Ctrl+Shift+A"
```

#### Example 4: Macro with WaitFor

Macros can include `wait_for` actions and will pause correctly:

```yaml
macros:
  guaranteed_focus:
    params:
      app: firefox
      retry_delay: "0.5"
    actions:
      - try:
          - focus: ${app}
        else:
          - exec: "${app}"  # Launch if not running
          - wait_for: focus
            timeout: 5.0
          - focus: ${app}

buttons:
  button1:
    actions:
      - macro: guaranteed_focus
          app: ferdium
      - key: "Ctrl+1"  # Executes after focus succeeds
```

#### Example 5: Macro in Try/Else with Error Handling

```yaml
macros:
  critical_operation:
    params:
      target: myapp
    actions:
      - focus: ${target}
      - wait: 0.2
      - key: "Ctrl+S"

buttons:
  button1:
    actions:
      - try:
          - macro: critical_operation
              target: vscode
        else:
          - text: "Operation failed!"
```

#### Example 6: Using Return and Fail

The `return` and `fail` actions provide flow control within macros:

```yaml
macros:
  validate_and_act:
    params:
      app: firefox
    actions:
      - try:
          - focus: ${app}
        else:
          - return:  # Stop successfully if app not found
      - key: F5  # Only runs if focus succeeded

  strict_validate:
    params:
      app: firefox
    actions:
      - try:
          - focus: ${app}
        else:
          - fail:  # Propagate error upward
      - key: F5

buttons:
  button1:
    actions:
      - macro: validate_and_act
          app: firefox
      - text: "Done"  # Always executes

  button2:
    actions:
      - try:
          - macro: strict_validate
              app: nonexistent
        else:
          - text: "Failed!"  # Executes if macro fails
```

#### Example 7: Real-World Pattern - Multi-Window Workflow

```yaml
macros:
  switch_ferdium_channel:
    params:
      channel: "1"
      message: ""
    actions:
      - try:
          - focus: ferdium
        else:
          - return:
      - wait: 0.1
      - key: "Ctrl+${channel}"
      - wait: 0.1
      - text: "${message}"

buttons:
  button1:
    icon: slack.png
    actions:
      - macro: switch_ferdium_channel
          channel: "1"
          message: "Hello team!"

  button2:
    icon: discord.png
    actions:
      - macro: switch_ferdium_channel
          channel: "2"
          message: "Status update"
```

#### Important Notes

- **Parameter Scope**: Each macro call has isolated parameter scope. Nested macros receive their own parameter values.
- **Recursive Macros**: Macros can call themselves recursively. Be cautious—infinite recursion will cause a stack overflow.
- **Type Conversion**: Parameter values are strings. When substituted into numeric fields (like `wait`), they're parsed automatically by YAML.
- **Error Handling**: If a macro is not found or parameter substitution fails, an error is raised that can be caught with try/else.
- **Late Binding**: Macros are expanded at button press time, not configuration load time.

## Dynamic Parameters

Dynamic parameters allow button text to display real-time information that updates automatically. Parameters are evaluated when buttons are refreshed and use the `${provider:argument}` syntax.

### Available Providers

#### 1. Time Provider (`${time:FORMAT}`)

Displays current date/time using strftime format strings.

**Examples:**
- `${time:%H:%M:%S}` → "14:32:45" (hours:minutes:seconds)
- `${time:%Y-%m-%d}` → "2025-10-20" (date)
- `${time:%A, %B %d}` → "Monday, October 20"
- `${time:%I:%M %p}` → "02:32 PM" (12-hour format)

Common format codes:
- `%H` - Hour (24-hour, 00-23)
- `%I` - Hour (12-hour, 01-12)
- `%M` - Minute (00-59)
- `%S` - Second (00-59)
- `%p` - AM/PM
- `%Y` - Year (4 digits)
- `%m` - Month (01-12)
- `%d` - Day (01-31)
- `%A` - Weekday name
- `%B` - Month name

#### 2. Environment Variable Provider (`${env:VAR}`)

Reads environment variables from the process.

**Examples:**
- `${env:USER}` → Current username
- `${env:HOME}` → Home directory path
- `${env:PATH}` → System PATH variable

**Error Handling:** If variable doesn't exist, displays "⚠"

#### 3. Service Provider (`${service:NAME}`)

References a background service's cached output. Services must be defined in the global `services` section.

**Example:**
```yaml
services:
  cpu: "top -bn1 | grep 'Cpu' | awk '{print $2}'"
  memory: "free -m | awk 'NR==2{printf \"%.0f%%\", $3*100/$2}'"

pages:
  Main:
    on_tick:
      - refresh:  # Update dynamic buttons every second

    button1:
      dynamic: true
      text: "CPU: ${service:cpu}%"

    button2:
      dynamic: true
      text: "RAM: ${service:memory}"
```

**Error States:**
- "..." - Service starting (first update in progress)
- "⚠" - Service failed or not defined

### Combining Multiple Providers

Multiple parameters can be combined in a single text string:

```yaml
button1:
  dynamic: true
  text: "${time:%H:%M} | ${env:USER} | CPU: ${service:cpu}%"
```

### Complete Example: System Monitor Dashboard

```yaml
services:
  cpu: "top -bn1 | grep 'Cpu' | awk '{print $2}'"
  memory: "free -m | awk 'NR==2{printf \"%.0f%%\", $3*100/$2}'"
  disk: "df -h / | tail -1 | awk '{print $5}'"
  uptime:
    exec: "uptime -p | sed 's/up //'"
    interval: 60

pages:
  Dashboard:
    on_tick:
      - refresh:  # Auto-update all dynamic buttons

    button1:
      dynamic: true
      text: "${time:%H:%M:%S}"
      background: "0x000000"

    button2:
      dynamic: true
      text: "CPU\n${service:cpu}%"
      background: "0x1a1a1a"

    button3:
      dynamic: true
      text: "RAM\n${service:memory}"
      background: "0x1a1a1a"

    button4:
      dynamic: true
      text: "DISK\n${service:disk}"
      background: "0x1a1a1a"

    button5:
      dynamic: true
      text: "Uptime:\n${service:uptime}"
      background: "0x1a1a1a"

    button15:
      text: "Refresh"
      actions:
        - refresh:  # Manual refresh button
```

### Automatic Dynamic Detection

**New in this version**: Buttons are automatically detected as dynamic based on their content. You no longer need to manually add `dynamic: true` in most cases!

#### How It Works

At configuration load time, KeyDeck scans all buttons and automatically detects dynamic parameters:

**Dynamic patterns** (with colon `:`):
- `${time:FORMAT}` - Time provider
- `${env:VAR}` - Environment variable
- `${service:NAME}` - Service provider

**Non-dynamic patterns** (no colon - these are macro parameters):
- `${param}` - Macro parameter placeholder
- `${value}` - Macro parameter placeholder

#### What Gets Scanned

The automatic detection analyzes:
1. **Button text** field (both simple and detailed variants)
2. **Button draw.value** field (graphics data sources)
3. **Button actions** (exec, text, key, focus commands)
4. **Macro calls** - both call-site parameters and macro body content
5. **Nested actions** (try/else, and, or, not blocks)

#### Examples

**Automatic detection** (no `dynamic:` needed):
```yaml
button1:
  text: "CPU: ${service:cpu}%"  # Automatically dynamic ✓

button2:
  draw:
    type: bar
    value: ${service:memory}     # Automatically dynamic ✓
    range: [0, 100]

button3:
  actions:
    - exec: "notify-send 'Time' '${time:%H:%M}'"  # Automatically dynamic ✓
```

**Manual override** (when needed):
```yaml
button4:
  text: "Static Label"
  dynamic: true   # Force dynamic even without ${...} patterns

button5:
  text: "Cached: ${service:cpu}%"
  dynamic: false  # Exclude from auto-refresh despite having ${...}
```

#### Edge Cases and Limitations

**Unsupported patterns** (will cause undefined behavior):
- Macro parameter names containing colons: `${key:value}` as a parameter name
- These are extremely rare and violate naming conventions

**Safe workarounds**:
- If a macro contains dynamic content, calling that macro makes the button dynamic
- Nested macros are fully supported with cycle detection
- Macro parameters can contain dynamic values at call sites

#### Best Practices

1. **Omit `dynamic:` field**: Let automatic detection handle it (works 95% of the time)
2. **Use `dynamic: true`** only when:
   - Button needs refresh but has no `${provider:arg}` patterns
   - You want to force refresh for buttons with computed/complex content
3. **Use `dynamic: false`** only when:
   - Button has `${provider:arg}` but should NOT auto-refresh
   - Content is intentionally static after initial evaluation
4. **Use appropriate intervals**: Set service `interval` based on update frequency needs
   - Fast updates (1s): CPU, memory, time
   - Moderate (30-60s): Disk, network stats
   - Slow (600s+): Weather, external APIs
5. **Handle errors gracefully**: Services show "⚠" on failure and automatically retry
6. **Optimize commands**: Keep service commands fast (<1s) to avoid timeouts
7. **Share services**: Multiple buttons can reference the same service efficiently


## Graphics Rendering

The `draw` configuration enables dynamic graphical visualizations on buttons, such as progress bars, gauges, and multi-bar displays. Graphics are rendered in real-time based on data from services, creating visual dashboards for system monitoring and status indicators.

**Rendering Pipeline**: Graphics are composited in this order:
1. Background color
2. Icon image
3. **Graphics** (from `draw` config)
4. Text overlay

### Basic Syntax

```yaml
button1:
  background: "#000000"    # Optional: button background
  draw:                    # Graphics layer
    type: gauge            # Required: graphic type
    value: ${service:cpu}  # Required: data source
    range: [0, 100]        # Required: [min, max] values
    color: "#00ff00"       # Optional: solid color
  text: "CPU"              # Optional: text overlay
```

### Graphic Types

#### 1. `gauge`
Circular arc gauge (speedometer style), sweeping from bottom-left to bottom-right.

**Use cases**: Percentage indicators, RPM, speed displays

**Example**:
```yaml
button1:
  draw:
    type: gauge
    value: ${service:disk_usage}
    range: [0, 100]
    color: "#00ffff"
```

#### 2. `bar`
Unified progress bar supporting all 4 directions (horizontal and vertical).

**Directions**:
- `left_to_right`: Horizontal fill from left (good for: network speed, download progress)
- `right_to_left`: Horizontal fill from right
- `top_to_bottom`: Vertical fill from top
- `bottom_to_top`: Vertical fill from bottom (default - good for: CPU, memory, volume)

**Use cases**: CPU usage, memory usage, network speed, temperature, volume levels

**Example - Vertical (default)**:
```yaml
button2:
  background: "#1a1a1a"
  draw:
    type: bar
    value: ${service:cpu_usage}
    range: [0, 100]
    color: "#ff6600"
  text: "CPU"
```

**Example - Horizontal**:
```yaml
button3:
  background: "#1a1a1a"
  draw:
    type: bar
    value: ${service:network_speed}
    range: [0, 100]
    direction: left_to_right
    color: "#00ff00"
  text: "Network"
```

#### 3. `multi_bar`
Multiple bars with support for all 4 directions.

**Directions**:
- `bottom_to_top` (default): Vertical bars side-by-side (good for: CPU cores, multi-channel levels)
- `top_to_bottom`: Vertical bars side-by-side, filling from top
- `left_to_right`: Horizontal bars stacked vertically (good for: network interfaces, storage volumes)
- `right_to_left`: Horizontal bars stacked vertically, filling from right

**Service must return space-separated values**: `"45 67 23 89"`

**Example - Vertical bars (default)**:
```yaml
services:
  cpu_cores: "top -bn1 | awk '/Cpu/ {print $2}' | head -4 | tr '\n' ' '"

button3:
  draw:
    type: multi_bar
    value: ${service:cpu_cores}
    range: [0, 100]
    color: "#ff00ff"
    bar_spacing: 2
  text: "Cores"
```

**Example - Horizontal bars**:
```yaml
button4:
  draw:
    type: multi_bar
    value: ${service:network_interfaces}
    range: [0, 100]
    direction: left_to_right
    color: "#00ffff"
    bar_spacing: 2
```

### Configuration Parameters

#### Required Parameters

- **type**: Graphic type (`gauge`, `bar`, `multi_bar`)
- **value**: Data source using `${service:name}` syntax (or static number for testing)
- **range**: Array `[min, max]` defining the value range

#### Optional Parameters

- **color**: Solid color in hex format (`"#RRGGBB"` or `"0xRRGGBB"`). Default: white
- **color_map**: Gradient color map with smooth interpolation (see [Color Gradients](#color-gradients))
- **width**: Graphic width in pixels. Default: button width minus padding
- **height**: Graphic height in pixels. Default: button height minus padding
- **position**: Array `[x, y]` for top-left position. Default: centered
- **padding**: Padding around graphic in pixels. Default: 5
- **direction**: Fill direction for bars (`bottom_to_top`, `top_to_bottom`, `left_to_right`, `right_to_left`)
- **segments**: Number of discrete blocks for segmented display (VU meter style). Default: continuous fill
- **bar_spacing**: Spacing between bars for multi_bar types in pixels. Default: 2

### Color Gradients

Use `color_map` instead of `color` for smooth color transitions based on value percentage.

**Format**: Array of `[threshold, color]` pairs where threshold is 0-100.

**Example - Traffic Light Gradient**:
```yaml
button1:
  draw:
    type: bar
    value: ${service:cpu}
    range: [0, 100]
    color_map:
      - [0, "#00ff00"]      # Green at 0%
      - [50, "#ffff00"]     # Yellow at 50%
      - [80, "#ff6600"]     # Orange at 80%
      - [100, "#ff0000"]    # Red at 100%
    # Colors interpolate smoothly between thresholds
```

### Segments (VU Meter Style)

The `segments` parameter divides graphics into discrete LED-style blocks instead of continuous fill.

**Without segments** (continuous):
```yaml
draw:
  type: bar
  value: ${service:volume}
  range: [0, 100]
  direction: left_to_right
  # Fills smoothly: ████████░░░░░░
```

**With segments** (discrete blocks):
```yaml
draw:
  type: bar
  value: ${service:volume}
  range: [0, 100]
  direction: left_to_right
  segments: 10
  # Fills in blocks: ██ ██ ██ ░░ ░░
```

**VU Meter Example**:
```yaml
button_audio:
  draw:
    type: bar
    value: ${service:audio_level}
    range: [0, 100]
    direction: bottom_to_top
    segments: 12
    color_map:
      - [0, "#00ff00"]     # Green for low
      - [70, "#ffff00"]    # Yellow for medium
      - [90, "#ff0000"]    # Red for peaks
```

### Complete Examples

#### System Monitor Dashboard

```yaml
services:
  cpu: "top -bn1 | grep 'Cpu' | awk '{print $2}' | sed 's/%Cpu(s)://'"
  memory: "free | awk 'NR==2{printf \"%.0f\", $3*100/$2}'"
  disk: "df -h / | tail -1 | awk '{print $5}' | sed 's/%//'"
  network_speed: "cat /sys/class/net/eth0/statistics/rx_bytes"  # Simplified

pages:
  Dashboard:
    on_tick:
      - refresh:  # Update all dynamic buttons every second

    button1:
      dynamic: true
      background: "#1a1a1a"
      draw:
        type: bar
        value: ${service:cpu}
        range: [0, 100]
        color_map:
          - [0, "#00ff00"]
          - [70, "#ffff00"]
          - [90, "#ff0000"]
      text: "CPU\n${service:cpu}%"

    button2:
      dynamic: true
      background: "#1a1a1a"
      draw:
        type: bar
        value: ${service:memory}
        range: [0, 100]
        color: "#00ffff"
      text: "RAM\n${service:memory}%"

    button3:
      dynamic: true
      background: "#1a1a1a"
      draw:
        type: gauge
        value: ${service:disk}
        range: [0, 100]
        color_map:
          - [0, "#00ff00"]
          - [80, "#ff9900"]
          - [95, "#ff0000"]
      text: "Disk\n${service:disk}%"
```

#### Multi-Core CPU Monitor

```yaml
services:
  cpu_cores:
    exec: "top -bn2 -d 0.5 | awk '/Cpu/ {print 100-$8}' | tail -4 | tr '\n' ' '"
    interval: 1
    timeout: 3

pages:
  Main:
    on_tick:
      - refresh:

    button1:
      dynamic: true
      background: "#000000"
      draw:
        type: multi_bar
        value: ${service:cpu_cores}
        range: [0, 100]
        color_map:
          - [0, "#00ff00"]
          - [75, "#ffff00"]
          - [90, "#ff0000"]
        bar_spacing: 2
        segments: 10  # Each core shows as segmented bar
      text: "CPU Cores"
```

#### Audio VU Meters (Stereo)

```yaml
services:
  audio_left: "pactl list sinks | grep 'Volume:' | awk '{print $5}' | sed 's/%//'"
  audio_right: "pactl list sinks | grep 'Volume:' | awk '{print $12}' | sed 's/%//'"

pages:
  Audio:
    button1:
      dynamic: true
      draw:
        type: bar
        value: ${service:audio_left}
        range: [0, 100]
        direction: bottom_to_top
        segments: 15
        color_map:
          - [0, "#00ff00"]
          - [80, "#ffff00"]
          - [95, "#ff0000"]
      text: "L"

    button2:
      dynamic: true
      draw:
        type: bar
        value: ${service:audio_right}
        range: [0, 100]
        direction: bottom_to_top
        segments: 15
        color_map:
          - [0, "#00ff00"]
          - [80, "#ffff00"]
          - [95, "#ff0000"]
      text: "R"
```

### Best Practices

1. **Combine with services**: Graphics work best with background services providing real-time data
2. **Mark as dynamic**: Always use `dynamic: true` for buttons with draw configs that reference services
3. **Use on_tick for updates**: Configure `on_tick: - refresh:` in your page to auto-update graphics
4. **Choose appropriate types**:
   - Gauge: Rotary-style indicators (RPM, speed, percentage)
   - Bar: Linear metrics (percentages, speeds). Use `segments` for segmented VU meter style
   - Multi-bar: Comparing multiple values (CPU cores, network interfaces)
5. **Color coding**: Use `color_map` to indicate states (green=good, yellow=warning, red=critical)
6. **Layering**: Combine graphics with text overlays for labeled displays

### Troubleshooting

**Graphics not showing**:
- Verify service is defined and running
- Check value is numeric (use `echo ${service:name}` for debugging)
- Ensure `dynamic: true` is set
- Confirm `on_tick` includes `refresh:`

**Wrong colors**:
- Verify hex color format: `"#RRGGBB"` or `"0xRRGGBB"`
- Check color_map thresholds are in 0-100 range
- Ensure thresholds are in ascending order

**Multi-bar not displaying all bars**:
- Verify service returns space-separated values
- Check that all values are numeric
- Ensure range accommodates all values
