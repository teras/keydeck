use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
    /// Optional directory for storing images referenced in button configurations. Otherwise, images are expected to be in the current working directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_dir: Option<String>,

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

    /// A collection of pages, each group identified by the device serial number. When a
    /// device is connected, the corresponding page group is loaded.
    /// When no specific page group is found, the "default" page group is used.
    #[serde(flatten)]
    pub page_groups: IndexMap<String, Pages>,
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

    /// Individual pages within the page group, each identified by a title.
    #[serde(flatten)]
    pub pages: IndexMap<String, Page>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum ServiceConfig {
    /// Simple form: just the command string (uses default interval and timeout)
    Simple(String),

    /// Detailed form: command with explicit interval and timeout
    Detailed {
        /// Command to execute via bash
        exec: String,

        /// Update interval in seconds (how often to run the command)
        #[serde(default = "default_service_interval")]
        interval: f64,

        /// Command timeout in seconds (kill if exceeds this)
        #[serde(default = "default_service_timeout")]
        timeout: f64,
    },
}

impl ServiceConfig {
    pub fn exec(&self) -> &str {
        match self {
            ServiceConfig::Simple(cmd) => cmd,
            ServiceConfig::Detailed { exec, .. } => exec,
        }
    }

    pub fn interval(&self) -> f64 {
        match self {
            ServiceConfig::Simple(_) => default_service_interval(),
            ServiceConfig::Detailed { interval, .. } => *interval,
        }
    }

    pub fn timeout(&self) -> f64 {
        match self {
            ServiceConfig::Simple(_) => default_service_timeout(),
            ServiceConfig::Detailed { timeout, .. } => *timeout,
        }
    }
}

fn default_service_interval() -> f64 {
    1.0 // 1 second
}

