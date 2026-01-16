use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub command: String,
    pub category: Option<String>,
    pub cwd: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
    pub env_clear: Option<bool>,
    pub inputs: Option<HashMap<String, InputConfig>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InputConfig {
    Select {
        options: Vec<String>,
        default: String,
    },
    Text {
        placeholder: Option<String>,
        default: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub tasks: Vec<Task>,
    pub history_limit: Option<usize>,
    pub ui: Option<UiConfig>,
    pub keys: Option<KeyBindings>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyBindings {
    #[serde(default)]
    pub global: HashMap<String, String>,     // For future global keys
    #[serde(default)]
    pub task_list: HashMap<String, String>,  // Keys in the list view
    #[serde(default)]
    pub task_running: HashMap<String, String>, // Keys in the running view (command mode)
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut task_list = HashMap::new();
        task_list.insert("quit".to_string(), "q".to_string());
        task_list.insert("up".to_string(), "up".to_string());
        task_list.insert("down".to_string(), "down".to_string());
        task_list.insert("select".to_string(), "enter".to_string());
        task_list.insert("delete_instance".to_string(), "d".to_string());
        task_list.insert("kill_instance".to_string(), "X".to_string());
        task_list.insert("fold_task".to_string(), "tab".to_string());

        let mut task_running = HashMap::new();
        task_running.insert("toggle_command_mode".to_string(), "ctrl+p".to_string());
        task_running.insert("back_to_list".to_string(), "b".to_string()); // Detach
        task_running.insert("quit_task".to_string(), "q".to_string()); // Actually detach/back, original code was 'q' -> back
        task_running.insert("kill_task".to_string(), "k".to_string());

        Self {
            global: HashMap::new(),
            task_list,
            task_running,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub status_bar_fg: Option<String>,
    pub status_bar_bg: Option<String>,
    pub command_mode_fg: Option<String>,
    pub command_mode_bg: Option<String>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            status_bar_fg: Some("white bold".to_string()), 
            status_bar_bg: Some("blue".to_string()),
            command_mode_fg: Some("white bold".to_string()),
            command_mode_bg: Some("red".to_string()),
        }
    }
}

impl UiConfig {
    pub fn parse_style(style_str: &str, is_bg: bool) -> String {
        let mut codes = Vec::new();
        
        for part in style_str.split_whitespace() {
            let part_lower = part.to_lowercase();
            let code = match part_lower.as_str() {
                // Reset
                "reset" | "default" => if is_bg { "49" } else { "39" },
                
                // Styles
                "bold" => "1",
                "dim" => "2",
                "italic" => "3",
                "underline" => "4",
                "blink" => "5",
                "reverse" => "7",
                "hidden" => "8",
                
                // Basic Colors (Foreground)
                "black" if !is_bg => "30",
                "red" if !is_bg => "31",
                "green" if !is_bg => "32",
                "yellow" if !is_bg => "33",
                "blue" if !is_bg => "34",
                "magenta" if !is_bg => "35",
                "cyan" if !is_bg => "36",
                "white" if !is_bg => "37",
                
                // Basic Colors (Background)
                "black" if is_bg => "40",
                "red" if is_bg => "41",
                "green" if is_bg => "42",
                "yellow" if is_bg => "43",
                "blue" if is_bg => "44",
                "magenta" if is_bg => "45",
                "cyan" if is_bg => "46",
                "white" if is_bg => "47",
                
                // Bright Colors (Foreground)
                "light_black" | "gray" | "grey" if !is_bg => "90",
                "light_red" if !is_bg => "91",
                "light_green" if !is_bg => "92",
                "light_yellow" if !is_bg => "93",
                "light_blue" if !is_bg => "94",
                "light_magenta" if !is_bg => "95",
                "light_cyan" if !is_bg => "96",
                "light_white" if !is_bg => "97",
                
                // Bright Colors (Background)
                "light_black" | "gray" | "grey" if is_bg => "100",
                "light_red" if is_bg => "101",
                "light_green" if is_bg => "102",
                "light_yellow" if is_bg => "103",
                "light_blue" if is_bg => "104",
                "light_magenta" if is_bg => "105",
                "light_cyan" if is_bg => "106",
                "light_white" if is_bg => "107",
                
                // Fallback for direct codes or unknown
                // Use original part to ensure lifetime is tied to style_str, not part_lower
                _ => part,
            };
            codes.push(code);
        }
        
        if codes.is_empty() {
            // Default fallback
            return if is_bg { "49".to_string() } else { "39".to_string() };
        }
        
        codes.join(";")
    }
}
