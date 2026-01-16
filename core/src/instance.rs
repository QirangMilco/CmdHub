use crate::models::Task;
use anyhow::{anyhow, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::sync::OnceLock;
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
    osc_parser: OscParser,
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
        if is_bash_shell(&shell) {
            let rcfile = ensure_bash_rcfile()?;
            cmd.arg("--noprofile");
            cmd.arg("--rcfile");
            cmd.arg(&rcfile);
            cmd.arg("-i");
            cmd.env("CMDHUB_INIT_CMD", command);
        } else {
            cmd.arg("-c");
            // Ensure the shell remains open after the command finishes
            let final_command = format!("{}; exec {}", command, shell);
            cmd.arg(final_command);
        }

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
            osc_parser: OscParser::new(),
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
            let mut titles = Vec::new();
            entry.osc_parser.collect_titles(data, &mut titles);
            let mut last_title = None;
            for title in titles {
                if title.trim().starts_with("CMDHUB:") {
                    let _ = apply_cmdhub_title(&title, &mut entry.info);
                } else {
                    last_title = Some(title);
                }
            }
            if let Some(title) = last_title {
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

    pub fn kill_and_remove(&self, id: &str) -> Result<bool> {
        let entry = {
            let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
            guard.remove(id)
        };
        if let Some(mut entry) = entry {
            let _ = entry.killer.kill();
            return Ok(true);
        }
        Ok(false)
    }

    pub fn remove(&self, id: &str) -> Result<bool> {
        let mut guard = self.instances.lock().map_err(|_| anyhow!("instance lock poisoned"))?;
        Ok(guard.remove(id).is_some())
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

const OSC_TITLE_LIMIT: usize = 2048;

struct OscParser {
    state: OscState,
    buf: Vec<u8>,
}

enum OscState {
    Idle,
    Esc,
    Osc,
    OscCode,
    Collect,
}

impl OscParser {
    fn new() -> Self {
        Self {
            state: OscState::Idle,
            buf: Vec::new(),
        }
    }

    fn collect_titles(&mut self, data: &[u8], titles: &mut Vec<String>) {
        for &b in data {
            match self.state {
                OscState::Idle => {
                    if b == 0x1b {
                        self.state = OscState::Esc;
                    }
                }
                OscState::Esc => {
                    if b == b']' {
                        self.state = OscState::Osc;
                    } else if b != 0x1b {
                        self.state = OscState::Idle;
                    }
                }
                OscState::Osc => {
                    if b == b'0' || b == b'2' {
                        self.state = OscState::OscCode;
                    } else {
                        self.state = OscState::Idle;
                    }
                }
                OscState::OscCode => {
                    if b == b';' {
                        self.buf.clear();
                        self.state = OscState::Collect;
                    } else {
                        self.state = OscState::Idle;
                    }
                }
                OscState::Collect => {
                    if b == 0x07 {
                        if let Ok(title) = std::str::from_utf8(&self.buf) {
                            titles.push(title.to_string());
                        }
                        self.buf.clear();
                        self.state = OscState::Idle;
                    } else if self.buf.len() < OSC_TITLE_LIMIT {
                        self.buf.push(b);
                    }
                }
            }
        }
    }
}

fn apply_cmdhub_title(title: &str, info: &mut InstanceInfo) -> bool {
    let title = title.trim();
    let payload = match title.strip_prefix("CMDHUB:") {
        Some(payload) => payload,
        None => return false,
    };
    let mut state = None;
    let mut pid = None;
    let mut code = None;
    for part in payload.split(';') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap_or("").trim();
        let value = kv.next().unwrap_or("").trim();
        match key {
            "state" => state = Some(value.to_string()),
            "pid" => pid = value.parse::<u32>().ok(),
            "code" => code = value.parse::<u32>().ok(),
            _ => {}
        }
    }

    match state.as_deref() {
        Some("running") => {
            info.status = InstanceStatus::Running;
            info.ended_at = None;
            info.child_pid = pid.or(info.child_pid);
            true
        }
        Some("exited") => {
            let exit_code = code.unwrap_or(0);
            info.status = InstanceStatus::Exited(exit_code);
            info.ended_at = Some(now_epoch());
            true
        }
        _ => false,
    }
}

fn is_bash_shell(shell: &str) -> bool {
    shell.ends_with("bash") || shell.contains("/bash")
}

fn ensure_bash_rcfile() -> Result<String> {
    static RCFILE: OnceLock<String> = OnceLock::new();
    if let Some(path) = RCFILE.get() {
        return Ok(path.clone());
    }
    let mut path = std::env::temp_dir();
    path.push("cmdhub_bashrc");
    let rc = r#"
cmdhub_emit() {
    printf '\033]0;CMDHUB:%s\007' "$1"
}

cmdhub_debug_trap() {
    if [ -n "${CMDHUB_IN_HOOK-}" ]; then
        return
    fi
    case "$BASH_COMMAND" in
        cmdhub_precmd*|cmdhub_debug_trap*|cmdhub_emit*|cmdhub_watch_fg*)
            return
            ;;
    esac
    cmdhub_watch_fg "$CMDHUB_SHELL_PID" "$CMDHUB_SHELL_PGID" 2>/dev/null &
    disown 2>/dev/null || true
}

cmdhub_precmd() {
    CMDHUB_IN_HOOK=1
    local code="$?"
    cmdhub_emit "state=exited;code=$code"
    CMDHUB_IN_HOOK=
}

cmdhub_watch_fg() {
    CMDHUB_IN_HOOK=1
    local shell_pid="$1"
    local shell_pgid="$2"
    local tpgid
    local i=0
    while [ $i -lt 50 ]; do
        tpgid="$(ps -o tpgid= -p "$shell_pid" 2>/dev/null | tr -d ' ')"
        if [ -n "$tpgid" ] && [ "$tpgid" != "$shell_pgid" ] && [ "$tpgid" != "-" ]; then
            cmdhub_emit "state=running;pid=$tpgid"
            CMDHUB_IN_HOOK=
            return
        fi
        sleep 0.02
        i=$((i+1))
    done
    CMDHUB_IN_HOOK=
}

CMDHUB_SHELL_PID="$$"
CMDHUB_SHELL_PGID="$(ps -o pgid= -p "$CMDHUB_SHELL_PID" 2>/dev/null | tr -d ' ')"

if [ -f /etc/bash.bashrc ]; then
    . /etc/bash.bashrc
fi
if [ -f "$HOME/.bashrc" ]; then
    . "$HOME/.bashrc"
fi

if declare -p PROMPT_COMMAND 2>/dev/null | grep -q 'declare -a'; then
    PROMPT_COMMAND=(cmdhub_precmd "${PROMPT_COMMAND[@]}")
else
    PROMPT_COMMAND="cmdhub_precmd${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
fi
trap 'cmdhub_debug_trap' DEBUG

if [ -n "${CMDHUB_INIT_CMD-}" ] && [ -z "${CMDHUB_INIT_DONE-}" ]; then
    CMDHUB_INIT_DONE=1
    eval "$CMDHUB_INIT_CMD"
fi
"#;
    fs::write(&path, rc.trim_start())?;
    let path_str = path.to_string_lossy().to_string();
    let _ = RCFILE.set(path_str.clone());
    Ok(path_str)
}
