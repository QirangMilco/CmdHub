# CmdHub 实现方案

## 1. 项目定位

跨平台（macOS / Windows / Linux）本机命令行应用管理工具。提供 GUI 界面管理所有需要命令行执行的后台或短期程序。

核心能力：

- **任务管理**：定义单条命令（id、名称、命令、工作路径、环境变量）
- **编排管理**：定义多个任务的启动顺序、延时、触发条件
- **实例监控**：查看运行中进程的状态、输出、运行时长
- **进程控制**：启动、停止、重启

技术栈：**Flutter（GUI） + Rust（核心逻辑）**，通过 `flutter_rust_bridge` v2 桥接。

## 2. 参考项目

| 项目 | 位置 | 借鉴内容 |
|---|---|---|
| v0 | `refs/v0/` | PTY 进程管理（`core/src/instance.rs`）、Task 模型、JSON 持久化模式、OSC 解析器 |
| VntApp | `refs/VntApp/` | Flutter 项目结构、`flutter_rust_bridge` 集成方式、CustomAppBar 风格、系统托盘/窗口管理 |

v0 的过度设计（server/client/RPC/WebSocket/Tauri/TUI/Web 六层分发）全部砍掉。本方案只保留 Rust 核心 + Flutter GUI 两层。

## 3. 数据模型

### 3.1 Task（任务）

```rust
pub struct Task {
    pub id: String,                      // 唯一标识，UUID v4
    pub name: String,                    // 前端显示名
    pub command: String,                 // 实际执行的命令
    pub cwd: Option<String>,             // 工作路径
    pub env: HashMap<String, String>,    // 额外环境变量（追加到系统环境）
    pub env_inherit: bool,               // 是否继承系统环境变量，默认 true
    pub mode: TaskMode,                  // 执行模式
}

pub enum TaskMode {
    Oneshot,      // shell -c 一次性执行，命令退出即结束
    Interactive,  // 交互式 shell，保持 PTY 供后续输入
}
```

与 v0 模型的差异：

- 去掉 `category`（无分组需求）
- 去掉 `InputConfig` 交互参数（本机 GUI 不需要模板参数替换）
- `env_clear` 改为语义更清晰的 `env_inherit`（默认 true）
- 新增 `mode` 字段区分一次性/交互式执行

### 3.2 Pipeline（编排）

```rust
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub steps: Vec<PipelineStep>,
}

pub struct PipelineStep {
    pub task_id: String,           // 引用 Task.id
    pub order: u32,                // 执行序号（从小到大依次处理）
    pub delay_ms: u64,             // 本步启动前的额外延时
    pub condition: StepCondition,  // 触发条件
}

pub enum StepCondition {
    OnStart,            // 编排启动时立即执行（仅第一步有效）
    AfterPrevious,      // 上一步进程退出后执行
    AfterDelay,         // 上一步启动后再等 delay_ms 后执行（不等退出）
}
```

### 3.3 TaskInstance（实例）

实例代表一次任务执行。无论是 Oneshot 还是 Interactive 模式，实例都会记录完整输出（环形缓冲区），退出后仍可查看，直到用户手动清除。

```rust
pub struct TaskInstance {
    pub id: String,              // instance-{task_id}-{counter}
    pub task_id: String,
    pub task_name: String,
    pub command: String,
    pub status: InstanceStatus,
    pub started_at: u64,         // Unix 时间戳
    pub ended_at: Option<u64>,
    pub child_pid: Option<u32>,
}

pub enum InstanceStatus {
    Running,
    Exited { code: u32 },        // Oneshot 命令执行完毕，输出仍可查看
    Error { message: String },
}
```

## 4. 分层架构

```
┌──────────────────────────────────────┐
│  Flutter UI Layer                    │
│  ├─ pages/   (Home, TaskEditor,      │
│  │            PipelineEditor,         │
│  │            InstanceDetail)        │
│  ├─ widgets/ (OutputViewer,          │
│  │            StatusBadge, EnvEditor)│
│  └─ services/ (CmdHubService         │
│               封装 bridge 调用)       │
├──────────────────────────────────────┤
│  flutter_rust_bridge (自动生成)       │
├──────────────────────────────────────┤
│  Rust Core                           │
│  ├─ api/      (供 bridge 暴露的接口)  │
│  ├─ models.rs (数据结构定义)          │
│  ├─ executor.rs (PTY 进程管理)        │
│  ├─ pipeline.rs (编排引擎)            │
│  └─ storage.rs (JSON 持久化)         │
└──────────────────────────────────────┘
```

## 5. Rust 核心模块

### 5.1 executor.rs — 进程执行器

