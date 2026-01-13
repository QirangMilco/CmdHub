use crate::models::AppConfig;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

const CONFIG_FILE_NAME: &str = "config.toml";

pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig> {
    let content = fs::read_to_string(path).await?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

pub async fn load_config_auto() -> Result<AppConfig> {
    let path = resolve_config_path()?;
    load_config(path).await
}

pub fn resolve_config_path() -> Result<PathBuf> {
    let candidates = config_candidates();
    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }
    let searched = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(anyhow!("config.toml not found; searched: {}", searched))
}

fn config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(dir) = std::env::var("CMDHUB_CONFIG_DIR") {
        paths.push(Path::new(&dir).join(CONFIG_FILE_NAME));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        paths.push(current_dir.join(CONFIG_FILE_NAME));
    }

    if let Ok(xdg_home) = std::env::var("XDG_CONFIG_HOME") {
        paths.push(Path::new(&xdg_home).join("cmdhub").join(CONFIG_FILE_NAME));
    } else if let Ok(home) = std::env::var("HOME") {
        paths.push(Path::new(&home).join(".config").join("cmdhub").join(CONFIG_FILE_NAME));
    }

    if let Ok(home) = std::env::var("HOME") {
        paths.push(Path::new(&home).join(".cmdhub").join(CONFIG_FILE_NAME));
    }

    paths
}
