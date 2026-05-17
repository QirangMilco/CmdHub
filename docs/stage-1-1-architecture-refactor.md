# Stage 1.1: 客户端-服务端架构重构方案 (Client-Server Refactoring)

## 1. 目标概述

当前 CmdHub 已完成 CLI/Server 拆分与 RPC/Attach 接入，但持久化、版本兼容、安全与后台守护仍未完全落地。本文档更新为“现状对齐 + 待补齐清单”。

- **CmdHub Server (Daemon)**: 负责进程托管、PTY 管理、状态维护、日志持久化。作为系统后台服务运行。
- **CmdHub Core**: 定义通用的数据模型、RPC 接口契约、以及核心逻辑（Session/Storage）。
- **CmdHub CLI (Client)**: 纯 UI 展示层。负责发送指令（启动/停止任务）和渲染从 Server 流式传输过来的数据。

## 2. 架构设计（现状对齐）

### 2.1 模块职责划分

| 模块 | 职责与现状 |
| :--- | :--- |
| **core** | 已有 `rpc` 模块和 `RpcError`，模型中同时存在 `InstanceInfo` 与 `SessionInfo`；目前运行时仅使用 `InstanceInfo`，`SessionInfo` 未接入持久化链路。 |
| **server** | 已独占 `SessionManager`，提供 tarpc RPC 监听（Unix/TCP），处理 PTY IO 与 attach 流，支持多客户端广播。 |
| **cli** | 已移除 `SessionManager`，启动时尝试连接 Server，失败时可自动启动本地 Server；TUI 数据源改为 RPC 调用。 |

### 2.2 通信协议策略

- **控制平面 (Control Plane)**: 已采用 tarpc RPC 处理低频交互。
  - `list_tasks()` -> `Vec<Task>`
  - `list_instances()` -> `Vec<InstanceInfo>`
  - `spawn(task_id, params)` -> `instance_id`
  - `stop(instance_id)`
  - `remove_if_exited(instance_id)`
  - `shutdown()`
- **数据平面 (Data Plane)**: 使用 Raw UDS/TCP Stream。
  - `attach(instance_id)` 通过首行发送 `instance_id`，返回 `OK\n`/`ERR\n`，随后双向转发流。
  - Server 端维护输出 ring buffer，attach 后先回放快照，再转发实时输出；支持多 client 订阅同一实例输出。

## 3. 实施计划与完成度

### 3.1 Core 层重构与协议定义

- [ ] **统一模型**: `SessionInfo` 与 `InstanceInfo` 尚未打通，持久化模型未接入运行时。
- [x] **RPC 定义**: `core/src/rpc.rs` 已定义 `CmdHubService` trait。
- [x] **错误处理**: 已提供 `RpcError` 枚举。
- [ ] **历史记录 API**: `get_history()` 仍缺失。

### 3.2 Server 端实现 (The Brain)

- [x] **Tokio server scaffold**: Server 启动 tarpc 监听并绑定 attach endpoint。
- [x] **SessionManager 迁移**: 运行态与 PTY 管理已迁移至 server。
- [x] **实现 RPC Handler**: 已实现 `list_tasks/list_instances/spawn/stop/remove_if_exited/shutdown`。
- [x] **PTY 流管理**: attach 双向转发 + 广播机制已实现。
- [ ] **守护进程化**: Server 本体仍是前台进程，需补齐 daemon 化与自管理。
- [ ] **日志重定向**: 仅 CLI 启动 server 时重定向至 `~/.cmdhub/server.log`，手动启动未覆盖。
- [ ] **持久化 Session/日志**: `SessionStore` 未接入，重启后状态无法恢复。

### 3.3 CLI 端重构 (The Remote)

- [x] **连接管理**: 支持连接现有 Server，必要时自动启动本地 Server。
- [x] **UI 数据源替换**: 通过 RPC 拉取 tasks/instances。
- [x] **Attach 逻辑重写**: 通过 Unix/TCP stream 与 Server 直连。

### 3.4 验证与清理

- [ ] **多会话测试**: 多个 CLI 同时观察任务输出需补充验证。
- [ ] **持久化测试**: Server 重启后的任务恢复未实现。
- [x] **清理代码**: CLI 不再依赖 portable-pty，PTY 仅保留于 core/server。

## 4. 进度追踪（更新版）

### 3.1 Core Definition
- [x] Define `CmdHubService` trait (tarpc)
- [x] Add `RpcError`
- [ ] Unify `SessionInfo` / `InstanceInfo`
- [ ] Add history APIs

### 3.2 Server Implementation
- [x] Setup tokio server scaffold
- [x] Port `SessionManager` to server
- [x] Implement RPC methods
- [ ] Daemonize + log redirection
- [ ] Persist session/logs

### 3.3 CLI Adaptation
- [x] Server auto-discovery/launch
- [x] RPC-based `App` state
- [x] Networked `Attach`

## 5. 关键注意事项（仍待实现）

1. **版本兼容性**: Client/Server 需增加版本握手或协议版本字段。
2. **安全性**: TCP 监听需限制为 localhost 或加入鉴权；默认推荐 UDS。
3. **日志/守护**: Server 本体需支持后台运行与 stdout/stderr 统一落盘。
4. **持久化**: 使用 `SessionStore` 持久化元数据与输出日志，提供历史检索 API。
