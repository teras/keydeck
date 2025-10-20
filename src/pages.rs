use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyDeckConf {
    /// Optional directory for storing images referenced in button configurations. Otherwise, images are expected to be in the current working directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_dir: Option<String>,

    /// Map of template layouts, where each template can define a reusable page layout.
    #[serde(skip_serializing_if = "Option::is_none")]
    templates: Option<HashMap<String, Page>>,

    /// Map of predefined button configurations, accessible by name for reusability across pages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<HashMap<String, Button>>,

    /// Map of color settings, allowing configuration of colors (e.g., background) by name.
    /// The color format is either "0xRRGGBB" or "0xAARRGGBB".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<HashMap<String, String>>,

    /// Map of services with external commands that can be executed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<HashMap<String, Service>>,

    /// Map of macros, which are reusable action sequences with optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macros: Option<HashMap<String, Macro>>,

    /// A collection of pages, each group identified by the device serial number. When a
    /// device is connected, the corresponding page group is loaded.
    /// When no specific page group is found, the "default" page group is used.
    #[serde(flatten)]
    pub page_groups: HashMap<String, Pages>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    /// Command to be executed for the service.
    pub exec: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    /// Optional window class for associating the page layout with specific applications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_class: Option<String>,

    /// Optional window title pattern for focusing on specific windows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,

    /// Locking page. If true the page cannot be automatically changed when focus changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<bool>,

    /// List of templates to apply to this page layout. All extra configurations (including redefined buttons) are merged with the template.
    #[serde(skip_serializing_if = "Option::is_none")]
    templates: Option<Vec<String>>,

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

    /// Text configuration for the button label or caption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextConfig>,

    /// Outline color (in the format "0xRRGGBB") for text rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<String>,

    /// Text color (in the format "0xRRGGBB") for text rendering. Defaults to white (0xFFFFFF).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,

    /// List of actions that will be executed when the button is pressed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
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
        fontsize: Option<f32>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum WaitForConfig {
    /// Simple string: just the event type
    Simple(String),

    /// Detailed config with timeout
    Detailed {
        event: String,
        #[serde(default = "default_waitfor_timeout")]
        timeout: f64,
    },
}

impl WaitForConfig {
    pub fn event(&self) -> &str {
        match self {
            WaitForConfig::Simple(e) => e,
            WaitForConfig::Detailed { event, .. } => event,
        }
    }

    pub fn timeout(&self) -> f64 {
        match self {
            WaitForConfig::Simple(_) => default_waitfor_timeout(),
            WaitForConfig::Detailed { timeout, .. } => *timeout,
        }
    }
}

fn default_waitfor_timeout() -> f64 {
    1.0
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum Action {
    /// Executes an external command.
    /// By default, spawns the command asynchronously (fire-and-forget).
    /// Set `wait: true` to wait for the command to complete and check its exit status.
    /// When `wait: true`, returns error if command fails (exit code != 0), allowing use with try/else.
    Exec {
        exec: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        wait: Option<bool>,
    },

    /// Jumps to a specified page.
    Jump { jump: String },

    /// Automatically returns to the predefined page, based on the focus change policy.
    AutoJump { autojump: () },

    /// Focuses on an application specified by window class.
    /// Simple string action that attempts to focus the window.
    /// Returns error if focus operation fails (can be caught with try/else).
    Focus { focus: String },

    /// Waits for a specific event to occur, with a timeout.
    /// If the event doesn't occur within the timeout, returns an error.
    /// Can be caught with try/else for error handling.
    WaitFor {
        #[serde(rename = "waitFor")]
        wait_for: WaitForConfig,
    },

    /// Sends a keyboard shortcut event. Some examples include "LCtrl+LShift+z" or "F12".
    /// The value is case-insensitive and can be a single character or a key name.
    Key { key: String },

    /// Sends a string of ASCII characters as individual keystrokes.
    /// Each character in the string is sent as a separate key press/release event.
    /// Supports escape sequences: \n (Enter), \t (Tab), \r (Enter), \\ (backslash), \e (Escape)
    Text { text: String },

    /// Waits for a specified time in seconds before executing the next action.
    Wait { wait: f32 },

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

    /// Calls a macro with optional parameters.
    /// Parameters are substituted in the macro's actions before execution.
    Macro(MacroCall),

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
}

impl KeyDeckConf {
    pub fn new() -> Self {
        let mut path = PathBuf::from(std::env::var("HOME").expect("Could not find home directory"));
        path.push(".config/keydeck.yaml");

        let data = fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Error: Failed to read config file at {}", path.display());
            eprintln!("Reason: {}", e);
            eprintln!("\nPlease create a config file at ~/.config/keydeck.yaml");
            eprintln!("See the documentation for configuration format.");
            std::process::exit(1);
        });

        let mut conf: KeyDeckConf = serde_yaml_ng::from_str(&data).unwrap_or_else(|e| {
            eprintln!("Error: Failed to parse config file at {}", path.display());
            eprintln!("Reason: {}", e);
            eprintln!("\nPlease check your YAML syntax.");
            std::process::exit(1);
        });

        for (_, pages) in &mut conf.page_groups {
            for (_, page) in &mut pages.pages {
                // Safely iterate over templates if it exists
                for template_name in page.templates.as_ref().unwrap_or(&Vec::new()) {
                    if let Some(template_page) = conf.templates.as_ref().and_then(|templates| templates.get(template_name)) {
                        for (button_name, template_button) in &template_page.buttons {
                            // Copy button only if it doesn't exist in the page
                            page.buttons
                                .entry(button_name.clone())
                                .or_insert_with(|| template_button.clone());
                        }
                    }
                }
            }
        }
        conf
    }
}