fn default_service_timeout() -> f64 {
    5.0 // 5 seconds
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
            image_dir: None,
            templates: None,
            buttons: None,
            colors: None,
            services: None,
            macros: None,
            tick_time: default_tick_time(),
            brightness: default_brightness(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Optional window name pattern for auto-switching to this page.
    /// Matches against both window class AND window title (case-insensitive substring match, OR logic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_name: Option<String>,

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

    /// Map of button configurations for this page, referenced by button index in the form
    /// of "button#", where "#" is the button index starting from 1.
    #[serde(flatten)]
    pub buttons: HashMap<String, ButtonConfig>,
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

fn get_default_config_path() -> PathBuf {
    let mut path = PathBuf::from(std::env::var("HOME").expect("Could not find home directory"));
    path.push(".config/keydeck/config.yaml");
    path
}

impl KeyDeckConf {
    /// Recursively resolves a template and all its parent templates, with cycle detection.
    /// Returns merged buttons, on_tick actions, and lock value in parent-first order (grandparent -> parent -> child).
    pub fn resolve_template_recursive(
        template_name: &str,
        templates: &IndexMap<String, Page>,
        visited: &mut Vec<String>,
    ) -> Result<(HashMap<String, ButtonConfig>, Option<Vec<Action>>, Option<bool>), String> {
        // Check for circular inheritance
        if visited.contains(&template_name.to_string()) {
            visited.push(template_name.to_string());
            let cycle_path = visited.join(" → ");
            return Err(format!("Circular template inheritance detected: {}", cycle_path));
        }

        // Get the template
        let template = templates.get(template_name).ok_or_else(|| {
            format!("Template '{}' not found", template_name)
        })?;

        // Mark as visited for cycle detection
        visited.push(template_name.to_string());

        let mut merged_buttons = HashMap::new();
        let mut merged_on_tick: Option<Vec<Action>> = None;
        let mut merged_lock: Option<bool> = None;

        // First, recursively resolve parent templates
        if let Some(parent_templates) = &template.inherits {
            for parent_name in parent_templates {
                let (parent_buttons, parent_on_tick, parent_lock) = Self::resolve_template_recursive(
                    parent_name,
                    templates,
                    visited,
                )?;
                // Merge parent buttons (later parents override earlier ones)
                merged_buttons.extend(parent_buttons);
                // on_tick is overridden by later parents (not merged)
                if parent_on_tick.is_some() {
                    merged_on_tick = parent_on_tick;
                }
                // lock is overridden by later parents (not merged)
                if parent_lock.is_some() {
                    merged_lock = parent_lock;
                }
            }
        }

        // Then merge this template's buttons (overriding parent buttons)
        merged_buttons.extend(template.buttons.clone());

        // on_tick is overridden by child (not merged)
        if template.on_tick.is_some() {
            merged_on_tick = template.on_tick.clone();
        }

        // lock is overridden by child (not merged)
        if template.lock.is_some() {
            merged_lock = template.lock;
        }

        // Remove from visited (backtrack for DFS)
        visited.pop();

        Ok((merged_buttons, merged_on_tick, merged_lock))
    }

    /// Load configuration from the default path, or return an empty default config if the file doesn't exist.
    /// This is useful for the configuration UI which should work without an existing config file.
    ///
    /// NOTE: This method is only compiled when building the library (for the UI).
    /// The daemon doesn't need this functionality as it always requires a valid config file.
    /// DO NOT remove this despite dead_code warnings in daemon builds - the UI depends on it.
    #[cfg(not(feature = "binary"))]
    pub fn from_file_or_default() -> Self {
        let path = get_default_config_path();
        if path.exists() {
            Self::new()
        } else {
            Self::default()
        }
    }

    pub fn new() -> Self {
        let path = get_default_config_path();

        let data = fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Error: Failed to read config file at {}", path.display());
            eprintln!("Reason: {}", e);
            eprintln!("\nPlease create a config file at ~/.config/keydeck/config.yaml");
            eprintln!("See the documentation for configuration format.");
            std::process::exit(1);
        });

        let deserializer = serde_yaml_ng::Deserializer::from_str(&data);
        let mut conf: KeyDeckConf = serde_path_to_error::deserialize(deserializer).unwrap_or_else(|e| {
            eprintln!("Error parsing config file: {}", path.display());
            eprintln!();
            eprintln!("Path: {}", e.path());
            eprintln!("{}", e.into_inner());
            std::process::exit(1);
        });

        // Set default image_dir if not specified in config
        if conf.image_dir.is_none() {
            if let Ok(home) = std::env::var("HOME") {
                conf.image_dir = Some(format!("{}/.config/keydeck/icons", home));
            }
        }

        // Validate tick_time is within range (1-60 seconds)
        if conf.tick_time < 1.0 || conf.tick_time > 60.0 {
            eprintln!("Error: tick_time must be between 1 and 60 seconds");
            eprintln!("Current value: {}", conf.tick_time);
            eprintln!("\nPlease update your config file at {}", path.display());
            std::process::exit(1);
        }

        // Validate that templates don't have window_name (only valid for pages)
        if let Some(templates) = &conf.templates {
            for (template_name, template) in templates {
                if template.window_name.is_some() {
                    eprintln!("Error: Template '{}' has 'window_name' field", template_name);
                    eprintln!("The 'window_name' field is only valid in pages, not templates.");
                    eprintln!("Templates are never directly displayed, so window matching doesn't apply.");
                    eprintln!("\nPlease remove the 'window_name' field from template '{}'", template_name);
                    eprintln!("Config file: {}", path.display());
                    std::process::exit(1);
                }
            }
        }

        // Resolve template inheritance for all pages
        let empty_templates = IndexMap::new();
        for (_, pages) in &mut conf.page_groups {
            for (page_name, page) in &mut pages.pages {
                // Recursively resolve all inherited templates
                if let Some(template_names) = &page.inherits {
                    let templates_map = conf.templates.as_ref().unwrap_or(&empty_templates);

                    for template_name in template_names {
                        let mut visited = Vec::new();
                        match Self::resolve_template_recursive(template_name, templates_map, &mut visited) {
                            Ok((resolved_buttons, resolved_on_tick, resolved_lock)) => {
                                // Merge resolved buttons into page (page buttons take priority)
                                for (button_name, button_config) in resolved_buttons {
                                    page.buttons
                                        .entry(button_name)
                                        .or_insert(button_config);
                                }
                                // Merge on_tick (page's on_tick takes priority over template's)
                                if page.on_tick.is_none() && resolved_on_tick.is_some() {
                                    page.on_tick = resolved_on_tick;
                                }
                                // Merge lock (page's lock takes priority over template's)
                                if page.lock.is_none() && resolved_lock.is_some() {
                                    page.lock = resolved_lock;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error resolving templates for page '{}': {}", page_name, e);
                                eprintln!("\nPlease check your template inheritance configuration.");
                                std::process::exit(1);
                            }
                        }
                    }
                }
            }
        }

        // Compute dynamic flags for all buttons after template resolution
        crate::dynamic_detection::compute_all_dynamic_flags(&mut conf);

        conf
    }
}
