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

## Overview

The KeyDeck configuration file defines:

- Global settings and service commands
- Individual device configurations identified by their serial numbers
- Page and button templates, and named colors

## Configuration Structure

The file has four main sections:

1. **Global Fields**: Define general settings, like image paths and colors.
2. **Device Configuration**: Define settings for each device using the device serial number.
3. **Page Templates**: Define reusable button layouts or actions.
4. **Buttons**: Specify reusable buttons with actions.

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
- 
- **templates**: *(optional)* A list of templates to apply to the page layout. Each template has the same format as a page, with its buttons merged into the current page. Buttons defined in the page override those in the template. 

- **window_class**: *(optional)* Specifies a window class that, when matched, automatically activates the page. This is useful for associating a page layout with a particular application.

- **window_title**: *(optional)* A pattern used to match window titles, enabling the page to be displayed only when certain windows are active.

- **lock**: *(optional)* A boolean value that, if `true`, prevents the page from automatically switching when focus changes. This is useful for pages that you want to remain active regardless of window focus changes.

#### Button Structure

Each button is defined as "button#", where `#` is the button index inside the page. The button can either be based on a template or have its own configuration.

When it is based on a template, the name of the button template is used as a parameter. Otherwise, the button configuration is defined directly with the following fields:

- **icon**: *(optional)* Specifies the path to an image file for the button. This icon will be displayed on the button. If `image_dir` is specified in the global configuration, icons are looked up relative to this directory.
- **background**: *(optional)* Background color for the button, in hexadecimal format or referencing a named color.
- **actions**: *(optional)* List of actions to execute when the button is pressed. Actions execute in sequence.

##### Available Actions for Buttons

Buttons support multiple actions, executed in sequence:
- **Exec**: Executes an external command. Useful for launching applications or running scripts.
  - **Example**: `- exec: "open /path/to/file"`
- **Jump**: Navigates to a specified page.
  - **Example**: `- jump: "Welcome"`
- **AutoJump**: Automatically returns to the active page based on the focused application.
  - **Example**: `- autojump:` (empty value)
- **Focus**: Brings a specified application window to focus by its window class.
  - **Example**: `- focus: "firefox"`
- **Key**: Sends a keyboard shortcut or keypress.
  - **Format**: `"Ctrl+Shift+T"` for combinations or `"F12"` for function keys.
  - **Example**: `- key: "LCtrl+LShift+z"`
- **Wait**: Pauses the sequence for a specified duration (in seconds).
  - **Example**: `- wait: 0.5` (waits half a second)

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