借鉴 v0 `core/src/instance.rs` 的 PTY 管理逻辑。

核心结构：

```rust
pub struct Executor {
    instances: Arc<Mutex<HashMap<String, InstanceEntry>>>,
    counters: Arc<Mutex<HashMap<String, u32>>>,
    buffer_cap: usize,  // 输出环形缓冲区容量（默认 64KB）
}

struct InstanceEntry {
    info: TaskInstance,
    killer: Box<dyn ChildKiller + Send + Sync>,
    buffer: RingBuffer,
    // Oneshot 模式下不需要保留 master/writer
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Box<dyn Write + Send>>,
}
```

关键方法：

| 方法 | 说明 |
|---|---|
| `spawn(task: &Task) -> TaskInstance` | 创建 PTY，设置 cwd/env，启动 shell |
| `kill(instance_id: &str)` | 向子进程发送 SIGTERM |
| `read_output(instance_id: &str) -> Vec<u8>` | 读取环形缓冲区内容 |
| `write_input(instance_id: &str, data: &[u8])` | 向交互式进程发送 stdin |
| `list_instances() -> Vec<TaskInstance>` | 列出所有实例 |
| `terminate_all()` | 退出应用时清理所有子进程 |

执行逻辑：

- **Oneshot 模式**：`shell -c "{command}"`，进程退出后自动更新状态
- **Interactive 模式**：启动交互式 shell（bash `--rcfile` + PROMPT_COMMAND hook），通过 master PTY 读写

进程退出监听：每个实例用 `tokio::task::spawn_blocking` + `child.wait()` 等待退出，退出后通过回调通知状态变更。

### 5.2 pipeline.rs — 编排引擎

```rust
pub struct PipelineRunner {
    active_runs: Arc<Mutex<HashMap<String, PipelineRunState>>>,
}

pub struct PipelineRunState {
    pub pipeline_id: String,
    pub step_states: Vec<StepState>,  // 与 pipeline.steps 一一对应
    pub status: PipelineStatus,
}

pub enum StepState {
    Pending,
    Running { instance_id: String },
    Completed { instance_id: String, exit_code: u32 },
    Failed { error: String },
}

pub enum PipelineStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

执行流程：

1. 按 `order` 升序排列步骤
2. 对于第一步（`condition == OnStart`）：立即启动
3. 对于后续步骤：
   - `AfterPrevious`：等待上一步实例退出（监听 executor 的状态回调），退出码为 0 才继续
   - `AfterDelay`：上一步启动后等 `delay_ms` 毫秒启动
4. 每一步启动前额外等待 `delay_ms`
5. 任一步失败则整个编排标记为 Failed
6. 用户可随时取消编排（kill 所有已启动实例）

### 5.3 storage.rs — 持久化

数据目录策略：macOS/Linux 使用用户目录，Windows 使用可执行文件同级目录（便携式部署）。

| 平台 | 路径 |
|---|---|
| macOS | `~/.cmdhub/` |
| Linux | `~/.cmdhub/` |
| Windows | `{exe_dir}/data/`（可执行文件所在目录下的 data 文件夹） |

获取方式：

```rust
fn data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().expect("cannot get exe path");
        exe.parent().unwrap().join("data")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = dirs::home_dir().expect("cannot get home dir");
        home.join(".cmdhub")
    }
}
```

文件结构：

```
{data_dir}/
├── tasks.json         # Task 列表
├── pipelines.json     # Pipeline 列表
└── instances/         # 运行中实例元数据（崩溃恢复用）
```

```rust
pub struct Storage {
    base_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self>;                        // 自动选择平台路径
    pub fn load_tasks() -> Result<Vec<Task>>;
    pub fn save_tasks(tasks: &[Task]) -> Result<()>;
    pub fn load_pipelines() -> Result<Vec<Pipeline>>;
    pub fn save_pipelines(pipelines: &[Pipeline]) -> Result<()>;
}
```

序列化格式：JSON（`serde_json`），手动可编辑、可版本控制。

### 5.4 api/mod.rs — Bridge 暴露接口

用 `#[flutter_rust_bridge::frb]` 注解标记，Flutter 端可直接调用。

任务管理：

```rust
pub fn create_task(task: Task) -> Result<Task>;
pub fn update_task(task: Task) -> Result<Task>;
pub fn delete_task(id: String) -> Result<()>;
pub fn list_tasks() -> Result<Vec<Task>>;
pub fn get_task(id: String) -> Result<Task>;
```

编排管理：

