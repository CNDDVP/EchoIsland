# 简体中文覆盖审计报告

审计日期：2026-09-05，最终验收完成于 2026-09-06。范围：EchoIsland v0.7.0 升级工作区；结论综合源码、词库、自动测试、Windows 构建产物、真实混合 DPI 运行数据和 100% 缩放展开态像素截图。该版本尚未发布；其他缩放率视觉矩阵、干净环境安装生命周期与 macOS 仍属明确的未验证范围。

## 结论与方法

默认语言为 zh-CN。已建立共享 `echoisland-i18n` 薄层，中英词库位于 `crates/i18n/locales/`；Rust 编译时嵌入、首次访问解析一次，无网络翻译或每帧磁盘读取。Node 构建脚本与外部集成使用同源 JSON，用户消息、命令、路径和协议值按原样传递。补齐原生审批卡片、等待操作说明、显示器回退名、宽度选项、CLI、错误边界、插件描述和集成通知文字，并统一“需要批准 / 等待回复 / 已完成”。

扫描覆盖 apps、crates、integrations、technical-notes、安装器和 Tauri 配置中的 Rust、HTML、JS/TS、JSON、PowerShell、Python、NSIS/WiX 及 Markdown。构建生成的第三方 schemas、node_modules、target、测试夹具不计入产品文案；测试中的英文用户内容和协议样例必须保留。

可重复校验：

```powershell
node scripts/check-i18n.mjs
node scripts/check-i18n.mjs --scan
node scripts/check-i18n.mjs --json
cargo test -p echoisland-i18n -p desktop-host
cargo test -p echoisland-desktop native_panel_scene::tests
cargo test -p echoisland-desktop directwrite
```

默认检查验证中英文 key、非空译文、模板参数、真实调用缺失词条、zh-CN 页面元数据及 DirectWrite 字体配置。--scan 返回需要人工判读的英文候选，不把技术日志或协议词当作必须翻译的界面；--json 输出完整“位置 / 英文 / 中文 / 完成状态 / 协议影响”清单。脚本不是自动证明零漏译的工具，新增交互仍须结合调用路径与实际界面审阅。

## 重点覆盖结果

| 位置 | 英文原文 | 中文翻译或处理 | 使用场景 | 是否完成 | 是否影响协议 |
| --- | --- | --- | --- | --- | --- |
| native_panel_renderer/card_visual_spec.rs | Approval Required；Approval | 需要批准 | 默认审批卡片、状态卡片 | 源码已完成 | 否 |
| native_panel_renderer/presentation_model.rs | A command may be waiting for approval in the Codex terminal. Allow or deny it there. | Codex 终端中可能有命令等待批准，请在那里允许或拒绝。 | 返回终端提示 | 源码已完成 | 否 |
| native_panel_scene 与 native_panel_core | Question；Completed；Running；Thinking；Idle | 等待回复；已完成；运行中；思考中；空闲 | 状态、标题、卡片、相对时间 | 源码已完成 | 仅展示，不改枚举和 status 值 |
| display_settings.rs；macos_native_panel/panel_display_source.rs | Display {number} | 显示器 {number} | 无名称显示器回退 | 源码已完成 | Display|x|y|w|h key 保留 |
| native_panel_core/settings.rs | S / M / L | 紧凑 / 标准 / 宽 | 宽度设置选项 | 源码已完成 | Compact/Standard/Wide 不变 |
| tray.rs | Show EchoIsland；Refresh snapshot；Quit | 显示 EchoIsland；刷新快照；退出 | 托盘菜单 | 已接入共享词库 | tray_show 等 ID 不变 |
| updater_service.rs | Update failed；raw updater errors | 更新失败；中文手动更新说明 | 更新状态和失败提示 | 展示边界已完成；安全策略另有专门测试 | URL/phase/key 不变 |
| commands.rs；command_services.rs；terminal_focus_service.rs | failed to open / parse / read；session not found | 无法打开/解析/读取；未找到会话；中文操作建议 | Tauri 命令失败 | 源码已完成，原始技术详情留诊断日志 | command 名和参数不变 |
| app_runtime.rs；http_receiver.rs | raw IPC / HTTP errors；Completed / Failed | 本地通信/HTTP 接收器中文失败说明；Agent 完成结果为“已完成 / 执行失败” | ipc-error 事件、插件事件生成的完成消息 | 源码与真实解析路径测试已完成 | ipc-error、HTTP reason phrase、事件/status 值不变 |
| apps/desktop-host/src/main.rs | unknown arg / command；missing value after | 未知参数/命令；参数后缺少值 | CLI 操作失败 | 源码和缺参 smoke 已完成 | CLI 参数、JSON 输出结构不变 |
| apps/desktop/scripts/run-tauri.mjs | Usage；Unsupported mode；build success/failure | 用法；不支持的模式；中文构建结果 | 构建入口 | 源码和语法检查已完成 | dev/build/portable 不变 |
| apps/desktop/web/index.html | lang=en | lang=zh-CN | Native placeholder 页面语言 | 已完成；按上游架构保持空 Web 宿主 | 无 |
| integrations、hook-bridge 与 adapters | Denied by EchoIsland；Open URL；Choose one value；Antigravity conversation activity；Feishu group/direct | 中文拒绝原因、链接/表单指引、会话活动、飞书群聊/私聊和安装说明 | 插件、Watcher 卡片、Hook 问题与 CLI 状态说明 | 源码已接入共享词库 | Hook/事件、JSON key、来源 ID 与 Agent 原文不变 |
| tauri.windows.conf.json 与 installer | Tauri WiX 扩展文案默认英文 | NSIS SimpChinese；WiX zh-CN 标准 UI；专用 zh-CN.wxl；中文快捷方式描述 | 安装、卸载 | 配置及构建产物语言表已完成；真实安装流程另验 | 注册表、Component ID 和程序路径不变 |
| tauri.conf.json | 缺少 Fork 中文说明 | 简体中文增强版、社区维护简介 | 应用包元数据 | 配置源码已完成 | productName/identifier 不变 |
| README / MIGRATION_GUIDE / technical-notes | 默认英文入口、上游身份说明 | 中文主入口、保留英文文档、Fork 维护说明 | 用户文档 | 由主升级报告核对 | 无 |

