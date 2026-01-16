use crate::models::Task;
use anyhow::{anyhow, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceStatus {
    Running,
    Exited(u32),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct InstanceInfo {
    pub id: String,
    pub task_id: String,
    pub task_name: String,
    pub status: InstanceStatus,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub child_pid: Option<u32>,
    pub title: Option<String>,
}

pub struct SpawnedInstance {
    pub info: InstanceInfo,
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
}

struct RingBuffer {
    buf: VecDeque<u8>,
    cap: usize,
}

impl RingBuffer {
    fn new(cap: usize) -> Self {
        Self {
            buf: VecDeque::with_capacity(cap),
            cap,
        }
    }

    fn push(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        if data.len() >= self.cap {
            self.buf.clear();
            let start = data.len() - self.cap;
            self.buf.extend(data[start..].iter().copied());
            return;
        }
        while self.buf.len() + data.len() > self.cap {
            self.buf.pop_front();
        }
        self.buf.extend(data.iter().copied());
    }

    fn snapshot(&self) -> Vec<u8> {
        self.buf.iter().copied().collect()
    }
}

struct InstanceEntry {
    info: InstanceInfo,
    killer: Box<dyn ChildKiller + Send + Sync>,
    buffer: RingBuffer,
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
}

#[derive(Clone)]
pub struct SessionManager {
    instances: Arc<Mutex<HashMap<String, InstanceEntry>>>,
    counters: Arc<Mutex<HashMap<String, u32>>>,
    buffer_cap: usize,
}

impl SessionManager {
    pub fn new(buffer_cap: usize) -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
            counters: Arc::new(Mutex::new(HashMap::new())),
            buffer_cap,
        }
    }

    pub fn spawn_raw(&self, task: &Task, command: &str) -> Result<SpawnedInstance> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.arg("-c");
        
        // Ensure the shell remains open after the command finishes
        let final_command = format!("{}; exec {}", command, shell);
        cmd.arg(final_command);

        if let Some(cwd) = task.cwd.clone() {
            cmd.cwd(cwd);
        }
        if task.env_clear.unwrap_or(false) {
            cmd.env_clear();
        }
        if let Some(env) = task.env.clone() {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let mut child = pair.slave.spawn_command(cmd)?;
        let child_pid = child.process_id();
        let killer = child.clone_killer();

        // Take the writer immediately to avoid "cannot take writer more than once" later
        let writer = pair.master.take_writer()?;

        let instance_id = self.next_instance_id(&task.id);
        let now = now_epoch();
        let info = InstanceInfo {
            id: instance_id.clone(),
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            status: InstanceStatus::Running,
            started_at: now,
            ended_at: None,
            child_pid,
            title: None,
        };

        let entry = InstanceEntry {
            info: info.clone(),
            killer,
            buffer: RingBuffer::new(self.buffer_cap),
            master: None,
            writer: None,
        };

        {
            let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
            guard.insert(instance_id.clone(), entry);
        }

        let instances = Arc::clone(&self.instances);
        let instance_id_clone = instance_id.clone();
        tokio::task::spawn_blocking(move || {
            let status = child.wait();
            let mut guard = match instances.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };
            if let Some(entry) = guard.get_mut(&instance_id_clone) {
                let ended_at = now_epoch();
                entry.info.ended_at = Some(ended_at);
                entry.info.status = match status {
                    Ok(exit) => InstanceStatus::Exited(exit.exit_code()),
                    Err(err) => InstanceStatus::Error(err.to_string()),
                };
            }
        });

        Ok(SpawnedInstance { info, master: pair.master, writer })
    }

    pub fn spawn(&self, task: &Task, command: &str) -> Result<InstanceInfo> {
        let spawned = self.spawn_raw(task, command)?;
        self.return_master(&spawned.info.id, spawned.master, spawned.writer)?;
        Ok(spawned.info)
    }

    pub fn list_instances(&self) -> Result<Vec<InstanceInfo>> {
        let guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        Ok(guard.values().map(|entry| entry.info.clone()).collect())
    }

    pub fn get_status(&self, id: &str) -> Result<Option<InstanceStatus>> {
        let guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        Ok(guard.get(id).map(|entry| entry.info.status.clone()))
    }

    pub fn append_output(&self, id: &str, data: &[u8]) -> Result<()> {
        let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        if let Some(entry) = guard.get_mut(id) {
            entry.buffer.push(data);
            if let Some(title) = parse_title(data) {
                entry.info.title = Some(title);
            }
        }
        Ok(())
    }

    pub fn buffer_snapshot(&self, id: &str) -> Result<Vec<u8>> {
        let guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        Ok(guard
            .get(id)
            .map(|entry| entry.buffer.snapshot())
            .unwrap_or_default())
    }

    pub fn kill(&self, id: &str) -> Result<()> {
        let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        let entry = guard.get_mut(id).ok_or_else(|| anyhow!("instance not found"))?;
        entry.killer.kill()?;
        Ok(())
    }

    pub fn take_master(&self, id: &str) -> Result<Option<(Box<dyn MasterPty + Send>, Box<dyn Write + Send>)>> {
        let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        if let Some(entry) = guard.get_mut(id) {
            if let (Some(master), Some(writer)) = (entry.master.take(), entry.writer.take()) {
                return Ok(Some((master, writer)));
            }
        }
        Ok(None)
    }

    pub fn return_master(&self, id: &str, master: Box<dyn MasterPty + Send>, writer: Box<dyn Write + Send>) -> Result<()> {
        let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        if let Some(entry) = guard.get_mut(id) {
            entry.master = Some(master);
            entry.writer = Some(writer);
        }
        Ok(())
    }

    pub fn remove_if_exited(&self, id: &str) -> Result<bool> {
        let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        if let Some(entry) = guard.get(id) {
            if matches!(entry.info.status, InstanceStatus::Exited(_)) {
                guard.remove(id);
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn terminate_all(&self, signal: i32) -> Result<()> {
        let guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        for entry in guard.values() {
            if let Some(pid) = entry.info.child_pid {
                unsafe {
                    libc::kill(pid as libc::pid_t, signal);
                }
            } else {
                let _ = entry.killer.clone_killer().kill();
            }
        }
        Ok(())
    }

    fn next_instance_id(&self, task_id: &str) -> String {
        let mut guard = self.counters.lock().expect("instance counters poisoned");
        let counter = guard.entry(task_id.to_string()).or_insert(0);
        *counter += 1;
        format!("{}#{}", task_id, *counter)
    }
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn parse_title(data: &[u8]) -> Option<String> {
    // Simple parser for xterm title sequences: \x1b]0;TITLE\x07 or \x1b]2;TITLE\x07
    // This is a heuristic and might miss split sequences or support only basic cases.
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0x1b && i + 3 < data.len() {
            // Check for ]0; or ]2;
            if data[i+1] == b']' && (data[i+2] == b'0' || data[i+2] == b'2') && data[i+3] == b';' {
                let start = i + 4;
                // Find terminator \x07 (BEL)
                if let Some(end) = data[start..].iter().position(|&b| b == 0x07) {
                    let title_bytes = &data[start..start+end];
                    if let Ok(title) = std::str::from_utf8(title_bytes) {
                        return Some(title.to_string());
                    }
                }
            }
        }
        i += 1;
    }
    None
}
