use crate::models::{InstanceEvent, InstanceStatus, Task, TaskInstance, TaskMode};
use anyhow::{anyhow, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

/// 环形缓冲区，用于存储实例输出
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
            let start = data.len().saturating_sub(self.cap);
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
    info: TaskInstance,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
    buffer: RingBuffer,
    #[allow(dead_code)]
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
}

/// 进程执行器
///
/// 管理所有任务实例的生命周期：创建 PTY、启动命令、缓冲输出、停止进程。
pub struct Executor {
    instances: Arc<Mutex<HashMap<String, InstanceEntry>>>,
    counters: Arc<Mutex<HashMap<String, u32>>>,
    buffer_cap: usize,
    event_tx: broadcast::Sender<InstanceEvent>,
}

impl Executor {
    pub fn new(buffer_cap: usize) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let mut instances = HashMap::new();
        // 加载历史实例（已退出的）
        if let Ok(history) = crate::storage::load_instances() {
            for (info, output) in history {
                let mut entry = InstanceEntry {
                    info: info.clone(),
                    killer: None,
                    buffer: RingBuffer::new(buffer_cap),
                    master: None,
                    writer: None,
                };
                entry.buffer.push(&output);
                instances.insert(info.id.clone(), entry);
            }
        }
        Self {
            instances: Arc::new(Mutex::new(instances)),
            counters: Arc::new(Mutex::new(HashMap::new())),
            buffer_cap,
            event_tx,
        }
    }

    /// 获取事件广播的接收端，供 Flutter 端订阅
    pub fn event_receiver(&self) -> broadcast::Receiver<InstanceEvent> {
        self.event_tx.subscribe()
    }

    /// 启动任务，返回实例信息
    pub fn spawn(&self, task: &Task, run_id: Option<String>) -> Result<TaskInstance> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = if cfg!(target_os = "windows") {
            "cmd.exe".to_string()
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        };

        let mut cmd = CommandBuilder::new(&shell);

        match task.mode {
            TaskMode::Oneshot => {
                if cfg!(target_os = "windows") {
                    cmd.arg("/C");
                } else {
                    cmd.arg("-c");
                }
                cmd.arg(&task.command);
            }
            TaskMode::Interactive => {
                if !cfg!(target_os = "windows") && is_bash(&shell) {
                    // 交互式 bash：使用 rcfile 注入 PROMPT_COMMAND 钩子
                    let rcfile = ensure_bash_rcfile()?;
                    cmd.arg("--noprofile");
                    cmd.arg("--rcfile");
                    cmd.arg(&rcfile);
                    cmd.arg("-i");
                } else if cfg!(target_os = "windows") {
                    // Windows 交互模式：保持 cmd 运行
                    cmd.arg("/K");
                    cmd.arg(&task.command);
                } else {
                    // 非 bash shell：执行命令后保持交互
                    cmd.arg("-c");
                    cmd.arg(format!("{}; exec {}", task.command, shell));
                }
            }
        }

        if let Some(ref cwd) = task.cwd {
            cmd.cwd(cwd);
        }

        if !task.env_inherit {
            cmd.env_clear();
        }

        // 继承系统环境变量
        if task.env_inherit {
            for (key, value) in std::env::vars() {
                cmd.env(key, value);
            }
        }

        // 覆盖/追加自定义环境变量
        for (key, value) in &task.env {
            cmd.env(key, value);
        }

        let mut child = pair.slave.spawn_command(cmd)?;
        let child_pid = child.process_id();
        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let master = Some(pair.master);

        let instance_id = self.next_instance_id(&task.id);
        let now = now_epoch();
        let info = TaskInstance {
            id: instance_id.clone(),
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            command: task.command.clone(),
            status: InstanceStatus::Running,
            started_at: now,
            ended_at: None,
            child_pid,
            run_id,
        };

        let entry = InstanceEntry {
            info: info.clone(),
            killer: Some(killer),
            buffer: RingBuffer::new(self.buffer_cap),
            master,
            writer: Some(writer),
        };

        {
            let mut guard = self
                .instances
                .lock()
                .map_err(|_| anyhow!("instance lock poisoned"))?;
            guard.insert(instance_id.clone(), entry);
        }

        let _ = self.event_tx.send(InstanceEvent::Started(info.clone()));

        // 后台线程读取 PTY 输出
        let instances = Arc::clone(&self.instances);
        let instance_id_clone = instance_id.clone();
        let event_tx = self.event_tx.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = &buf[..n];
                        let text = String::from_utf8_lossy(data).to_string();
                        let _ = event_tx.send(InstanceEvent::Output {
                            instance_id: instance_id_clone.clone(),
                            text,
                        });
                        if let Ok(mut guard) = instances.lock() {
                            if let Some(entry) = guard.get_mut(&instance_id_clone) {
                                entry.buffer.push(data);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // 后台线程等待进程退出
        let instances = Arc::clone(&self.instances);
        let instance_id_for_wait = instance_id.clone();
        let event_tx = self.event_tx.clone();

        std::thread::spawn(move || {
            let status = child.wait();
            let mut exit_info = None;
            {
                let mut guard = match instances.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                if let Some(entry) = guard.get_mut(&instance_id_for_wait) {
                    let ended_at = now_epoch();
                    entry.info.ended_at = Some(ended_at);
                    // 如果已手动停止，保留 Killed 状态
                    if !matches!(entry.info.status, InstanceStatus::Killed) {
                        entry.info.status = match status {
                            Ok(exit) => InstanceStatus::Exited {
                                code: exit.exit_code(),
                            },
                            Err(err) => InstanceStatus::Error {
                                message: err.to_string(),
                            },
                        };
                    }
                    exit_info = Some(entry.info.clone());
                }
            }
            if let Some(info) = exit_info {
                // 保存实例信息及输出到文件
                let output = {
                    let guard = instances.lock().unwrap_or_else(|e| e.into_inner());
                    guard.get(&instance_id_for_wait)
                        .map(|e| e.buffer.snapshot())
                        .unwrap_or_default()
                };
                let _ = crate::storage::save_instance(&info, &output);
                let _ = event_tx.send(InstanceEvent::Exited(info));
            }
        });

        Ok(info)
    }

    /// 向交互式实例的 stdin 写入数据
    pub fn write_input(&self, instance_id: &str, data: &[u8]) -> Result<()> {
        let mut guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        let entry = guard
            .get_mut(instance_id)
            .ok_or_else(|| anyhow!("instance not found: {}", instance_id))?;
        if let Some(ref mut writer) = entry.writer {
            writer.write_all(data)?;
            writer.flush()?;
        }
        Ok(())
    }

    /// 终止实例
    pub fn kill(&self, instance_id: &str) -> Result<()> {
        let mut guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        let entry = guard
            .get_mut(instance_id)
            .ok_or_else(|| anyhow!("instance not found: {}", instance_id))?;
        if let Some(ref mut killer) = entry.killer {
            match killer.kill() {
                Ok(()) => {
                    // 成功发送终止信号，标记为已手动停止
                    entry.info.ended_at = Some(now_epoch());
                    entry.info.status = InstanceStatus::Killed;
                    let output = entry.buffer.snapshot();
                    let _ = crate::storage::save_instance(&entry.info, &output);
                }
                Err(e) => {
                    let msg = e.to_string();
                    let is_no_such_process =
                        msg.contains("No such process") || msg.contains("os error 3");
                    if !is_no_such_process {
                        return Err(e.into());
                    }
                    // 进程已不存在，强制更新为已退出并持久化
                    entry.info.ended_at = Some(now_epoch());
                    entry.info.status = InstanceStatus::Exited { code: 0 };
                    let output = entry.buffer.snapshot();
                    let _ = crate::storage::save_instance(&entry.info, &output);
                }
            }
        }
        Ok(())
    }

    /// 获取实例的完整输出快照
    pub fn read_output(&self, instance_id: &str) -> Result<Vec<u8>> {
        let guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        let mut bytes = guard
            .get(instance_id)
            .map(|entry| entry.buffer.snapshot())
            .unwrap_or_default();
        // 过滤掉 EOT 控制字符 (^D, 0x04) 和其他除常用控制字符外的不可打印字符
        bytes.retain(|&b| {
            b.is_ascii_graphic() || b.is_ascii_whitespace() || b == b'\n' || b == b'\r' || b == b'\t' || b == b'\x1b' // 保留 ANSI 转义序列
        });
        Ok(bytes)
    }

    /// 列出所有实例（包括运行中和已退出）
    pub fn list_instances(&self) -> Result<Vec<TaskInstance>> {
        let guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        Ok(guard.values().map(|entry| entry.info.clone()).collect())
    }

    /// 获取单个实例信息
    pub fn get_instance(&self, instance_id: &str) -> Result<Option<TaskInstance>> {
        let guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        Ok(guard.get(instance_id).map(|entry| entry.info.clone()))
    }

    /// 移除已退出的实例
    pub fn remove_instance(&self, instance_id: &str) -> Result<bool> {
        let mut guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        let removed = guard.remove(instance_id).is_some();
        if removed {
            let _ = crate::storage::remove_instance_files(instance_id);
        }
        Ok(removed)
    }

    /// 退出应用时清理所有子进程
    pub fn terminate_all(&self) -> Result<()> {
        let guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        for entry in guard.values() {
            if let Some(ref killer) = entry.killer {
                let _ = killer.clone_killer().kill();
            }
        }
        Ok(())
    }

    /// 获取运行中实例数量
    pub fn running_count(&self) -> Result<usize> {
        let guard = self
            .instances
            .lock()
            .map_err(|_| anyhow!("instance lock poisoned"))?;
        Ok(guard
            .values()
            .filter(|e| matches!(e.info.status, InstanceStatus::Running))
            .count())
    }

    fn next_instance_id(&self, task_id: &str) -> String {
        let mut guard = self
            .counters
            .lock()
            .expect("instance counters poisoned");
        let counter = guard.entry(task_id.to_string()).or_insert(0);
        *counter += 1;
        format!("{}-{}", task_id, *counter)
    }
}

pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn is_bash(shell: &str) -> bool {
    shell.ends_with("bash") || shell.contains("/bash")
}

/// 生成注入 PROMPT_COMMAND 钩子的 bashrc，用于 Interactive 模式的状态追踪
fn ensure_bash_rcfile() -> Result<String> {
    use std::io::Write;
    let mut path = std::env::temp_dir();
    path.push("cmdhub_bashrc");

    let rc = r#"
cmdhub_precmd() {
    local code="$?"
    printf '\033]0;CMDHUB:state=exited;code=%s\007' "$code"
}
if declare -p PROMPT_COMMAND 2>/dev/null | grep -q 'declare -a'; then
    PROMPT_COMMAND=(cmdhub_precmd "${PROMPT_COMMAND[@]}")
else
    PROMPT_COMMAND="cmdhub_precmd${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
fi
if [ -f /etc/bash.bashrc ]; then
    . /etc/bash.bashrc
fi
if [ -f "$HOME/.bashrc" ]; then
    . "$HOME/.bashrc"
fi
"#;

    let mut file = std::fs::File::create(&path)?;
    file.write_all(rc.trim_start().as_bytes())?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskMode;

    #[tokio::test]
    async fn test_spawn_oneshot_echo() {
        let executor = Executor::new(64 * 1024);
        let task = Task {
            id: "test-echo".to_string(),
            name: "Test Echo".to_string(),
            command: "echo hello world".to_string(),
            cwd: None,
            env: HashMap::new(),
            env_inherit: true,
            mode: TaskMode::Oneshot,
        };
        let instance = executor.spawn(&task).expect("spawn failed");
        assert_eq!(instance.task_id, "test-echo");
        assert!(matches!(instance.status, InstanceStatus::Running));

        // 等待 echo 进程退出
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        let output = executor.read_output(&instance.id).expect("read failed");
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("hello world"), "expected 'hello world' in output, got: {}", text);
    }
}
