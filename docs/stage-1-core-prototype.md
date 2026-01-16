# 阶段一：核心重构与多实例透传架构 (Week 1-2)

## 目标
构建一个类似 Tmux 的“会话级”任务管理工具。
- **核心能力**：支持为同一个任务定义启动多个独立实例，并对其实施生命周期管理。
- **交互模式**：采用 "Menu (Ratatui) + Passthrough (Native)" 混合模式。
- **操作体验**：支持实例切换、后台挂起 (Detach)、状态实时追踪。

## 详细任务清单

### 1. Core 库：会话与实例管理器
- [x] **[Model] 实例状态追踪**
  - 定义 `InstanceStatus`: `Running`, `Exited(code)`, `Error(msg)`。
  - 每个实例分配唯一 ID (如 `task-id#1`, `task-id#2`)。
- [x] **[Manager] SessionManager 实现**
  - 维护全局实例表，支持 `spawn(task_config)`、`kill(instance_id)`、`get_status(instance_id)`。
  - 异步监听进程退出，实时更新状态表。
- [x] **[API] 句柄剥离 (Raw Spawn)**
  - 实现 `spawn_raw`，启动进程后返回 `PtyPair` (Master) 的读写句柄。
  - 确保句柄在实例运行期间被 CLI 正确持有或移交。
- [x] **[Data] 上下文回放缓冲**
  - 为每个实例维护一个 RingBuffer<u8> (如 16KB)。
  - IO 转发线程在读取 PTY 数据时，不仅写入 Stdout，同时写入 RingBuffer。
  - `spawn_raw` 或 `attach` 时，先输出 Buffer 内容。

### 2. CLI：多实例管理界面 (Ratatui)
- [x] **[UI] 树形/分组任务列表**
  - 第一层：配置文件中定义的任务模板。
  - 第二层：该任务下已创建的实例（显示 PID、运行时长、状态）。
- [x] **[Interaction] 快捷操作**
  - `Enter`: 启动新实例或进入选中实例。
  - `d`: 仅删除已退出的实例记录。
  - `X`: 强制杀死选中的运行中实例。
  - `Q`: 杀死所有实例并退出整个 CmdHub。
- [x] **[UI] 视觉层级优化**
  - 支持 Tab 键折叠/展开任务分组。
  - 实例状态增加颜色指示 (🟢 Running, 🔴 Error, ⚪ Exited)。

### 3. CLI：增强型透传引擎 (Passthrough + Command Mode)
- [x] **[Engine] 滚动区与状态栏**
  - 维持底部 1 行物理滚动限制 (`CSI 1;H-1r`)。
  - 绘制静态状态栏，显示：`[Task Name] | Instance ID | Status: Running | Ctrl+p: Cmd Mode`。
- [x] **[Logic] 模式切换状态机**
  - **透传模式 (Default)**：输入完全透传给 PTY，输出完全透传给 Stdout。
  - **指令模式 (Command Mode)**：
    - 拦截 `Ctrl+p` 进入此模式。状态栏高亮提示。
    - `q`: **Detach**。停止 IO 转发，保留进程，返回任务列表。
    - `k`: **Kill**。杀死当前进程，返回任务列表。
    - `b`: **Global Detach**。类似于 tmux detach，直接退出 CLI 但保持所有后台任务运行（需配合 Stage 2 的 Server 模式，阶段一先实现回到列表）。
- [x] **[Engine] 零拷贝 IO 转发**
  - 优化 `read/write` 循环，确保在透传模式下达到原生终端性能。
- [x] **[Interaction] 信号与实践传递**
  - 监听终端 Resize 事件，同步更新底层 PTY 的 rows/cols，防止 Vim 错位。
- [x] **[Config] 按键映射配置**
  - 避免硬编码 Ctrl+b，提取为 PREFIX_KEY 常量（建议默认 Ctrl+p 或 Ctrl+\）。
  - 实现 PREFIX_KEY + PREFIX_KEY 发送原按键功能。

### 4. 流程集成与稳定性
- [x] **[Flow] 动态参数注入**
  - 若任务含 `inputs`，进入实例前弹出 Ratatui 表单填充变量。
- [x] **[Safety] 终端复原**
  - 确保在任何崩溃或异常退出时，正确执行 `RM (Reset Mode)` 指令恢复全屏滚动区，避免破坏用户终端显示。
- [x] **[Safety] 孤儿进程处理**
  - 在 Stage 1 (无 Daemon) 模式下，当主进程 CmdHub 退出或 Crash 时，确保向所有子 PTY 发送 SIGHUP 或 SIGTERM，防止产生僵尸进程或后台残留进程占用端口。

## 交付物
一个功能完备的 CLI 任务中心：
1. **多实例并存**：可以同时运行多个 `npm start` 或 `tail -f` 实例并自由切换。
2. **Tmux 级体验**：通过快捷键在“操作任务”和“管理界面”之间无缝穿梭。
3. **状态透明**：在列表页一眼看清哪些任务崩溃了，哪些还在正常运行。
4. **极致性能**：运行页无任何渲染损耗，完美支持 `vim`、`fzf` 等复杂终端工具。