## 字体与布局验证边界

- DirectWrite 主字体改为系统 `Microsoft YaHei UI`，locale 为 `zh-CN`，未打包大型字体。图标继续使用 `Segoe MDL2 Assets`；其他字形交由 DirectWrite 系统 fallback。fallback 字段本身不是自定义多字体渲染链，不能据此宣称任意电脑都已验证 Emoji。
- 原生卡片已有按字符宽度估算、最大宽度和省略布局。共享场景 29 项测试通过，覆盖审批、问题、完成、显示器、宽度、设置和值标签等输出；字体工厂可在本 Windows 环境初始化。
- Windows 11 真实双屏运行已验证目标屏选择和物理定位：主屏 3840×2160、144 DPI（150%）得到 `1605,0,630,120`；副屏 1920×1080、96 DPI（100%）、起点 `3840,0` 得到 `4590,0,420,80`。两者都在所选屏幕物理顶部居中，窗口尺寸按目标屏 DPI 换算。测试同时捕获并修复了 payload/renderer 刷新路径丢失目标屏物理上下文的问题。
- 最终发行版在 1080p 100% 副屏经真实 hover 从 420×80 展开到 420×876；直接屏幕捕获同时显示中文、English、1–16 数字、多位总数、Agent 徽章、绿色“运行中”和 Emoji。原始 420×876 PNG 为 `E:\本地项目\EchoIsland-toolchain\qa-screenshots\EchoIsland-0.7.0-zh-CN-expanded-1080p-100.png`，SHA-256 为 `5ef455cfdc50239ba18217a80d901b6c6eb499d5053f5fc6599b30a0ad11aa5f`。目视检查未见乱码、方框、重叠或窗口越界；长标题按设计显示省略号。
- 本报告仍不宣称已通过：负坐标/竖屏的真实硬件视觉、125%/150%/200% 展开态文字无遮挡、睡眠恢复或 Explorer 重启、全新用户安装/升级/卸载。这些需要相应硬件或干净虚拟机；坐标与缩放组合已有数值回归，但不能替代全部像素矩阵。
- 标题 `compact_title`、预览 `display_snippet`、Agent API 的 80 字标题及渲染层换行/省略现通过共享 `echoisland_i18n::text` 按扩展字素簇（grapheme）处理，保留原来的长度预算、中部 58% 头尾分配和各自省略号风格；零预算安全返回空文本。清理预览 Markdown 时只删除独立标记，保留 `*️⃣` 等完整 Emoji。CJK、拉丁字母、ZWJ 家族/职业 Emoji、肤色修饰、国旗、组合重音与逐宽度边界已通过测试。原始 Agent 消息和协议字段不修改；真实字形 fallback 仍需硬件截图验证。
- 当前上游 Native renderer 未提供完整 UI Automation/无障碍语义树，因此没有可承诺已覆盖的独立原生 accessibility label 集合。

