# EchoIsland 简体中文增强版迁移与接入指南

维护仓库：[CNDDVP/EchoIsland](https://github.com/CNDDVP/EchoIsland)。本轮开发版本 `0.7.0`（标签约定 `v0.7.0-cn`），技术基线为 FunplayAI upstream `a7ebfff`。是否发布以 CNDDVP Release 页为准。

## 从 0.6.3-cn 升级

1. 退出旧版，备份 `%LOCALAPPDATA%\EchoIsland` 中的设置和会话文件，以及 `%USERPROFILE%\.echoisland`。token 是本机凭据，不应公开或跨电脑复用。
2. 从 [CNDDVP Releases](https://github.com/CNDDVP/EchoIsland/releases) 下载增强版。NSIS 与 MSI 二选一；不要同时安装到同一目录。
3. NSIS 使用当前用户安装与自启。新 MSI 使用当前用户安装；旧管理员级 MSI 的安装上下文不同，先通过 Windows“已安装的应用”卸载旧 MSI，再安装新版。
4. 启动新版，检查显示器选择、提示音、吉祥物、最近会话和返回终端。配置路径与协议标识保留。
5. 扩展工具按下文接入。Rust 已读取 Codex App 会话时，Python watcher 默认不再重复扫描，避免覆盖状态。

更新入口和 updater 元数据都限定为 CNDDVP。metadata 或可信签名缺失时，按发布页说明手动更新。便携版始终手动替换完整文件集合，不注册安装器自启。

## 安装格式

| 格式 | 用途 |
| --- | --- |
| `EchoIsland_*_x64-setup.exe` | 推荐的简体中文 NSIS，当前用户安装、HKCU 自启、卸载清理 |
| `EchoIsland_*_x64_zh-CN.msi` | 简体中文 MSI，适合 MSI 分发；当前用户自启 |
| 便携版 | 同目录保留 EchoIsland.exe、echoisland-hook-bridge.exe、EchoIsland.portable；不单独复制主程序 |

## AI 工具接入

Codex 和 Claude Code：启动时在检测到各自配置目录后安装或修复 EchoIsland Hook，保留其他配置。工具版本和 Hook 支持决定实际事件能力。Codex App 使用 Rust 扫描器读取本机会话索引和日志。

Kimi、Antigravity、ZCode 等观察型工具：在源码目录运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\integrations\setup-integrations.ps1
```

脚本配置当前用户的观察桥接。需要真实 Python；WindowsApps 中跳转微软商店的 python.exe/pythonw.exe 不可用。脚本成功不表示工具具备批准或问题回复功能，请按实际 Adapter 能力判断。

OpenClaw：使用项目安装入口显式安装插件。接收器状态和 token 从当前用户路径读取，不复制其他电脑用户名、token 或硬编码端口。

```powershell
cargo run -p desktop-host -- install-openclaw
```

源码环境需先构建 hook bridge；若找不到桥接程序，用 `--bridge` 指定本轮构建的可执行文件。

## 多显示器与排查

- 设置中选择目标显示器，程序使用该屏物理边界和 DPI。屏幕排列变化后重新检查选择。
- watcher 仅在推送成功后记录去重状态，接收器短暂不可用时在后续轮询重试。
- 会话缺失时检查工具运行状态、会话路径、bridge/Hook、本地接收器和日志中的连接错误。
- 进程发现不等于实时会话或审批能力；Windows 通用 node.exe 无法可靠区分所有 npm Agent。

## 验证说明

旧迁移说明曾把纯坐标单测描述成完整混合 DPI 验证。本轮区分：自动测试验证目标屏数据、坐标数学和保护逻辑；真实双屏、缩放切换、睡眠恢复、Explorer 重启及安装升级卸载必须有独立运行证据，以本轮优化报告为准。

开发者同步前阅读 [上游审计](technical-notes/UPSTREAM_AUDIT_2026-09-05.zh-CN.md) 与 [Patch Ledger](technical-notes/CN_FORK_PATCHES.md)，不得以上游覆盖 Fork 来消解冲突。
