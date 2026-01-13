use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Pending,
    Running,
    Exited,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInfo {
    pub id: Uuid,
    pub task_id: String,
    pub task_name: String,
    pub session_name: Option<String>,
    pub command: String,
    pub cwd: Option<PathBuf>,
    pub env: Option<HashMap<String, String>>,
    pub env_clear: bool,
    pub status: SessionStatus,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub exit_code: Option<u32>,
    pub runner_pid: Option<u32>,
    pub child_pid: Option<u32>,
    pub socket_path: Option<PathBuf>,
    #[serde(default)]
    pub running_task_pids: Vec<u32>,
}

pub struct SessionStore {
    active_dir: PathBuf,
    history_dir: PathBuf,
}

impl SessionStore {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME").map_err(|_| anyhow!("HOME not set"))?;
        let base_dir = Path::new(&home).join(".cmdhub").join("sessions");
        let active_dir = base_dir.join("active");
        let history_dir = base_dir.join("history");
        fs::create_dir_all(&active_dir)?;
        fs::create_dir_all(&history_dir)?;
        Ok(Self {
            active_dir,
            history_dir,
        })
    }

    pub fn session_dir(&self, id: Uuid) -> PathBuf {
        self.active_dir.join(id.to_string())
    }

    pub fn history_session_dir(&self, id: Uuid) -> PathBuf {
        self.history_dir.join(id.to_string())
    }

    pub fn session_meta_path(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("meta.json")
    }

    pub fn session_log_path(&self, id: Uuid) -> PathBuf {
        self.session_dir(id).join("output.log")
    }

    pub fn create_session(
        &self,
        task_id: String,
        task_name: String,
        session_name: Option<String>,
        command: String,
        cwd: Option<PathBuf>,
        env: Option<HashMap<String, String>>,
        env_clear: bool,
    ) -> Result<SessionInfo> {
        let id = Uuid::new_v4();
        let dir = self.session_dir(id);
        fs::create_dir_all(&dir)?;
        let info = SessionInfo {
            id,
            task_id,
            task_name,
            session_name,
            command,
            cwd,
            env,
            env_clear,
            status: SessionStatus::Pending,
            started_at: now_epoch(),
            ended_at: None,
            exit_code: None,
            runner_pid: None,
            child_pid: None,
            socket_path: None,
            running_task_pids: Vec::new(),
        };
        self.write_session(&info)?;
        Ok(info)
    }

    pub fn load_session(&self, id: Uuid) -> Result<SessionInfo> {
        let meta_path = self.session_meta_path(id);
        let data = fs::read(&meta_path)?;
        let info: SessionInfo = serde_json::from_slice(&data)?;
        Ok(info)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        list_sessions_in(&self.active_dir)
    }

    pub fn list_history(&self) -> Result<Vec<SessionInfo>> {
        list_sessions_in(&self.history_dir)
    }

    pub fn write_session(&self, info: &SessionInfo) -> Result<()> {
        let meta_path = self.session_meta_path(info.id);
        let data = serde_json::to_vec_pretty(info)?;
        fs::write(meta_path, data)?;
        Ok(())
    }

    pub fn move_to_history(&self, id: Uuid, max_entries: usize) -> Result<()> {
        let from = self.session_dir(id);
        let to = self.history_session_dir(id);
        if from.exists() {
            if to.exists() {
                fs::remove_dir_all(&to)?;
            }
            fs::rename(from, to)?;
        }
        self.prune_history(max_entries)?;
        Ok(())
    }

    pub fn prune_history(&self, max_entries: usize) -> Result<()> {
        let mut sessions = list_sessions_in(&self.history_dir)?;
        if sessions.len() <= max_entries {
            return Ok(());
        }
        sessions.sort_by_key(|info| info.started_at);
        let excess = sessions.len().saturating_sub(max_entries);
        for info in sessions.into_iter().take(excess) {
            let dir = self.history_session_dir(info.id);
            if dir.exists() {
                let _ = fs::remove_dir_all(dir);
            }
        }
        Ok(())
    }
}

fn list_sessions_in(dir: &Path) -> Result<Vec<SessionInfo>> {
    let mut sessions = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let meta_path = entry.path().join("meta.json");
        if !meta_path.exists() {
            continue;
        }
        if let Ok(data) = fs::read(&meta_path) {
            if let Ok(info) = serde_json::from_slice::<SessionInfo>(&data) {
                sessions.push(info);
            }
        }
    }
    sessions.sort_by_key(|info| info.started_at);
    Ok(sessions)
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