## 有意保留的英文与剩余工作

| 类别与位置 | 保留原因 / 当前状态 | 是否需要翻译 |
| --- | --- | --- |
| Core/IPC/HTTP：session_id、source、Unauthorized、handler_cancelled、Bearer、HTTP reason phrase 等 | 协议字段/错误码/报文；中文只在展示边界生成 | 否，翻译会影响兼容性 |
| CLI argument、环境变量、URL、路径、Window/Agent/Session ID | 程序标识及用户真实数据 | 否 |
| terminal_focus 的进程名、窗口匹配词、AppleScript 的 System Events、Windows 图标字体名 | 系统 API/终端匹配需要精确字符串 | 否 |
| installer/main.wxs 中 Desktop Shortcut / Uninstaller Shortcut / Start Menu Shortcut | HKCU RegistryValue 名，是升级/卸载追踪标识，不是按钮文案 | 否 |
| resources/mascot/default/pet.json 的 `EchoIsland 白色助手` | 资源清单元数据已中文化，产品名 EchoIsland 保留 | 已完成；若新增选择器继续复用该中文名 |
| tracing/diagnostics、adapter/parser/persistence 内部 error context、hook bridge 日志 | 技术排障证据；可见桌面/CLI错误边界提供中文概括 | 不批量翻译；新边界需复查 |
| Agent 消息、用户问题、工具名、实际项目名和终端标题 | 属于外部内容，必须保留原文 | 否 |
| README.en.md、*.en.md、代码示例和测试样例 | 明确保留的英语文档和技术内容 | 否 |
| NSIS/MSI 原生安装/升级/卸载、实际 GitHub Release 页面 | NSIS/MSI 已构建并审阅语言表、安装范围、HKCU 自启及卸载元数据；干净用户上的安装/升级/卸载生命周期和 Release 页面尚未执行 | 产物静态审阅完成；生命周期待干净环境验证 |
| 原生 UIA 标签与完整硬件视觉矩阵 | 100% 展开态已有像素证据；Native renderer 仍未提供完整 UI Automation 语义，其他缩放/布局需扩展视觉矩阵 | 继续跟踪已列明的边界 |

## 本轮已执行验证

- `node scripts/check-i18n.mjs`：137 个中英词条、252 个生产文件，检查通过；新增 `completion.failed` 覆盖插件事件生成的失败消息。
- `cargo test -p echoisland-i18n -p desktop-host`：共享词库 2 项测试通过，CLI 编译通过。
- `cargo test -p echoisland-desktop native_panel_scene::tests`：29 项通过。
- `cargo test -p echoisland-desktop directwrite`：7 项通过，包含 Windows DirectWrite 工厂初始化和 painter 路由。旧字体断言已更新后复测通过。
- `node --check apps/desktop/scripts/run-tauri.mjs`：通过。
- `node apps/desktop/scripts/run-tauri.mjs invalid-mode`：输出“不支持的模式：invalid-mode”，退出码 1。
- `cargo run --quiet -p desktop-host -- send --addr`：输出“--addr 后缺少参数值”，退出码 1。

- Unicode 追加验证：共享 `text` 模块 4 项、Core 标题投影 2 项、Native queue 2 项共 8 个新增测试通过；i18n 全部 6 项通过。`unicode-segmentation` 复用锁定的 1.13.2，只为 i18n 增加依赖关系。

## 全量共享词条清单

下表“已完成”指词条与展示调用层，不能替代上述安装器和硬件验收。自动定位同时涵盖 Rust 和脚本调用；“共享词库”表示动态选择或预留错误回退。所有 key 都是内部英文标识，只有值参与本地化。

