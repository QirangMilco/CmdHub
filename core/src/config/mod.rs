use crate::models::AppConfig;
use anyhow::Result;
use std::path::Path;
use tokio::fs;

pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig> {
    let content = fs::read_to_string(path).await?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}
