# 阶段一：核心原型 (Week 1-2)

## 目标
实现 PTY 封装与 CLI 基本交互。

## 详细任务
- [x] 初始化 Rust 项目结构 (Workspace)
- [x] 实现 `core` 库：配置解析与模型 (TOML)
- [x] 实现 `core` 库：PTY 封装与生命周期管理 (使用 `portable-pty`)
- [x] 实现 `cli` 二进制：基于 `Ratatui` 的简单交互界面
- [x] 实现命令执行并实时输出到 Ratatui 界面
- [x] 验证 PTY 字节流到 UTF-8 的转换与展示
- [x] 优化 CLI：支持日志自动换行与手动滚动 (PageUp/PageDown)
- [x] 优化 CLI：支持通过 Esc 或 Ctrl+C 中止运行中的命令

## 交付物
能够执行简单命令并输出到 Ratatui 界面的 CLI 程序。
