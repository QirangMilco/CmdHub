use crate::models::{Pipeline, Task};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// 获取平台数据目录
///
/// Windows: exe 同级 data 文件夹（便携式部署）
/// macOS/Linux: ~/.cmdhub/
pub fn data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().context("cannot get exe path")?;
        let dir = exe
            .parent()
            .context("exe has no parent")?
            .join("data");
        Ok(dir)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home =
            dirs::home_dir().context("cannot get home directory")?;
        Ok(home.join(".cmdhub"))
    }
}

fn tasks_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("tasks.json"))
}

fn pipelines_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("pipelines.json"))
}

fn ensure_data_dir() -> Result<PathBuf> {
    let dir = data_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 加载所有任务
pub fn load_tasks() -> Result<Vec<Task>> {
    let path = tasks_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    let tasks: Vec<Task> = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(tasks)
}

/// 保存所有任务
pub fn save_tasks(tasks: &[Task]) -> Result<()> {
    ensure_data_dir()?;
    let path = tasks_path()?;
    let data = serde_json::to_string_pretty(tasks)?;
    fs::write(&path, data)?;
    Ok(())
}

/// 加载所有编排
pub fn load_pipelines() -> Result<Vec<Pipeline>> {
    let path = pipelines_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    let pipelines: Vec<Pipeline> = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(pipelines)
}

/// 保存所有编排
pub fn save_pipelines(pipelines: &[Pipeline]) -> Result<()> {
    ensure_data_dir()?;
    let path = pipelines_path()?;
    let data = serde_json::to_string_pretty(pipelines)?;
    fs::write(&path, data)?;
    Ok(())
}
