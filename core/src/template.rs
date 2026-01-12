use crate::models::InputConfig;
use anyhow::{anyhow, Result};
use std::collections::HashMap;

fn default_from_input(config: &InputConfig) -> Option<String> {
    match config {
        InputConfig::Select { default, .. } => Some(default.clone()),
        InputConfig::Text { default, .. } => default.clone(),
    }
}

pub fn render_command(
    command: &str,
    values: &HashMap<String, String>,
    inputs: Option<&HashMap<String, InputConfig>>,
) -> Result<String> {
    let mut rendered = String::with_capacity(command.len());
    let mut cursor = 0;

    while let Some(start) = command[cursor..].find("{{") {
        let start = cursor + start;
        rendered.push_str(&command[cursor..start]);

        let after_start = start + 2;
        let end = command[after_start..]
            .find("}}")
            .ok_or_else(|| anyhow!("unclosed template variable"))?
            + after_start;

        let inner = command[after_start..end].trim();
        let mut parts = inner.splitn(2, '|');
        let name = parts.next().unwrap_or("").trim();
        if name.is_empty() {
            return Err(anyhow!("empty template variable"));
        }
        let inline_default = parts.next().map(|value| value.trim().to_string());

        let fallback = inputs
            .and_then(|map| map.get(name))
            .and_then(default_from_input);
        let value = values
            .get(name)
            .cloned()
            .or(inline_default)
            .or(fallback)
            .ok_or_else(|| anyhow!("missing value for template variable: {}", name))?;

        rendered.push_str(&value);
        cursor = end + 2;
    }

    rendered.push_str(&command[cursor..]);
    Ok(rendered)
}
