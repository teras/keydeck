use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyDeckConf {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    templates: Option<HashMap<String, Page>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<HashMap<String, Button>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<HashMap<String, Service>>,
    #[serde(flatten)]
    pub page_groups: HashMap<String, Pages>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pages {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_page: Option<String>,
    #[serde(default = "default_restore_mode")] // Use the default function
    pub restore_mode: FocusChangeRestorePolicy,
    #[serde(flatten)]
    pub pages: IndexMap<String, Page>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub exec: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum FocusChangeRestorePolicy {
    Keep,
    Last,
    Main,
}

// Define a function that returns the default variant
fn default_restore_mode() -> FocusChangeRestorePolicy {
    FocusChangeRestorePolicy::Main // Choose your default variant here
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    templates: Option<Vec<String>>,
    #[serde(flatten)]
    pub buttons: HashMap<String, ButtonConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Button {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ButtonConfig {
    Template(String),
    Detailed(Button),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum TextConfig {
    Simple(String),
    Detailed {
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        fontsize: Option<f32>,
    },
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged, deny_unknown_fields)]
pub enum Action {
    Exec { exec: String },
    Jump { jump: String },
    AutoJump { autojump: () },
    Focus { focus: String },
    Key { key: String },
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
