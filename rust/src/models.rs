use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 执行模式
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskMode {
    /// shell -c 一次性执行，命令退出即结束
    Oneshot,
    /// 交互式 shell，保持 PTY 供后续输入
    Interactive,
}

impl Default for TaskMode {
    fn default() -> Self {
        Self::Oneshot
    }
}

/// 任务定义：对应命令行的单条命令
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    /// 唯一标识，UUID v4
    pub id: String,
    /// 前端显示名
    pub name: String,
    /// 实际执行的命令
    pub command: String,
    /// 工作路径
    pub cwd: Option<String>,
    /// 额外环境变量（追加到系统环境）
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// 是否继承系统环境变量，默认 true
    #[serde(default = "default_env_inherit")]
    pub env_inherit: bool,
    /// 执行模式
    #[serde(default)]
    pub mode: TaskMode,
}

fn default_env_inherit() -> bool {
    true
}

/// 编排步骤的触发条件
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepCondition {
    /// 编排启动时立即执行（仅第一步有效）
    OnStart,
    /// 上一步进程退出后执行
    AfterPrevious,
    /// 上一步启动后再等指定毫秒后执行（不等退出）
    AfterDelay,
}

/// 编排步骤
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineStep {
    /// 引用的 Task id
    pub task_id: String,
    /// 执行序号
    pub order: u32,
    /// 本步启动前的额外延时（毫秒）
    #[serde(default)]
    pub delay_ms: u64,
    /// 触发条件
    pub condition: StepCondition,
}

/// 编排：一组按顺序执行的任务
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub steps: Vec<PipelineStep>,
}

/// 实例状态
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InstanceStatus {
    Running,
    Exited {
        code: u32,
    },
    Killed,
    Error {
        message: String,
    },
}

/// 任务实例：一次任务执行
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskInstance {
    /// instance-{task_id}-{counter}
    pub id: String,
    pub task_id: String,
    pub task_name: String,
    pub command: String,
    pub status: InstanceStatus,
    /// Unix 时间戳
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub child_pid: Option<u32>,
    /// 所属编排运行 ID（单独启动时为 None）
    #[serde(default)]
    pub run_id: Option<String>,
}

/// 编排运行中的步骤状态
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepState {
    Pending,
    Running { instance_id: String },
    Completed { instance_id: String, exit_code: u32 },
    Failed { error: String },
}

/// 编排运行状态
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 编排运行状态
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PipelineRunState {
    pub run_id: String,
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub step_states: Vec<StepState>,
    pub status: PipelineStatus,
    /// Unix 时间戳
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

/// 实例事件，用于推送给 Flutter 端实时更新
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InstanceEvent {
    Started(TaskInstance),
    Output {
        instance_id: String,
        /// UTF-8 解码后的文本增量
        text: String,
    },
    Exited(TaskInstance),
    Error {
        instance_id: String,
        message: String,
    },
}
