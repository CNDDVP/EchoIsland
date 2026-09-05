# EchoIsland 简体中文增强版

[English](README.en.md) · [上游项目](https://github.com/FunplayAI/EchoIsland) · [下载发布版本](https://github.com/CNDDVP/EchoIsland/releases) · [迁移与接入指南](MIGRATION_GUIDE.md)

面向中国 Windows 用户的轻量 AI Coding Agent Control Center：把多个 AI 编码工具的工作状态、批准请求、问题和完成提醒集中到桌面顶部的原生灵动岛，帮助你回到对应的工作现场。

本仓库由 **CNDDVP 社区维护**，基于 [FunplayAI/EchoIsland](https://github.com/FunplayAI/EchoIsland) 开发，遵循 [MIT License](LICENSE)，保留原作者版权声明。技术底座持续跟进上游，同时维护完整简体中文和 Windows 增强能力。

| 版本信息 | 内容 |
| --- | --- |
| 当前开发版本 | `0.7.0`，CN 发布标签约定 `v0.7.0-cn` |
| 技术基线 | upstream `a7ebfff`（上游清单版本 `0.6.1`） |
| 主要平台 | Windows 10 / 11；macOS 跟随上游，验证状态见版本报告 |
| 架构 | Rust Core / Adapter / Runtime / IPC + Tauri / Win32 / Direct2D / DirectWrite |

## 中文版能力

- 默认简体中文的原生卡片、批准与回复操作、托盘、设置、更新提示和安装器；代码及协议标识保持原样。
- 最多展示 16 个会话，中文与长工具名称徽章自适应，多位任务计数完整显示。
- 依据所选显示器的物理边界和缩放定位，面向混合 DPI、多屏及负坐标桌面。
- 观察型接入 Kimi、Antigravity 和 ZCode；保留近期会话过滤、真实标题与发送失败重试。
- NSIS / MSI 当前用户安装与自启；独立便携版；更新入口仅指向 CNDDVP。

## AI 工具支持边界

| 工具 / 来源 | 能力 |
| --- | --- |
| Codex CLI / Codex App | 会话扫描、标题/上下文、Hook 配置与工作现场返回；实际 Hook 能力受工具版本影响 |
| Claude Code | 会话扫描、Hook、批准/问题/完成事件 |
| OpenClaw | 本地接收器、会话事件、插件工具批准；按接入指南显式安装 |
| Kimi / Antigravity / ZCode | CN 观察型桥接；不代表已支持所有实时审批和回复协议 |
| Gemini / GLM / VS Code / Cursor / Trae | 上游来源注册和进程发现；Windows 通用 node.exe 进程不一定能识别具体工具 |
| OpenCode / Continue / Aider 等 | 后续按 Adapter 能力扩展，不把路线图描述为现有完整接入 |

Agent 或用户自己生成的内容按原文展示。英文产品名、命令、路径、URL、错误码和协议字段不翻译。

## 开发与验证

需要 Rust stable（含 rustfmt、clippy）、Node.js 22 或更新版、npm，以及 Windows MSVC C++ Build Tools / Windows SDK。

```powershell
npm ci
npm run desktop:dev
```

请使用 npm 包装脚本启动和打包：它会先构建 `echoisland-hook-bridge` 并复制到 Tauri 资源目录，避免干净检出时因缺资源失败。仅检查 Rust 时也需先准备 bridge：

```powershell
cargo build -p echoisland-hook-bridge
Copy-Item target/debug/echoisland-hook-bridge.exe apps/desktop/src-tauri/resources/echoisland-hook-bridge.exe
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets --no-deps -- -D warnings
cargo test --workspace
npm run check
```

构建 Windows 安装器或便携版：

```powershell
npm run desktop:build
npm run desktop:build:portable
```

NSIS/MSI 输出目录由构建器日志给出。便携版位于 `apps/desktop/dist/`，需要一起携带 `EchoIsland.exe`、`echoisland-hook-bridge.exe` 和 `EchoIsland.portable`。

## 更新与维护

只从 [CNDDVP Releases](https://github.com/CNDDVP/EchoIsland/releases) 安装本增强版。CN updater 元数据或可信签名不可用时采用手动更新，不从上游发布页下载覆盖中文版。开发分支版本号不代表该版本已经发布。

- [中文变更记录](CHANGELOG.zh-CN.md)
- [本轮 0.7.0 优化报告](technical-notes/OPTIMIZATION_REPORT_2026-09-05.zh-CN.md)
- [完整上游差异报告](technical-notes/UPSTREAM_AUDIT_2026-09-05.zh-CN.md)
- [Fork Patch Ledger](technical-notes/CN_FORK_PATCHES.md)
- [简体中文覆盖审计](technical-notes/ZH_CN_COVERAGE_AUDIT.md)
- [技术说明](technical-notes/README.zh-CN.md)

同步采用共同祖先、上游、Fork 三方审计及语义合并，先检查保护补丁，再测试和构建。上游检查工作流只生成差异产物，不自动合并 main。真实硬件矩阵、安装验证和平台限制以本轮验证报告为准。
