use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pages {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_page: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    pub pages: IndexMap<String, Page>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    #[serde(flatten)]
    pub buttons: IndexMap<String, Option<Button>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activate: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Button {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Text {
    Simple(String),
    Detailed {
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        fontsize: Option<f32>,
    },
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Action {
    Exec { exec: String },
    Jump { jump: String },
    Focus { focus: String },
}

impl Pages {
    pub fn new() -> Self {
        let mut path = PathBuf::from(std::env::var("HOME").expect("Could not find home directory"));
        path.push(".config/streamdeck-cli.yaml");
        let data = fs::read_to_string(path).expect("Failed to read config file ~/.config/streamdeck-cli.yaml");
        serde_yaml_ng::from_str(&data).expect("Failed to parse config file ~/.config/streamdeck-cli.yaml")
    }
}