# 扩展 AI 工具接入

EchoIsland 0.7.0 默认使用原生 Rust Adapter 读取 Codex CLI / Codex App；Claude Code 和 OpenClaw 使用受管 Hook / 插件。此目录保留 Kimi CLI、Antigravity、ZCode 的社区接入能力。

| 接入方式 | 能力 | 边界 |
| --- | --- | --- |
| Codex 原生扫描 | 会话、标题、最近消息、任务活动 | 只读 SQLite 和会话文件，支持 WAL 更新；工具审批仍在 Codex 内进行 |
| OpenClaw 受管插件 | 会话、消息、工具调用、批准 / 拒绝 | 需在 OpenClaw 中加载插件；连接失效不会替代 OpenClaw 自己的安全控制 |
| Kimi / Antigravity Watcher | 观察近期活动与闲置、显示会话消息或标题 | 只发送非阻塞通知，不接管审批；活跃窗口 120 秒，忽略超过 2 小时的历史会话 |
| ZCode Bridge | 转发非阻塞 Hook 事件 | 不转发审批或提问事件；不执行事件提供的命令 |
| Gemini / GLM / VS Code / Cursor / Trae 进程发现 | 识别运行中的程序 | 进程存在不代表任务运行；Windows 的 npm `node.exe` 启动方式不保证被识别 |

在 Windows 上，从完整仓库运行 `setup-integrations.ps1` 会部署 Watcher、ZCode 脚本和中文目录，并创建当前用户的 Watcher 自启快捷方式。脚本排除 WindowsApps 的 Python 别名。OpenClaw 需要从仓库运行 `cargo run -p desktop-host -- install-openclaw` 显式安装受管插件，然后重启 OpenClaw 使其加载。

迁移电脑时应重新运行配置脚本。OpenClaw 使用当前用户的运行目录，已去除固定用户名。单独分发此目录时，需同时复制 `crates/i18n/locales` 到本目录的 `locales`。

Watcher 默认关闭旧 Codex App 扫描，避免与原生扫描重复。仅在配合缺少原生扫描的旧 EchoIsland 时，可设置 `ECHOISLAND_WATCHER_CODEX_APP=1`。升级后应重新部署 Watcher 脚本。

推送成功后才保存去重状态；推送失败、文件未写完整或数据库锁定时，下次轮询重试。接收地址只允许本机 HTTP 回环 `/event`，禁止跟随重定向；每次请求读取最新 Token。OpenClaw 普通事件最长等待 3 秒，审批 / 提问最长等待 5 分钟。

回归测试：

```powershell
python -m unittest discover -s integrations/tests -p "test_*.py" -v
node --test integrations/tests/openclaw.test.mjs
cargo test -p echoisland-adapters -p echoisland-core -p echoisland-hook-bridge --locked
```

测试使用临时目录和本地模拟服务，不修改真实 Agent 配置。