```rust
pub fn create_pipeline(pipeline: Pipeline) -> Result<Pipeline>;
pub fn update_pipeline(pipeline: Pipeline) -> Result<Pipeline>;
pub fn delete_pipeline(id: String) -> Result<()>;
pub fn list_pipelines() -> Result<Vec<Pipeline>>;
pub fn get_pipeline(id: String) -> Result<Pipeline>;
pub fn run_pipeline(id: String) -> Result<String>;  // 返回 run_id
pub fn cancel_pipeline(run_id: String) -> Result<()>;
```

实例管理：

```rust
pub fn spawn_task(task_id: String) -> Result<TaskInstance>;
pub fn kill_instance(instance_id: String) -> Result<()>;
pub fn list_instances() -> Result<Vec<TaskInstance>>;
pub fn read_output(instance_id: String) -> Result<Vec<u8>>;
pub fn write_input(instance_id: String, data: Vec<u8>) -> Result<()>;
```

事件流（供 UI 实时更新）：

```rust
// 通过 flutter_rust_bridge 的 StreamSink 实现
pub fn instance_events(sink: StreamSink<InstanceEvent>) -> Result<()>;

pub enum InstanceEvent {
    Started(TaskInstance),
    Output { instance_id: String, data: Vec<u8> },
    Exited(TaskInstance),
    Error { instance_id: String, message: String },
}
```

## 6. Flutter UI 设计

风格参考 VntApp：teal 主色调、Material Design、ListTile + 交替行背景色。

### 6.1 页面结构

**HomePage（主页）**

- 顶部 TabBar：`任务` | `编排` | `运行中`
- 任务 Tab：
  - FAB（+）新建任务
  - 列表项显示：任务名称、命令预览、模式标签
  - 操作：运行（▶）、编辑（✎）、删除（✕）
- 编排 Tab：
  - FAB（+）新建编排
  - 列表项显示：编排名称、步骤数
  - 操作：运行（▶）、编辑（✎）、删除（✕）
- 实例 Tab（显示所有运行中和已退出的实例）：
  - 列表项显示：任务名、状态指示器、运行时长（运行中）/ 退出码（已退出）
  - Oneshot 任务退出后自动保留在列表中，输出完整可查
  - 操作：查看输出、停止（运行中）、清除（已退出）
  - 空态提示："暂无实例记录"

**TaskEditorPage（任务编辑）**

- 表单字段：
  - 名称（单行输入）
  - 命令（多行输入）
  - 工作路径（文本框 + 文件夹选择按钮，调用系统原生对话框）
  - 执行模式（下拉选择：一次性 / 交互式）
  - 继承系统环境变量（开关）
  - 环境变量（动态键值对列表，可添加/删除行）
- 底部操作栏：保存、测试运行

**PipelineEditorPage（编排编辑）**

- 编排名称输入
- 步骤列表（可拖拽排序）：
  - 每步显示：序号、任务名称、触发条件、延时
  - 点击展开编辑：选择 Task（下拉）、延时（数字输入）、触发条件（下拉）
  - 操作：添加步骤、删除步骤、上移/下移
- 底部操作栏：保存、运行

**InstanceDetailPage（实例详情）**

- 顶部信息区：任务名、启动时间、PID、状态徽标、退出码（Exited 时显示）
- 主体：OutputViewer（等宽字体纯文本输出，支持自动滚动到底部）
- Oneshot 任务：输出一次性完整展示，退出后只读
- Interactive 任务：额外显示输入框（发送 stdin）
- 底部操作栏：停止/重启（运行中）、复制输出、重新运行（已退出）

### 6.2 关键 Widget

**OutputViewer（输出查看器）**

- 接收 `Stream<List<int>>`，解码为 UTF-8 字符串
- 用 `ListView.builder` + `SelectableText` 渲染
- 简单去除 ANSI 转义码（用 `ansi_strip` 或正则），不模拟完整终端
- 自动滚动到底部（用户手动上滚后暂停自动滚动）

**StatusBadge（状态徽标）**

- 绿色圆点 + 文字：运行中
- 灰色圆点 + 文字：已退出
- 红色圆点 + 文字：错误

**EnvEditor（环境变量编辑器）**

- 动态行列表，每行：Key 输入框 + Value 输入框 + 删除按钮
- "添加变量"按钮

## 7. 技术选型

