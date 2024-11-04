use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
#[serde(untagged, deny_unknown_fields)]
pub enum Action {
    /// Executes an external command.
    Exec { exec: String },

    /// Jumps to a specified page.
    Jump { jump: String },

    /// Automatically returns to the predefined page, based on the focus change policy.
    AutoJump { autojump: () },

    /// Focuses on an application specified by window class.
    Focus { focus: String },

    /// Sends a keyboard shortcut event. Some examples include "LCtrl+LShift+z" or "F12".
    /// The value is case-insensitive and can be a single character or a key name.
    Key { key: String },

    /// Waits for a specified time in seconds before executing the next action.
    Wait { wait: f32 },
}

impl KeyDeckConf {
    pub fn new() -> Self {
        let mut path = PathBuf::from(std::env::var("HOME").expect("Could not find home directory"));
        path.push(".config/keydeck.yaml");
        let data = fs::read_to_string(path).expect("Failed to read config file ~/.config/keydeck.yaml");
        let mut conf: KeyDeckConf = serde_yaml_ng::from_str(&data).expect("Failed to parse config file ~/.config/keydeck.yaml");

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
