use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub inputs: Option<HashMap<String, InputConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InputConfig {
    Select { options: Vec<String>, default: String },
    Text { placeholder: Option<String>, default: Option<String> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub tasks: Vec<Task>,
}
