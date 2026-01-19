# CmdHub
一个跨平台的通用命令行程序管理界面

## 快速开始

```bash
cargo run -p cmdhub-cli
```

客户端默认读取工作目录 `ch-client.toml` 或全局 `$HOME/.cmdhub/client.toml`，支持在 TUI 内多实例启动、切换与透传运行。

服务器默认读取工作目录 `ch-server.toml` 或全局 `$HOME/.cmdhub/server.toml`，任务列表由服务器提供。
