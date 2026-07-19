// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Macro {
    /// Optional default parameter values for the macro.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, String>>,

    /// Actions to execute when the macro is called. Stored as raw YAML value
    /// to allow parameter substitution before parsing into Action types.
    pub actions: serde_yaml_ng::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MacroCall {
    /// Name of the macro to call.
    #[serde(rename = "macro")]
    pub name: String,

    /// Parameters to pass to the macro. Merged with macro's default params.
    /// All fields except "macro" are treated as parameters.
    #[serde(flatten)]
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDeckConf {
    /// Map of template layouts, where each template can define a reusable page layout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<IndexMap<String, Page>>,

    /// Map of predefined button configurations, accessible by name for reusability across pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<IndexMap<String, Button>>,

    /// Map of color settings, allowing configuration of colors (e.g., background) by name.
    /// The color format is either "0xRRGGBB" or "0xAARRGGBB".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<IndexMap<String, String>>,

    /// Map of services with external commands that can be executed in background threads.
    /// Services provide cached data that can be referenced in button text via ${service:name}.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<IndexMap<String, ServiceConfig>>,

    /// Map of macros, which are reusable action sequences with optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macros: Option<IndexMap<String, Macro>>,

    /// Global tick interval in seconds (default: 2.0, range: 1-60).
    /// Controls how often the tick event fires globally for all devices.
    #[serde(default = "default_tick_time")]
    pub tick_time: f64,

    /// Global device brightness level (0-100, default: 80).
    #[serde(default = "default_brightness")]
    pub brightness: u8,

    /// Background/wallpaper image path for the device LCD.
    /// Only supported on devices with background image capability (e.g., Ajazz/Mirabox).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_image: Option<String>,

    /// List of glob patterns for icons that should be protected from cleanup.
    /// Icons matching these patterns won't be deleted even if unused.
    /// This is useful for icons used by dynamic content or button state switching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected_icons: Option<Vec<String>>,

    /// Enables the konsole terminal-context resolver (Linux/KDE only). When on, the
    /// daemon asks the focused konsole which program runs in the active tab and
    /// reports it as the `context`/`git` variables, exactly like the kitty
    /// integration. Ignored on non-Linux platforms; kept in the file regardless so
    /// configs stay portable across machines.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub konsole_context: bool,

    /// Programs of interest for the konsole resolver: only these are reported as the
    /// `context` variable (anything else, including the bare shell, reports empty),
    /// mirroring the kitty watcher's APPS list. Defaults to a common set when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub konsole_apps: Option<Vec<String>>,

    /// A collection of pages, each group identified by the device serial number. When a
    /// device is connected, the corresponding page group is loaded.
    /// When no specific page group is found, the "default" page group is used.
    #[serde(flatten)]
    pub page_groups: IndexMap<String, Pages>,
}

impl KeyDeckConf {
    /// Migrates the legacy `window_name` field on every page and template into the
    /// unified [`When`] structure. Applied right after deserialization (daemon loader
    /// and config UI load), so old configs keep auto-switching and get rewritten as
    /// `when` on the next save. Idempotent; a no-op once configs are upgraded.
    pub fn migrate_legacy_window_name(&mut self) {
        fn migrate(page: &mut Page) {
            if let Some(pattern) = page.window_name.take() {
                if page.when.is_none() {
                    page.when = Some(When::window(pattern));
                }
            }
        }
        if let Some(templates) = &mut self.templates {
            for (_, template) in templates.iter_mut() {
                migrate(template);
            }
        }
        for (_, pages) in self.page_groups.iter_mut() {
            for (_, page) in pages.pages.iter_mut() {
                migrate(page);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pages {
    /// Optional main page name; if provided, used as the default page in the group. Defaults
    /// to the first page in the group if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_page: Option<String>,

    /// Policy for restoring focus when switching between pages.
    #[serde(default = "default_restore_mode")] // Uses the default function to set a value
    pub restore_mode: FocusChangeRestorePolicy,

    /// Visual effect applied to button images when pressed.
    /// Only used on devices that support software button press feedback.
    #[serde(default)]
    pub press_effect: PressEffectConfig,

    /// Individual pages within the page group, each identified by a title.
    #[serde(flatten)]
    pub pages: IndexMap<String, Page>,
}

/// Configuration for the visual effect applied to buttons when pressed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PressEffectConfig {
    /// Shrink the content when pressed (Lanczos resize). No canvas size reduction.
    /// The `pixels` value is the margin on each side when shrunk.
    Shrink {
        #[serde(default = "default_shrink_pixels")]
        pixels: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        border_color: Option<String>,
    },
    /// Shift content on press. Canvas is reduced by `pixels` in each dimension.
    /// Unpressed: content at (0,0), border_color fills right+bottom.
    /// Pressed: content at (T,T), border_color fills left+top.
    Shift {
        #[serde(default = "default_shift_pixels")]
        pixels: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        border_color: Option<String>,
    },
    /// 3D bevel border. Canvas is reduced by 2×`pixels` in each dimension.
    /// Unpressed: raised (highlight top+left, shadow bottom+right).
    /// Pressed: sunken (shadow top+left, highlight bottom+right).
    /// Diagonal transition at bottom-left and top-right corners.
    Emboss {
        #[serde(default = "default_emboss_pixels")]
        pixels: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        border_color: Option<String>,
    },
}

impl PressEffectConfig {
    /// Returns (width_reduction, height_reduction) for the render pipeline canvas.
    pub fn canvas_reduction(&self) -> (u32, u32) {
        match self {
            PressEffectConfig::Shrink { .. } => (0, 0),
            PressEffectConfig::Shift { pixels, .. } => (*pixels, *pixels),
            PressEffectConfig::Emboss { pixels, .. } => (3 * *pixels, 3 * *pixels),
        }
    }

    /// Returns the border_color string if set.
    pub fn border_color(&self) -> Option<&str> {
        match self {
            PressEffectConfig::Shrink { border_color, .. }
            | PressEffectConfig::Shift { border_color, .. }
            | PressEffectConfig::Emboss { border_color, .. } => border_color.as_deref(),
        }
    }
}

impl Default for PressEffectConfig {
    fn default() -> Self {
        PressEffectConfig::Shrink {
            pixels: default_shrink_pixels(),
            border_color: None,
        }
    }
}

fn default_shrink_pixels() -> u32 {
    4
}

fn default_shift_pixels() -> u32 {
    4
}

fn default_emboss_pixels() -> u32 {
    2
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    /// Command to execute via bash
    pub exec: String,

    /// Update interval in seconds (how often to run the command)
    #[serde(default = "default_service_interval", skip_serializing_if = "is_default_interval")]
    pub interval: f64,

    /// Optional command timeout in seconds (None = no timeout)
    /// Can be specified as: missing, null, empty, or a number
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_optional_f64")]
    pub timeout: Option<f64>,
}

fn default_service_interval() -> f64 {
    1.0 // 1 second
}

fn is_default_interval(interval: &f64) -> bool {
    *interval == 1.0
}

/// Custom deserializer for optional f64 that treats null, missing, and empty string as None
fn deserialize_optional_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    // Accept either null or a valid f64
    Option::<f64>::deserialize(deserializer)
}

fn default_tick_time() -> f64 {
    2.0 // 2 seconds
}

fn default_brightness() -> u8 {
    80 // 80%
}

impl Default for KeyDeckConf {
    fn default() -> Self {
        KeyDeckConf {
            templates: None,
            buttons: None,
            colors: None,
            services: None,
            macros: None,
            tick_time: default_tick_time(),
            brightness: default_brightness(),
            background_image: None,
            protected_icons: None,
            konsole_context: false,
            konsole_apps: None,
            page_groups: IndexMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum FocusChangeRestorePolicy {
    /// Keeps the current page when changing focus between configurations.
    Keep,

    /// Returns to the last selected page when changing focus.
    Last,

    /// Returns to the main page when changing focus.
    Main,
}

// Default focus change restore policy function.
fn default_restore_mode() -> FocusChangeRestorePolicy {
    FocusChangeRestorePolicy::Main // Default is set to Main
}

/// A single filter value inside a `when` group, or a list of them.
/// A list means OR: the filter matches if ANY listed value matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WhenValue {
    One(String),
    Many(Vec<String>),
}

impl WhenValue {
    /// Returns true if `pred` accepts any of the values (OR semantics).
    pub fn any<F: Fn(&str) -> bool>(&self, pred: F) -> bool {
        match self {
            WhenValue::One(v) => pred(v),
            WhenValue::Many(vs) => vs.iter().any(|v| pred(v)),
        }
    }
}

/// Auto-switch conditions in disjunctive normal form (map = AND, list = OR).
///
/// `groups` is a list of AND-groups joined by OR: a page activates when ANY group
/// matches, and a group matches when ALL its key/value filters match. Each value may
/// itself be a list (OR among values). Reserved keys `window`/`class`/`title` match the
/// focused window (case-insensitive substring); any other key matches an external
/// context variable (exact match, set via `keydeck --set key=value`).
///
/// In YAML this accepts either a single mapping (one group) or a list of mappings
/// (many groups), and is serialized back in the same shape. Values must be strings
/// (quote numbers, e.g. `git: "1"`).
#[derive(Debug, Clone, Default)]
pub struct When {
    pub groups: Vec<IndexMap<String, WhenValue>>,
}

impl When {
    /// Builds a single-group `when` matching a window pattern (legacy `window_name`).
    pub fn window(pattern: String) -> Self {
        let mut group = IndexMap::new();
        group.insert("window".to_string(), WhenValue::One(pattern));
        When {
            groups: vec![group],
        }
    }

    /// Evaluates the condition. `check(key, value)` reports whether a single
    /// key/value filter matches the current state (focus + context variables).
    pub fn matches<F: Fn(&str, &str) -> bool>(&self, check: F) -> bool {
        self.groups.iter().any(|group| {
            group
                .iter()
                .all(|(key, value)| value.any(|v| check(key, v)))
        })
    }
}

impl Serialize for When {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Round-trips in the friendly shape: a lone group as a mapping, else a list.
        if self.groups.len() == 1 {
            self.groups[0].serialize(serializer)
        } else {
            self.groups.serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for When {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Single(IndexMap<String, WhenValue>),
            Multi(Vec<IndexMap<String, WhenValue>>),
        }
        let groups = match Repr::deserialize(deserializer)? {
            Repr::Single(group) => vec![group],
            Repr::Multi(groups) => groups,
        };
        Ok(When { groups })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Legacy per-page window pattern. Superseded by `when`; still accepted on read and
    /// migrated into `when` (see [`KeyDeckConf::migrate_legacy_window_name`]), but never
    /// written back — the config UI upgrades it to `when` on the next save.
    #[serde(default, skip_serializing)]
    pub window_name: Option<String>,

    /// Auto-switch conditions: activate this page when the focused window and/or external
    /// context variables match. See [`When`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<When>,

    /// Locking page. If true the page cannot be automatically changed when focus changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<bool>,

    /// List of templates this page/template inherits from. Buttons are merged in order (parent first, child overrides).
    /// Templates can also inherit from other templates, enabling multi-level inheritance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherits: Option<Vec<String>>,

    /// Actions to execute on each tick event (fires every 1 second).
    /// Useful for periodic updates, status checks, or time-based automations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_tick: Option<Vec<Action>>,

    /// Map of encoder configurations for this page, referenced by encoder index in the form
    /// of "encoder#", where "#" is the encoder index starting from 1.
    /// Encoders support twist (left/right rotation) and press actions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoders: Option<IndexMap<String, Encoder>>,

    /// Map of button configurations for this page, referenced by button index in the form
    /// of "button#", where "#" is the button index starting from 1.
    #[serde(flatten)]
    pub buttons: HashMap<String, ButtonConfig>,
}

/// Configuration for a rotary encoder (knob).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Encoder {
    /// Actions to execute when the encoder is twisted clockwise (right).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twist_right: Option<Vec<Action>>,

    /// Actions to execute when the encoder is twisted counter-clockwise (left).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twist_left: Option<Vec<Action>>,

    /// Actions to execute when the encoder is pressed (pushed down and released).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub press: Option<Vec<Action>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Button {
    /// Icon image filename for the button display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Background color (in the format "0xRRGGBB" or "0xAARRGGBB") for the button display,
    /// or a color reference to a named color in the configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,

    /// Graphics drawing configuration for the button. An array of graphics that will be drawn
    /// in order (first item drawn first, last item on top).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draw: Option<Vec<DrawConfig>>,

    /// Text configuration for the button label or caption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextConfig>,

    /// Outline color (in the format "0xRRGGBB") for text rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<String>,

    /// Text color (in the format "0xRRGGBB") for text rendering. Defaults to white (0xFFFFFF).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,

    /// Whether this button should be refreshed automatically by the `refresh:` action (no parameters).
    /// When true, the button will be included in automatic refresh cycles (e.g., on_tick).
    /// When None, automatic detection is used (see is_dynamic_computed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<bool>,

    /// Computed at configuration load time: whether this button contains dynamic parameters.
    /// Automatically detected by scanning text, draw.value, and actions for ${provider:arg} patterns.
    /// This field is not serialized (not part of YAML config).
    /// Used as fallback when `dynamic` is None.
    #[serde(skip, default)]
    pub is_dynamic_computed: bool,

    /// List of actions that will be executed when the button is pressed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum ButtonConfig {
    /// Reference to a template name to use as the button configuration.
    Template(String),

    /// Detailed configuration for a button, including icon, background, and actions.
    Detailed(Button),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum TextConfig {
    /// Simple text string displayed on the button.
    Simple(String),

    /// Detailed configuration for text, with optional font size.
    Detailed {
        /// Text to display on the button.
        value: String,

        /// Font size for the text, optional.
        #[serde(skip_serializing_if = "Option::is_none")]
        font_size: Option<f32>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct DrawConfig {
    /// Type of graphic to draw
    #[serde(rename = "type")]
    pub graphic_type: GraphicType,

    /// Data source - single ${...} expression that evaluates to number(s)
    /// For multi_bar types, evaluates to space-separated numbers
    pub value: String,

    /// Value range [min, max]
    pub range: [f32; 2],

    /// Single solid color (hex format: "#RRGGBB" or "0xRRGGBB")
    /// Mutually exclusive with color_map
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Color gradient map with smooth interpolation
    /// Format: [[threshold, color], ...] sorted by threshold
    /// Mutually exclusive with color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_map: Option<Vec<ColorMapEntry>>,

    /// Width in pixels (default: button width - 2*padding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    /// Height in pixels (default: button height - 2*padding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    /// Position [x, y] from top-left (default: centered)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<[i32; 2]>,

    /// Padding around graphic (default: 5 pixels)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<u32>,

    /// Direction for level types (default: bottom_to_top for vertical, left_to_right for horizontal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,

    /// Number of segments for bar/level displays (default: continuous fill)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<u32>,

    /// Spacing between bars for multi_bar types (default: 2 pixels)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bar_spacing: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ColorMapEntry {
    /// Array format: [threshold, color]
    Array([serde_yaml_ng::Value; 2]),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GraphicType {
    Gauge,
    Bar,
    MultiBar,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase", untagged)]
pub enum RefreshTarget {
    /// Refresh all dynamic buttons (explicit "dynamic" string)
    Dynamic(String),

    /// Refresh a single button by index
    Single(u8),

    /// Refresh multiple buttons by index
    Multiple(Vec<u8>),
}

fn default_refresh_target() -> RefreshTarget {
    RefreshTarget::Dynamic("dynamic".to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum Action {
    /// Jumps to a specified page.
    Jump { jump: String },

    /// Automatically returns to the predefined page, based on the focus change policy.
    AutoJump { auto_jump: () },

    /// Focuses on an application by matching window class OR title.
    /// The focus string is checked against both window class and title (case-insensitive substring match).
    /// Returns error if no matching window is found (can be caught with try/else).
    Focus { focus: String },

    /// Sends a keyboard shortcut event. Some examples include "LCtrl+LShift+z" or "F12".
    /// The value is case-insensitive and can be a single character or a key name.
    Key { key: String },

    /// Sends a string of ASCII characters as individual keystrokes.
    /// Each character in the string is sent as a separate key press/release event.
    /// Supports escape sequences: \n (Enter), \t (Tab), \r (Enter), \\ (backslash), \e (Escape)
    Text { text: String },

    /// Waits for a specified time in seconds before executing the next action.
    Wait { wait: f32 },

    /// Waits for a specific event to occur, with optional timeout.
    /// If the event doesn't occur within the timeout, returns an error.
    /// Can be caught with try/else for error handling.
    /// Timeout defaults to 1.0 second if not specified.
    WaitFor {
        #[serde(rename = "wait_for")]
        wait_for_event: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<f64>,
    },

    /// Executes an external command.
    /// By default, spawns the command asynchronously (fire-and-forget).
    /// Set `wait: true` to wait for the command to complete and check its exit status.
    /// When `wait: true`, returns error if command fails (exit code != 0), allowing use with try/else.
    Exec {
        exec: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        wait: Option<bool>,
    },

    /// Calls a macro with optional parameters.
    /// Parameters are substituted in the macro's actions before execution.
    Macro(MacroCall),

    /// Try/else block for error handling.
    /// Executes try_actions sequentially, stopping on first error.
    /// If try fails and else_actions is present, executes else block.
    /// If try fails and no else, continues to next action (error swallowed).
    Try {
        #[serde(rename = "try")]
        try_actions: Vec<Action>,

        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "else")]
        else_actions: Option<Vec<Action>>,
    },

    /// Returns successfully from the current action sequence.
    /// Stops execution of remaining actions without triggering error handlers.
    Return {
        #[serde(rename = "return")]
        return_unit: (),
    },

    /// Fails and stops execution of the current action sequence.
    /// Triggers error handling in try/else blocks.
    Fail {
        #[serde(rename = "fail")]
        fail_unit: (),
    },

    /// Executes actions sequentially. Returns Ok if ALL succeed, Err on first failure.
    /// Short-circuits on first error. Useful for expressing complex conditions.
    And {
        #[serde(rename = "and")]
        and_actions: Vec<Action>,
    },

    /// Executes actions sequentially. Returns Ok on FIRST success, Err if all fail.
    /// Short-circuits on first success. Useful for fallback logic.
    Or {
        #[serde(rename = "or")]
        or_actions: Vec<Action>,
    },

    /// Inverts the result of a single action. Returns Ok if action fails, Err if succeeds.
    /// Useful for checking that something is NOT running or available.
    Not {
        #[serde(rename = "not")]
        not_action: Box<Action>,
    },

    /// Refreshes button(s) to update their visual content.
    /// - "dynamic": refreshes all buttons marked with `dynamic: true`
    /// - Single number: refreshes that specific button
    /// - Array of numbers: refreshes those specific buttons
    /// Returns error if button number is invalid or button doesn't exist.
    Refresh {
        #[serde(default = "default_refresh_target")]
        refresh: RefreshTarget,
    },
}

#[cfg(test)]
mod when_tests {
    use super::*;

    fn parse_page(yaml: &str) -> Page {
        serde_yaml_ng::from_str(yaml).expect("page should parse")
    }

    #[test]
    fn single_group_round_trips_as_mapping() {
        let page = parse_page("when: { window: kitty, context: claude }\n");
        let when = page.when.as_ref().unwrap();
        assert_eq!(when.groups.len(), 1);
        let out = serde_yaml_ng::to_string(&page).unwrap();
        // A lone group serializes back as a mapping, not a list.
        assert!(out.contains("when:"));
        assert!(!out.trim_start().starts_with("when:\n-"));
        let reparsed = parse_page(&out);
        assert_eq!(reparsed.when.as_ref().unwrap().groups.len(), 1);
    }

    #[test]
    fn value_list_is_or() {
        let page = parse_page("when: { window: [kitty, konsole] }\n");
        let when = page.when.as_ref().unwrap();
        let val = when.groups[0].get("window").unwrap();
        assert!(val.any(|v| v == "konsole"));
        assert!(!val.any(|v| v == "firefox"));
    }

    #[test]
    fn group_list_is_or_of_ands() {
        let page = parse_page(
            "when:\n  - { window: konsole, context: mc }\n  - { window: kitty, context: claude }\n",
        );
        let when = page.when.as_ref().unwrap();
        assert_eq!(when.groups.len(), 2);
        // Round-trips back to a list of two groups.
        let out = serde_yaml_ng::to_string(&page).unwrap();
        let reparsed = parse_page(&out);
        assert_eq!(reparsed.when.as_ref().unwrap().groups.len(), 2);
    }

    #[test]
    fn matches_dnf_semantics() {
        // (konsole AND mc) OR (kitty AND claude)
        let page = parse_page(
            "when:\n  - { window: konsole, context: mc }\n  - { window: kitty, context: claude }\n",
        );
        let when = page.when.as_ref().unwrap();
        // kitty + claude matches the second group.
        assert!(when.matches(|k, v| match k {
            "window" => v == "kitty",
            _ => v == "claude",
        }));
        // konsole + claude matches neither group.
        assert!(!when.matches(|k, v| match k {
            "window" => v == "konsole",
            _ => v == "claude",
        }));
    }

    #[test]
    fn legacy_window_name_migrates_to_when() {
        let mut conf: KeyDeckConf =
            serde_yaml_ng::from_str("pages:\n  Foo:\n    window_name: firefox\n").unwrap();
        conf.migrate_legacy_window_name();
        let page = &conf.page_groups["pages"].pages["Foo"];
        assert!(page.window_name.is_none());
        let when = page.when.as_ref().unwrap();
        assert_eq!(when.groups.len(), 1);
        assert!(when.groups[0].get("window").unwrap().any(|v| v == "firefox"));
        // Serializes as `when`, never as legacy `window_name`.
        let out = serde_yaml_ng::to_string(&conf).unwrap();
        assert!(out.contains("when:"));
        assert!(!out.contains("window_name"));
    }

    #[test]
    fn explicit_when_wins_over_legacy() {
        let mut page = parse_page("window_name: firefox\nwhen: { window: kitty }\n");
        // migration is a no-op when `when` already present
        if let Some(name) = page.window_name.take() {
            if page.when.is_none() {
                page.when = Some(When::window(name));
            }
        }
        assert!(page.when.unwrap().groups[0]
            .get("window")
            .unwrap()
            .any(|v| v == "kitty"));
    }
}