| 位置 / Key | 英文原文 | 中文翻译 | 使用场景 | 是否完成 | 是否影响协议 |
| --- | --- | --- | --- | --- | --- |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:361<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:134<br>`action.open_terminal` | Open terminal to review | 打开终端查看 | action | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:351<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:131<br>`action.review` | Review | 查看 | action | 词条完成 | 否 |
| crates/adapters/src/claude/install.rs:58<br>`adapter.claude_status` | Claude Code hooks are installed through ~/.claude/settings.json. Project-local settings may still override global behavior. | Claude Code 通过 ~/.claude/settings.json 安装 Hook；项目设置可能覆盖全局设置。 | adapter | 词条完成 | 否 |
| crates/adapters/src/codex/install.rs:199<br>`adapter.codex_features_table` | Codex features must be a table; the original file was not modified | Codex 配置的 features 必须是表，未修改原文件 | adapter | 词条完成 | 否 |
| crates/adapters/src/codex/install.rs:193<br>`adapter.codex_invalid_toml` | Codex configuration is not valid TOML; the original file was not modified | Codex 配置不是有效的 TOML，未修改原文件 | adapter | 词条完成 | 否 |
| crates/adapters/src/codex/install.rs:219<br>`adapter.codex_read_toml` | Failed to read Codex TOML configuration | 无法读取 Codex TOML 配置 | adapter | 词条完成 | 否 |
| crates/adapters/src/agent_sources.rs:20<br>`adapter.focus_unavailable` | Session focus is not supported for source: {source} | 此来源尚不支持定位会话：{source} | adapter | 词条完成 | 否 |
| crates/adapters/src/openclaw/install.rs:67<br>`adapter.openclaw_status` | OpenClaw uses a managed hook and local plugin for session, message, tool and approval events. | OpenClaw 通过受管 Hook 与本地插件接入；Hook 捕获会话和消息，插件加载后支持工具事件与 EchoIsland 审批。 | adapter | 词条完成 | 否 |
| apps/desktop/src-tauri/src/tray.rs:26<br>`app.quit` | Quit | 退出 | app | 词条完成 | 否 |
| apps/desktop/src-tauri/src/tray.rs:24<br>`app.refresh` | Refresh snapshot | 刷新快照 | app | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:76<br>`app.settings` | Settings | 设置 | app | 词条完成 | 否 |
| apps/desktop/src-tauri/src/tray.rs:23<br>`app.show` | Show EchoIsland | 显示 EchoIsland | app | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:893<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:415<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:56<br>`approval.badge` | Approval | 需要批准 | approval | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:292<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:409<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:451<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:477<br>`approval.required` | Approval Required | 需要批准 | approval | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:308<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:425<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:460<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:486<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:59<br>`approval.terminal` | Allow / deny in terminal | 在终端中允许 / 拒绝 | approval | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:43<br>`approval.tool` | Tool permission | 工具权限 | approval | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:305<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:422<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:45<br>`approval.waiting` | Waiting for your approval | 等待你的批准 | approval | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:96<br>`build.bridge_missing` | Hook bridge build succeeded but binary was not found: {path} | Hook 桥接程序构建成功，但未找到可执行文件：{path} | build | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:102<br>`build.bridge_ready` | Hook bridge prepared: {path} | Hook 桥接程序已准备完成：{path} | build | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:170<br>`build.portable_missing` | Portable build succeeded but binary was not found: {path} | 便携版构建成功，但未找到可执行文件：{path} | build | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:181<br>`build.portable_ready` | Portable executable created: {path} | 便携版程序已生成：{path} | build | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:119<br>`build.portable_windows` | Portable mode currently only supports Windows. | 便携模式目前仅支持 Windows。 | build | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:55<br>apps/desktop/scripts/run-tauri.mjs:188<br>`build.start_failed` | Failed to start {command}: {error} | 启动 {command} 失败：{error} | build | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:114<br>`build.unsupported_mode` | Unsupported mode: {mode} | 不支持的模式：{mode} | build | 词条完成 | 否 |
| apps/desktop/scripts/run-tauri.mjs:109<br>`build.usage` | Usage: node ./scripts/run-tauri.mjs <dev\|build\|portable> [...args] | 用法：node ./scripts/run-tauri.mjs <dev\|build\|portable> [...args] | build | 词条完成 | 否 |
| apps/desktop-host/src/main.rs:174<br>apps/desktop-host/src/main.rs:207<br>`cli.bridge_missing` | Bridge binary not found at {path}. Build it with cargo build -p echoisland-hook-bridge or pass --bridge <path>. | 未在 {path} 找到桥接程序。请先执行 cargo build -p echoisland-hook-bridge，或指定 --bridge <path>。 | cli | 词条完成 | 否 |
| apps/desktop-host/src/main.rs:112<br>`cli.input_required` | Use --file <path> or --stdin | 请使用 --file <path> 或 --stdin | cli | 词条完成 | 否 |
| apps/desktop-host/src/main.rs:82<br>apps/desktop-host/src/main.rs:90<br>apps/desktop-host/src/main.rs:157<br>apps/desktop-host/src/main.rs:190<br>`cli.missing_value` | Missing value after {argument} | {argument} 后缺少参数值 | cli | 词条完成 | 否 |
| apps/desktop-host/src/main.rs:58<br>`cli.operation_failed` | The operation failed. Check the command arguments, configuration files and local communication service. | 操作失败，请检查命令参数、配置文件和本地通信服务。 | cli | 词条完成 | 否 |
| apps/desktop-host/src/main.rs:98<br>apps/desktop-host/src/main.rs:164<br>apps/desktop-host/src/main.rs:197<br>`cli.unknown_argument` | Unknown argument: {argument} | 未知参数：{argument} | cli | 词条完成 | 否 |
| apps/desktop-host/src/main.rs:223<br>`cli.unknown_command` | Unknown command: {command} | 未知命令：{command} | cli | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:574<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:107<br>`completion.done` | Completed | 已完成 | completion | 词条完成 | 否 |
| apps/desktop/src-tauri/src/http_receiver.rs<br>`completion.failed` | Failed | 执行失败 | Agent 插件事件生成的失败消息 | 词条与真实解析路径完成 | 否；事件/status 值保持原文 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:1027<br>apps/desktop/src-tauri/src/native_panel_scene/build.rs:565<br>`completion.task_done` | Task completed | 任务完成 | completion | 词条完成 | 否 |
| apps/desktop/src-tauri/src/display_settings.rs:72<br>apps/desktop/src-tauri/src/macos_native_panel/panel_display_source.rs:84<br>apps/desktop/src-tauri/src/native_panel_scene/build.rs:425<br>`display.number` | Display {number} | 显示器 {number} | display | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:502<br>`empty.sessions` | No active sessions | 暂无活动会话 | empty | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/build.rs:572<br>`empty.tasks` | No active tasks | 暂无活动任务 | empty | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:508<br>`empty.watching` | EchoIsland is watching for new activity. | EchoIsland 正在监听新动态。 | empty | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:143<br>apps/desktop/src-tauri/src/commands.rs:149<br>apps/desktop/src-tauri/src/commands.rs:155<br>`error.adapter` | Could not read the AI tool configuration. Check the configuration file and permissions. | 无法读取 AI 工具配置，请检查配置文件和访问权限。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/windows_native_panel/directwrite.rs:136<br>`error.directwrite` | The text renderer is not initialized. | 文字渲染器尚未初始化。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:325<br>`error.display_index` | Display index is out of range: {index} | 显示器编号超出范围：{index} | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/display_settings.rs:101<br>`error.displays` | Could not read the display list. Reconnect the display and try again. | 无法读取显示器列表，请重新连接显示器后重试。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/terminal_focus_service.rs:150<br>apps/desktop/src-tauri/src/terminal_focus_service.rs:215<br>`error.focus` | Could not return to the terminal. Make sure the corresponding terminal is still open. | 无法返回终端，请确认对应终端仍处于打开状态。 | error | 词条完成 | 否 |
| 共享词库；动态调用或预留回退<br>`error.http` | The HTTP receiver could not start. Check local port availability and data folder permissions. | HTTP 接收器无法启动，请检查本地端口占用和数据目录权限。 | error | 词条完成 | 否 |
| 共享词库；动态调用或预留回退<br>`error.http_auth` | Could not initialize the HTTP receiver token. Check the EchoIsland data folder permissions. | 无法初始化 HTTP 接收器令牌，请检查 EchoIsland 数据目录的权限。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/tray.rs:32<br>`error.icon` | The default application icon could not be found. | 未找到应用的默认图标。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/command_services.rs:62<br>`error.invalid_sample` | The sample event is invalid. Check its protocol version, source and session ID. | 示例事件无效，请检查协议版本、来源和会话 ID。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/app_runtime.rs:135<br>`error.ipc` | The local communication service could not start. Check whether another EchoIsland instance is running. | 本地通信服务无法启动，请检查是否已有其他 EchoIsland 实例运行。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:272<br>apps/desktop/src-tauri/src/commands.rs:281<br>apps/desktop/src-tauri/src/commands.rs:292<br>apps/desktop/src-tauri/src/commands.rs:496<br>apps/desktop/src-tauri/src/commands.rs:507<br>apps/desktop/src-tauri/src/commands.rs:525<br>apps/desktop/src-tauri/src/commands.rs:540<br>`error.native` | Could not update the native panel. Please restart EchoIsland. | 无法更新原生面板，请重启 EchoIsland。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:393<br>apps/desktop/src-tauri/src/commands.rs:413<br>`error.open_settings` | Could not open the settings folder. Check your file permissions and try again. | 无法打开设置目录，请检查文件权限后重试。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:428<br>apps/desktop/src-tauri/src/commands.rs:448<br>`error.open_url` | Could not open the link. Open the CNDDVP release page manually. | 无法打开链接，请手动打开 CNDDVP 发布页。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/command_services.rs:59<br>`error.parse_sample` | Could not parse the sample event. Check its JSON format. | 示例事件解析失败，请检查 JSON 格式。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:183<br>apps/desktop/src-tauri/src/commands.rs:198<br>`error.permission` | This approval request is no longer available. Return to the terminal to review it. | 此批准请求已不可用，请返回终端查看。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:214<br>apps/desktop/src-tauri/src/commands.rs:229<br>`error.question` | This question is no longer available. Return to the terminal to answer it. | 此问题已不可用，请返回终端回答。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/command_services.rs:57<br>`error.read_sample` | Could not read the sample event file. Check that it exists and is readable. | 无法读取示例事件文件，请检查文件是否存在且可读取。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/command_services.rs:80<br>`error.sample_debug_only` | Sample ingest is only available in debug builds. | 示例事件导入仅在调试版本中可用。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/command_services.rs:87<br>`error.sample_name_empty` | Sample file name is empty. | 示例文件名不能为空。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/command_services.rs:94<br>`error.sample_name_invalid` | Invalid sample file name: {name} | 无效的示例文件名：{name} | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/terminal_focus_service.rs:62<br>apps/desktop/src-tauri/src/terminal_focus_service.rs:211<br>`error.session_missing` | Session not found: {id} | 未找到会话：{id} | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:303<br>apps/desktop/src-tauri/src/commands.rs:310<br>apps/desktop/src-tauri/src/commands.rs:316<br>apps/desktop/src-tauri/src/commands.rs:332<br>apps/desktop/src-tauri/src/commands.rs:343<br>`error.settings` | Could not save settings. Check your file permissions and try again. | 无法保存设置，请检查文件权限后重试。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/commands.rs:74<br>apps/desktop/src-tauri/src/commands.rs:104<br>`error.snapshot` | Could not read the session state. Please restart EchoIsland. | 无法读取会话状态，请重启 EchoIsland。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/terminal_focus_service.rs:216<br>`error.terminal_binding` | The foreground window is not a Windows Terminal tab that can be bound. | 当前前台不是可绑定的 Windows Terminal 标签页。 | error | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/build.rs:570<br>`headline.active` | {count} active tasks | {count} 个进行中任务 | headline | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/build.rs:544<br>`headline.approval` | Waiting for approval | 等待批准 | headline | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/build.rs:555<br>`headline.completed` | {count} tasks completed | {count} 个任务已完成 | headline | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/build.rs:542<br>`headline.question` | Waiting for an answer | 等待回答 | headline | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/build.rs:538<br>`headline.waiting` | Waiting for you | 等待处理 | headline | 词条完成 | 否 |
| 共享词库；动态调用或预留回退<br>`integration.antigravity_activity` | Antigravity conversation activity | Antigravity 会话有新活动 | integration | 词条完成 | 否 |
| crates/adapters/src/feishu/mod.rs:251<br>`integration.feishu_direct` | Feishu direct | 飞书私聊 | integration | 词条完成 | 否 |
| crates/adapters/src/feishu/mod.rs:250<br>`integration.feishu_group` | Feishu group | 飞书群聊 | integration | 词条完成 | 否 |
| crates/adapters/src/feishu/mod.rs:256<br>`integration.message_file` | File | 文件 | integration | 词条完成 | 否 |
| crates/adapters/src/feishu/mod.rs:255<br>`integration.message_image` | Image | 图片 | integration | 词条完成 | 否 |
| crates/adapters/src/feishu/mod.rs:257<br>`integration.message_other` | Message | 消息 | integration | 词条完成 | 否 |
| crates/adapters/src/feishu/mod.rs:254<br>`integration.message_text` | Text | 文本 | integration | 词条完成 | 否 |
| crates/adapters/src/openclaw/install.rs:824<br>integrations/openclaw-plugin/index.ts:167<br>`integration.openclaw_denied` | Denied by EchoIsland | 已被 EchoIsland 拒绝 | integration | 词条完成 | 否 |
| crates/adapters/src/openclaw/install.rs:637<br>crates/adapters/src/openclaw/install.rs:793<br>integrations/openclaw-plugin/index.ts:136<br>`integration.openclaw_description` | Forward OpenClaw runtime events to EchoIsland. | 将 OpenClaw 运行事件转发到 EchoIsland。 | integration | 词条完成 | 否 |
| integrations/openclaw-plugin/echoisland-http.mjs:36<br>`integration.receiver_loopback` | EchoIsland receiver must be a loopback HTTP /event URL | EchoIsland 接收地址必须是本机回环 HTTP /event 地址 | integration | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:133<br>`prompt.body` | You may need to approve something in the Codex terminal. | 可能需要在 Codex 终端中批准。 | prompt | 词条完成 | 否 |
| 共享词库；动态调用或预留回退<br>`prompt.meta` | {source} · Prompt | {source} · 提示 | prompt | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/presentation_model.rs:983<br>`prompt.review_body` | A command may be waiting for approval in the Codex terminal. Allow or deny it there. | Codex 终端中可能有命令等待批准，请在那里允许或拒绝。 | prompt | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:124<br>`prompt.title` | {source} needs attention | {source} 需要关注 | prompt | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:73<br>`question.input` | Your input is required | 需要你的输入 | question | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:894<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:320<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:439<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:447<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:497<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:603<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:86<br>`question.required` | Question | 等待回复 | question | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:336<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:619<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:156<br>`question.terminal` | Answer in terminal | 在终端回答 | question | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:333<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:616<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:75<br>`question.waiting` | Waiting for your answer | 等待你的回答 | question | 词条完成 | 否 |
| crates/core/src/agent.rs:147<br>`session.untitled` | Untitled session | 未命名会话 | session | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:81<br>`settings.display` | Display | 显示器 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:121<br>`settings.mascot` | Mascot | 吉祥物 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:110<br>apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:126<br>`settings.off` | Off | 关闭 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:108<br>apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:124<br>`settings.on` | On | 开启 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:105<br>`settings.sound` | Completion sound | 完成提示音 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_scene/settings_scene.rs:93<br>`settings.width` | Island width | 悬浮条宽度 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/settings.rs:142<br>`settings.width.compact` | S | 紧凑 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/settings.rs:143<br>`settings.width.standard` | M | 标准 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/settings.rs:144<br>`settings.width.wide` | L | 宽 | settings | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:873<br>`source.feishu` | Feishu | 飞书 | source | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:902<br>apps/desktop/src-tauri/src/native_panel_core/queue.rs:917<br>apps/desktop/src-tauri/src/native_panel_core/queue.rs:922<br>`source.session` | Session | 会话 | source | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:883<br>`source.unknown` | Unknown | 未知 | source | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:410<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:52<br>`status.approval_meta` | #{id} · Approval | #{id} · 需要批准 | status | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:895<br>`status.idle` | Idle | 空闲 | status | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:892<br>`status.processing` | Thinking | 思考中 | status | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:346<br>`status.prompt_meta` | {source} · Prompt | {source} · 提示 | status | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:321<br>apps/desktop/src-tauri/src/native_panel_renderer/card_visual_spec.rs:604<br>apps/desktop/src-tauri/src/native_panel_scene/status_card_scene.rs:82<br>`status.question_meta` | #{id} · Question | #{id} · 等待回复 | status | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:891<br>`status.running` | Running | 运行中 | status | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:896<br>`status.unknown` | Unknown status | 未知状态 | status | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:949<br>`time.days` | {count} days ago | {count}天前 | time | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:947<br>`time.hours` | {count} hours ago | {count}小时前 | time | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:942<br>`time.minutes` | {count} minutes ago | {count}分钟前 | time | 词条完成 | 否 |
| apps/desktop/src-tauri/src/native_panel_core/queue.rs:940<br>`time.now` | Just now | 刚刚 | time | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:81<br>`update.available` | Version {version} available | {version} 版本可用 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:39<br>apps/desktop/src-tauri/src/updater_service.rs:67<br>`update.check` | Check for updates | 检查更新 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:40<br>`update.check_action` | Check | 检查 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:53<br>`update.checking` | Checking for updates | 正在检查更新 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:54<br>`update.checking_action` | Checking... | 检查中… | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:95<br>`update.downloading` | Downloading update | 正在下载更新 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:96<br>`update.downloading_action` | Downloading... | 下载中… | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:137<br>`update.failed` | Update failed | 更新失败 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:313<br>apps/desktop/src-tauri/src/updater_service.rs:349<br>apps/desktop/src-tauri/src/updater_service.rs:359<br>apps/desktop/src-tauri/src/updater_service.rs:389<br>`update.failure_help` | The update could not be completed. Open the CNDDVP release page to download the update manually. | 更新未能完成。请打开 CNDDVP 发布页手动下载更新。 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:82<br>`update.install_action` | Install | 安装 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:123<br>`update.installed` | Update installed | 更新已安装 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:109<br>`update.installing` | Installing update | 正在安装更新 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:110<br>`update.installing_action` | Installing... | 安装中… | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:68<br>`update.latest` | Up to date | 已是最新 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:209<br>apps/desktop/src-tauri/src/updater_service.rs:333<br>`update.manual_signing` | A dedicated CNDDVP update signing key is not configured. Please update from the release page. | 本版本尚未配置 CNDDVP 专用更新签名，请从发布页手动更新。 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:229<br>`update.portable` | Portable version | 便携版 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:233<br>`update.portable_help` | Portable versions require a manual update download. | 便携版需要手动下载更新。 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:138<br>apps/desktop/src-tauri/src/updater_service.rs:230<br>`update.release_page` | Open release page | 打开发布页 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:124<br>`update.restarting` | Restarting | 正在重启 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:220<br>apps/desktop/src-tauri/src/updater_service.rs:269<br>`update.status_unavailable` | Update status is unavailable. Please try again. | 更新状态暂时不可用，请重试。 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/updater_service.rs:207<br>`update.untrusted_source` | The update URL is not a CNDDVP release. Please update manually. | 更新地址不属于 CNDDVP 发布页，请手动更新。 | update | 词条完成 | 否 |
| apps/desktop/src-tauri/src/windows_native_panel/platform_loop.rs:498<br>`window.native_panel` | EchoIsland Native Panel | EchoIsland 灵动岛 | window | 词条完成 | 否 |
| apps/hook-bridge/src/main.rs:643<br>`hook.choose_one` | Choose one value for {field}: {values} | 请为 {field} 选择一个值：{values} | hook | 词条完成 | 否；字段标签和值保持原文 |
| apps/hook-bridge/src/main.rs:503<br>`hook.denied` | Denied by EchoIsland approval workflow | 已被 EchoIsland 批准流程拒绝 | hook | 词条完成 | 否；decision 结构不变 |
| apps/hook-bridge/src/main.rs:435<br>`hook.open_url` | Open URL: {url} | 打开链接：{url} | hook | 词条完成 | 否；URL 原样传递 |
| apps/hook-bridge/src/main.rs:648<br>`hook.provide_value` | Provide a value for {field}. | 请为 {field} 提供一个值。 | hook | 词条完成 | 否；字段名保持原文 |
| apps/hook-bridge/src/main.rs:656<br>`hook.reply_json` | Reply with JSON containing fields: {fields} | 请回复包含这些字段的 JSON：{fields} | hook | 词条完成 | 否；JSON field 保持原文 |
| apps/hook-bridge/src/main.rs:427<br>crates/core/src/state.rs:675<br>`question.default` | Question | 问题 | question | 词条完成 | 否；仅缺少 Agent 文本时使用 |
