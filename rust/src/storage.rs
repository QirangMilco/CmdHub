use crate::models::{Pipeline, PipelineRunState, Task, TaskInstance};
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

fn instances_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("instances"))
}

fn runs_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("runs.json"))
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

// ============================================================
// 实例持久化
// ============================================================

/// 保存已退出实例的信息和输出
pub fn save_instance(instance: &TaskInstance, output: &[u8]) -> Result<()> {
    let dir = instances_dir()?;
    fs::create_dir_all(&dir)?;
    let info_path = dir.join(format!("{}.json", instance.id));
    let data = serde_json::to_string_pretty(instance)?;
    fs::write(&info_path, data)?;
    let output_path = dir.join(format!("{}.txt", instance.id));
    fs::write(&output_path, output)?;
    Ok(())
}

/// 加载所有历史实例（仅已退出的）
pub fn load_instances() -> Result<Vec<(TaskInstance, Vec<u8>)>> {
    let dir = instances_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut results = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let data = fs::read_to_string(&path)?;
            let instance: TaskInstance = serde_json::from_str(&data)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            let output_path = path.with_extension("txt");
            let output = if output_path.exists() {
                fs::read(&output_path)?
            } else {
                Vec::new()
            };
            results.push((instance, output));
        }
    }
    Ok(results)
}

/// 删除实例持久化文件
pub fn remove_instance_files(instance_id: &str) -> Result<()> {
    let dir = instances_dir()?;
    let json_path = dir.join(format!("{}.json", instance_id));
    let txt_path = dir.join(format!("{}.txt", instance_id));
    let _ = fs::remove_file(json_path);
    let _ = fs::remove_file(txt_path);
    Ok(())
}

// ============================================================
// 编排运行持久化
// ============================================================

/// 保存编排运行
pub fn save_runs(runs: &[PipelineRunState]) -> Result<()> {
    ensure_data_dir()?;
    let path = runs_path()?;
    let data = serde_json::to_string_pretty(runs)?;
    fs::write(&path, data)?;
    Ok(())
}

/// 加载所有编排运行
pub fn load_runs() -> Result<Vec<PipelineRunState>> {
    let path = runs_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    let runs: Vec<PipelineRunState> = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(runs)
}

/// 添加或更新单个编排运行
pub fn add_run(run: &PipelineRunState) -> Result<()> {
    let mut runs = load_runs()?;
    if let Some(pos) = runs.iter().position(|r| r.run_id == run.run_id) {
        runs[pos] = run.clone();
    } else {
        runs.push(run.clone());
    }
    save_runs(&runs)
}

/// 删除编排运行
pub fn remove_run(run_id: &str) -> Result<()> {
    let mut runs = load_runs()?;
    runs.retain(|r| r.run_id != run_id);
    save_runs(&runs)
}

// ============================================================
// 日志系统 —— 带级别控制
// ============================================================

/// 日志级别
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn from_env() -> Self {
        if let Ok(val) = std::env::var("CMDHUB_LOG") {
            match val.to_lowercase().as_str() {
                "debug" => Self::Debug,
                "info" => Self::Info,
                "warn" => Self::Warn,
                "error" => Self::Error,
                _ => Self::default(),
            }
        } else {
            Self::default()
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        // debug 编译输出 Info 及以上，release 只输出 Warn 及以上
        if cfg!(debug_assertions) {
            Self::Info
        } else {
            Self::Warn
        }
    }
}

/// 判断指定级别是否应输出
pub fn log_enabled(level: LogLevel) -> bool {
    static CURRENT: std::sync::OnceLock<LogLevel> = std::sync::OnceLock::new();
    let current = *CURRENT.get_or_init(LogLevel::from_env);
    level >= current
}

/// 写入日志（格式化）
pub fn log_msg(level: LogLevel, msg: &str) {
    if !log_enabled(level) {
        return;
    }
    let prefix = match level {
        LogLevel::Debug => "[DEBUG]",
        LogLevel::Info => "[INFO]",
        LogLevel::Warn => "[WARN]",
        LogLevel::Error => "[ERROR]",
    };
    let line = format!("{} {}", prefix, msg);
    if let Ok(dir) = data_dir() {
        let path = dir.join("cmdhub.log");
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            use std::io::Write;
            let _ = writeln!(file, "{}", line);
        }
    }
}

/// 便捷函数
pub fn log_debug(msg: &str) { log_msg(LogLevel::Debug, msg); }
pub fn log_info(msg: &str) { log_msg(LogLevel::Info, msg); }
pub fn log_warn(msg: &str) { log_msg(LogLevel::Warn, msg); }
pub fn log_error(msg: &str) { log_msg(LogLevel::Error, msg); }