| 层 | 技术 | 说明 |
|---|---|---|
| GUI 框架 | Flutter 3.x | 跨平台，与 VntApp 一致 |
| Rust 桥接 | `flutter_rust_bridge` v2 | 类型安全，支持 async/Stream |
| PTY 管理 | `portable-pty` 0.8 | 与 v0 一致，跨平台伪终端 |
| 异步运行时 | `tokio` (full) | PTY I/O 和编排引擎需要异步 |
| 序列化 | `serde` + `serde_json` | 配置文件持久化 |
| 唯一 ID | `uuid` v1 (v4 feature) | Task/Pipeline/Instance ID |
| Dart 本地存储 | `shared_preferences` | 窗口大小、用户偏好等轻量 KV |
| 系统托盘 | `system_tray` + `window_manager` | 最小化到托盘 |
| 数据目录 | `std::env::current_exe` + `dirs` crate | Windows 用 exe 同级 data 目录，macOS/Linux 用 `~/.cmdhub/` |

## 8. 项目文件结构

```
CmdHub/
├── rust/                          # Rust 项目（Cargo workspace 或单独 crate）
│   ├── Cargo.toml
│   └── src/
│       ├── api/
│       │   ├── mod.rs             # frb init + 模块导出
│       │   └── cmdhub_api.rs      # 所有 #[frb] 接口函数
│       ├── models.rs              # Task, Pipeline, TaskInstance 等数据结构
│       ├── executor.rs            # PTY 进程执行器
│       ├── pipeline.rs            # 编排引擎
│       ├── storage.rs             # JSON 文件持久化
│       └── lib.rs
├── lib/                           # Flutter 项目
│   ├── main.dart                  # 入口 + MaterialApp + 系统托盘初始化
│   ├── models/
│   │   └── models.dart            # Dart 侧数据类（mirror Rust structs）
│   ├── pages/
│   │   ├── home_page.dart         # 主页（TabBar 三栏）
│   │   ├── task_editor_page.dart  # 任务编辑页
│   │   ├── pipeline_editor_page.dart
│   │   └── instance_detail_page.dart
│   ├── widgets/
│   │   ├── custom_app_bar.dart    # 复用 VntApp 风格
│   │   ├── output_viewer.dart     # 输出查看器
│   │   ├── status_badge.dart      # 状态徽标
│   │   └── env_editor.dart        # 环境变量键值对编辑器
│   ├── services/
│   │   └── cmdhub_service.dart    # 封装 bridge 调用的 service 类
│   └── src/rust/                  # flutter_rust_bridge 自动生成
├── pubspec.yaml
├── flutter_rust_bridge.yaml
└── assets/
    └── app_icon.png
```

## 9. 实现顺序

| 阶段 | 内容 | 交付标准 |
|---|---|---|
| 1 | Rust 核心：models → storage → executor | 能通过 Rust 测试启动命令、读取输出、停止进程 |
| 2 | Bridge 接口：api/cmdhub_api.rs | Flutter 端能调用所有核心函数 |
| 3 | Flutter 基础 UI：main + HomePage | 能看到任务列表、能启动/停止命令 |
| 4 | 任务编辑：TaskEditorPage + EnvEditor | 能新建/编辑/删除任务 |
| 5 | 实例详情：InstanceDetailPage + OutputViewer | 能实时查看命令输出 |
| 6 | 编排引擎：pipeline.rs + PipelineEditorPage | 能创建编排并按序执行多任务 |
| 7 | 系统托盘 + 窗口管理 | 关闭窗口最小化到托盘，应用图标 |

## 10. 关键设计决策

**为什么不用 SQLite 而用 JSON 文件？**

Task 和 Pipeline 的数据量很小（通常几十条），JSON 文件足够。手动可编辑、可 Git 版本控制、无需引入额外数据库依赖。

**输出流如何传给 Flutter？**

用 `flutter_rust_bridge` 的 `StreamSink`。Rust 侧有新输出就 push，Dart 侧用 `StreamBuilder` 消费。不需要 WebSocket 或 HTTP。

**Oneshot 任务的输出能查看吗？**

能。所有任务（无论 Oneshot 还是 Interactive）的输出都通过环形缓冲区捕获。Oneshot 命令执行完毕后，实例保留在列表中，状态变为 `Exited`，输出完整可查，直到用户手动清除。这与 Interactive 模式的区别仅在于：Oneshot 的 PTY 在进程退出后关闭，不能再发送 stdin。

**长命令 vs 短命令？**

通过 `TaskMode` 区分：

- `Oneshot`（默认）：`shell -c` 执行，命令退出即结束，输出保留可查
- `Interactive`：启动交互式 shell 保持 PTY，可发送 stdin，适合需要持续交互的长期进程

**跨平台差异如何处理？**

- `portable-pty` 在 Rust 层处理 PTY 的跨平台
- 系统托盘用 `system_tray` + `window_manager`
- 数据目录用 `dirs` crate
- Flutter 框架自身跨平台
