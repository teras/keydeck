# KeyDeck YAML Configuration Guide

This document provides a comprehensive guide to the YAML configuration format used by the `keydeck` application. The configuration file, located at `~/.config/keydeck/keydeck.yaml`, allows you to customize the setup for StreamDeck devices and similar hardware.

Each device is identified by its serial number or can use a generic `default` configuration.

## Table of Contents

- [Overview](#overview)
- [Configuration Structure](#configuration-structure)
- [Detailed Configuration](#detailed-configuration)
  - [Global Fields](#global-fields)
  - [Device-Specific Configuration](#device-specific-configuration)
  - [Page Configuration](#page-configuration)
  - [Button Structure](#button-structure)
  - [Templates](#templates)
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

## Detailed Configuration

### Global Fields

Global fields are configurations that apply universally across devices. Available options include:

- `image_dir`: *(optional)* A string path to the directory containing button images. If unspecified, KeyDeck uses the current working directory.
- `colors`: A dictionary of named colors, specified in hexadecimal format (`0xRRGGBB` or `0xAARRGGBB`).

#### Example

```
image_dir: /home/teras/Works/System/Drivers/StreamDeck
colors:
  background: 0x40FFFFFF
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

- **templates**: *(optional)* A list of templates to apply to the page layout. Each template has the same format as a page, with its buttons merged into the current page. Buttons defined in the page override those in the template.

- **on_tick**: *(optional)* A list of actions to execute on each tick event (fires every 1 second). Useful for periodic updates, status checks, or time-based automations. See [Available Actions for Buttons](#available-actions-for-buttons) for supported action types.

- **window_class**: *(optional)* Specifies a window class that, when matched, automatically activates the page. This is useful for associating a page layout with a particular application.

- **window_title**: *(optional)* A pattern used to match window titles, enabling the page to be displayed only when certain windows are active.

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

In this example, the `on_tick` handler executes every 1 second while the page is active, showing a notification. This is useful for:
- Updating time-sensitive button displays
- Polling system status
- Periodic health checks
- Automated tasks that run on a schedule

#### Button Structure

Each button is defined as "button#", where `#` is the button index inside the page. The button can either be based on a template or have its own configuration.

When it is based on a template, the name of the button template is used as a parameter. Otherwise, the button configuration is defined directly with the following fields:

- **icon**: *(optional)* Specifies the path to an image file for the button. This icon will be displayed on the button. If `image_dir` is specified in the global configuration, icons are looked up relative to this directory.
- **background**: *(optional)* Background color for the button, in hexadecimal format or referencing a named color.
- **actions**: *(optional)* List of actions to execute when the button is pressed. Actions execute in sequence.

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
  - **page**: Waits for any page change
  - **tick**: Waits for timer tick (every 1 second)
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
| `page` | Page changes on the device | `set_page()` via central loop |
| `tick` | Timer fires (every 1 second) | Central event loop in `server.rs` |
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
    window_class: kitty
# ...

8B840A19374D:
  restore_mode: last
  Main:
    templates:
      - home_layout
  Firefox:
    button4:
      icon: firefox.png
      background: background
    window_class: firefox
```


### Templates

Templates are reusable layouts applied to pages. Templates include button configurations that can be applied in the `templates` field of a page.

#### Template Structure

The structure resembles a page configuration, with each template containing buttons and their actions.

- `button#`: Each button configuration, similar to page buttons.

#### Example

```
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

