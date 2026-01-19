use crate::models::{AppConfig, ClientConfig};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

const CLIENT_CONFIG_FILE_NAME: &str = "ch-client.toml";
const SERVER_CONFIG_FILE_NAME: &str = "ch-server.toml";
const CLIENT_GLOBAL_FILE_NAME: &str = "client.toml";
const SERVER_GLOBAL_FILE_NAME: &str = "server.toml";
const TASKS_DIR_NAME: &str = "tasks";

pub async fn load_server_config<P: AsRef<Path>>(path: P) -> Result<AppConfig> {
    let content = fs::read_to_string(&path).await?;
    let mut config: AppConfig = toml::from_str(&content)?;
    
    // Check for tasks directory relative to config file
    if let Some(parent) = path.as_ref().parent() {
        let tasks_dir = parent.join(TASKS_DIR_NAME);
        if tasks_dir.exists() && tasks_dir.is_dir() {
            let mut entries = fs::read_dir(tasks_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        // We assume task files contain a [[tasks]] array or similar structure
                        // For simplicity, let's try to parse as AppConfig partial and merge tasks
                        #[derive(serde::Deserialize)]
                        struct PartialConfig {
                            tasks: Option<Vec<crate::models::Task>>,
                        }
                        
                        if let Ok(partial) = toml::from_str::<PartialConfig>(&content) {
                            if let Some(tasks) = partial.tasks {
                                config.tasks.extend(tasks);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(config)
}

pub async fn load_server_config_auto() -> Result<AppConfig> {
    let path = resolve_server_config_path()?;
    load_server_config(path).await
}

pub async fn load_client_config<P: AsRef<Path>>(path: P) -> Result<ClientConfig> {
    let content = fs::read_to_string(&path).await?;
    let config: ClientConfig = toml::from_str(&content)?;
    Ok(config)
}

pub async fn load_client_config_auto() -> Result<ClientConfig> {
    let path = resolve_client_config_path()?;
    load_client_config(path).await
}

pub fn resolve_server_config_path() -> Result<PathBuf> {
    let candidates = server_config_candidates();
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
    Err(anyhow!(
        "server config not found; searched: {}",
        searched
    ))
}

pub fn resolve_client_config_path() -> Result<PathBuf> {
    let candidates = client_config_candidates();
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
    Err(anyhow!(
        "client config not found; searched: {}",
        searched
    ))
}

fn server_config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        paths.push(current_dir.join(SERVER_CONFIG_FILE_NAME));
    }

    if let Ok(home) = std::env::var("HOME") {
        paths.push(Path::new(&home).join(".cmdhub").join(SERVER_GLOBAL_FILE_NAME));
    }

    paths
}

fn client_config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        paths.push(current_dir.join(CLIENT_CONFIG_FILE_NAME));
    }

    if let Ok(home) = std::env::var("HOME") {
        paths.push(Path::new(&home).join(".cmdhub").join(CLIENT_GLOBAL_FILE_NAME));
    }

    paths
}
