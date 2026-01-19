# Stage 1.1: 客户端-服务端架构重构方案 (Client-Server Refactoring)

## 1. 目标概述

当前 CmdHub CLI 是单体应用，生命周期与终端窗口绑定。为了实现多端接入（CLI/Web/Desktop）并支持任务在后台持久运行（如 tmux/screen），必须将架构重构为 **Client-Server** 模式。

- **CmdHub Server (Daemon)**: 负责进程托管、PTY 管理、状态维护、日志持久化。作为系统后台服务运行。
- **CmdHub Core**: 定义通用的数据模型、RPC 接口契约、以及核心逻辑（Session/Storage）。
- **CmdHub CLI (Client)**: 纯粹的 UI 展示层。负责发送指令（启动/停止任务）和渲染从 Server 流式传输过来的数据。

## 2. 架构设计

### 2.1 模块职责划分

| 模块 | 职责变更 |
| :--- | :--- |
| **core** | 新增 `rpc` 模块，定义 Service trait 和 Request/Response 结构。统一 `InstanceInfo` 和 `SessionInfo` 的概念。 |
| **server** | 独占 `SessionManager`。启动 RPC 服务监听（Unix Domain Socket 或 TCP）。处理 PTY IO 流的转发。 |
| **cli** | 移除 `SessionManager`。启动时尝试连接 Server，连接失败则尝试启动 Server。TUI 数据源改为 RPC 调用。 |

### 2.2 通信协议策略

- **控制平面 (Control Plane)**: 使用 RPC (推荐 `tarpc` 或 `tonic`) 处理低频交互。
  - `list_instances()` -> `Vec<InstanceInfo>`
  - `spawn_task(task_id, params)` -> `instance_id`
  - `stop_instance(instance_id)`
  - `get_history()`
- **数据平面 (Data Plane)**: 处理高频 IO (PTY Stream)。
  - `attach(instance_id)` -> 建立全双工流 (WebSocket 或 Raw TCP/UDS Stream)。
  - 将 Server 端的 PTY Master `Reader`/`Writer` 对接至网络 Socket。

## 3. 详细实施计划

### 阶段 3.1: Core 层重构与协议定义

- [ ] **统一模型**: 在 `core/src/models.rs` 中统一 `Session` (持久化记录) 和 `Instance` (运行时状态) 的字段定义，确保能被序列化 (Serde)。
- [ ] **RPC 定义**: 在 `core/src/rpc.rs` 中定义服务接口。
  ```rust
  #[tarpc::service]
  pub trait CmdHubService {
      async fn list_instances() -> Vec<InstanceInfo>;
      async fn spawn(task_id: String, env: HashMap<String, String>) -> Result<String, String>;
      async fn stop(instance_id: String) -> Result<(), String>;
  }
  ```
- [ ] **错误处理**: 定义统一的 `RpcError` 枚举。

### 阶段 3.2: Server 端实现 (The Brain)

- [ ] **守护进程化**: 修改 `server/src/main.rs`，使其启动一个 Tokio Runtime 并监听端口/Socket文件。
- [ ] **SessionManager 迁移**: 将 `cli/src/main.rs` 中的 `SessionManager` 逻辑完全迁移至 `server` crate。
- [ ] **实现 RPC Handler**: 实现 `CmdHubService` trait，对接 `SessionManager`。
- [ ] **PTY 流管理**: 
  - 当 Client 请求 `attach` 时，Server 需要升级连接或建立新连接。
  - 实现一个 `Broadcast` 机制，允许多个 Client 同时观察同一个任务的输出（类似直播）。

### 阶段 3.3: CLI 端重构 (The Remote)

- [ ] **连接管理**: 启动时检查 Server 是否存活。
  - 若存活：建立连接。
  - 若不存在：`Command::new("cmdhub-server").spawn()` 启动后台进程并等待连接。
- [ ] **UI 数据源替换**:
  - `App::refresh_instances()` 从调用本地函数改为 `client.list_instances().await`。
- [ ] **Attach 逻辑重写**:
  - 现有的 `run_passthrough` 直接操作本地 PTY FD。
  - 新逻辑需要操作 `TcpStream` 或 `UnixStream`，将网络字节流写入 `stdout`，将 `stdin` 读取发送至网络。

### 阶段 3.4: 验证与清理

- [ ] **多会话测试**: 打开两个 CLI 窗口，验证能否看到相同的任务列表和状态。
- [ ] **持久化测试**: 关闭 CLI，Server 保持运行；重新打开 CLI，任务仍在运行。
- [ ] **清理代码**: 移除 CLI 中残留的直接 PTY 依赖 (portable-pty 依赖应仅存在于 Server 和 Core)。

## 4. 进度追踪

### 3.1 Core Definition
- [ ] Move `InstanceInfo` to generic serializable model
- [ ] Define `CmdHubService` trait (tarpc)

### 3.2 Server Implementation
- [ ] Setup `tokio` server scaffold
- [ ] Port `SessionManager` to Server
- [ ] Implement `list` and `spawn` RPC methods

### 3.3 CLI Adaptation
- [ ] Implement Server auto-discovery/launch
- [ ] Replace `App` state management with RPC calls
- [ ] Implement Networked `Attach` (Passthrough)

## 5. 关键注意事项

1.  **版本兼容性**: 确保 Core 版本变更时，Client 和 Server 能检测版本不匹配。
2.  **安全性**: 暂时仅监听 Localhost (127.0.0.1) 或 用户级 Unix Socket (`~/.cmdhub/cmdhub.sock`)，避免暴露给公网。
3.  **日志**: Server 的 stdout/stderr 应该重定向到文件 (`~/.cmdhub/server.log`) 以便调试，因为它是后台运行的。
