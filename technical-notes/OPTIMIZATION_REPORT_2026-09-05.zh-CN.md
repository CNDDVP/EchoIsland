# 本轮 EchoIsland 优化报告

审计开始于 2026-09-05，最终构建与验收完成于 2026-09-06。本轮在 `upstream-sync/20260905` 分支完成三方差异审计、语义合并、CNDDVP 保护补丁复核、简体中文补齐、Windows 构建和真实混合 DPI 运行验证；未推送、未创建 `v0.7.0-cn` 标签、未发布 GitHub Release。

## Upstream

| 项目 | 结果 |
| --- | --- |
| 原 commit | CNDDVP `27c48cf9e926b9cb2b8fddafb7285cdb5bbe3f1d`（`v0.6.3-cn`） |
| 新 commit | FunplayAI upstream `a7ebfff5887f7edae285f71fac049d025361880f` |
| 共同祖先 | `938dd186adb553292947e548551d0e7da986aec4` |
| CNDDVP 独有历史 | 8 个 commits，完整保留在合并父链中 |
| 同步 commits | 16 个 upstream commits |
| 未同步 commits | 0；本分支合并 `upstream/main` 的完整提交历史，并对冲突处进行语义合并 |
| CNDDVP 版本 | `0.7.0`；后续发布标签约定为 `v0.7.0-cn` |

完整 BASE → UPSTREAM → CNDDVP 分模块分析、逐提交 A/B/C/D 分类和冲突结论见 [UPSTREAM_AUDIT_2026-09-05.zh-CN.md](UPSTREAM_AUDIT_2026-09-05.zh-CN.md)，完整文件级差异清单见 [UPSTREAM_DIFF_2026-09-05.txt](UPSTREAM_DIFF_2026-09-05.txt)。同步方式为正常双父合并，没有执行 `reset --hard upstream/main`，也没有整目录覆盖 Fork。

## 本轮同步功能

1. 吸收上游 mascot sprite renderer、默认资源帧和动画表现，沿用共享 Native renderer / visual plan 架构。
2. 同步 native runtime 状态稳定性、面板更新调度、缓存、状态关闭保护和完成态副作用处理，恢复缺失 fixture 被静默跳过的测试。
3. 同步 Native Core、Windows host runtime、renderer、visual plan 和大型测试文件的模块拆分，减少后续同步时的单文件冲突面。
4. 同步原生会话接入、焦点返回、Codex App 目标识别、进程来源发现及 macOS 共享架构改进。
5. 同步弹出动画平滑和 Direct2D/DirectWrite 渲染缓存优化，保留 CN 侧布局、中文字体和多会话容量。
6. 同步 hook bridge 资源准备、workspace clippy 范围和 macOS compile CI；新增只生成审计产物、不自动合并 `main` 的 upstream 定期检查工作流。
7. 采用上游 mixed-DPI descriptor 方向，并在真实 Windows 双屏验收中发现、修复完整 payload 与 renderer refresh 丢失目标屏物理上下文的缺口。

## CNDDVP 保留能力

1. 继续定位为“简体中文深度优化 + Windows 优先增强版”，README 默认中文，保留 MIT、上游身份和社区维护说明。
2. 保留最多 16 个会话、820 px 宽度上限、中文/长工具名动态徽章、多位计数和正向开关语义。
3. 保留并加强目标显示器物理顶部居中：logical/physical frame、目标 DPI、SetWindowPos、绘制和 Hit Test 使用同一屏幕上下文；失败路径保留旧可用状态。
4. 保留 Watcher“发送成功后才去重”、失败重试、partial/truncate/lock 处理，以及 WindowsApps `python.exe` / `pythonw.exe` stub 排除。
5. 保留 Kimi、Antigravity、ZCode 观察型接入、真实标题、2 小时历史过滤和 120 秒闲置判断；没有把观察能力表述为完整审批能力。
6. 保留 NSIS/MSI 当前用户安装、HKCU 自启和卸载时只删除 EchoIsland 自启值；安装不要求管理员权限。
7. Updater 只信任 CNDDVP Releases；独立签名材料未配置时关闭自动安装并引导手动更新，防止中文版被 FunplayAI 官方包覆盖。
8. 维护 18 项 [CN_FORK_PATCHES.md](CN_FORK_PATCHES.md) 条目；已由上游 fixtures 取代的旧静默跳过补丁明确标记退役，其余补丁写明删除条件。

## 中文化

新增共享 `echoisland-i18n` 薄层，以 `zh-CN` 为默认、`en-US` 为回退。Rust Native UI、DirectWrite、托盘、设置、审批/问题/完成卡片、错误边界、CLI、构建脚本、Hook、外部集成、Updater 和安装器共同使用或校验同源词库。最终检查覆盖 **137 个中英词条、252 个生产文件**。

本轮最后补齐了 Agent 插件事件自动生成的 `Completed` / `Failed`，默认显示为“已完成 / 执行失败”；真实解析路径测试同时修复了顶层 `event` 为字符串时被错误当作嵌套对象解包的问题。协议 key、事件/status 值、CLI 参数、环境变量、路径、URL、Agent/Session ID、用户和 Agent 原文保持英文或原样，不做破坏兼容性的翻译。

DirectWrite 使用 Windows 系统 `Microsoft YaHei UI` 和 `zh-CN` locale，图标继续使用 `Segoe MDL2 Assets`，未打包大型 CJK 字体。标题、消息预览、换行和省略使用 Unicode grapheme 边界，覆盖 CJK、组合重音、ZWJ Emoji、肤色、国旗与 keycap。完整位置、原文、译文、场景与协议影响见 [ZH_CN_COVERAGE_AUDIT.md](ZH_CN_COVERAGE_AUDIT.md)。

有意保留的英文包括产品/工具品牌、系统 API 字符串、协议报文、技术日志、英文文档、测试样例和外部内容。`--scan` 是人工复核候选器；其候选主要属于这些类别，不能把扫描器无候选误当作零漏译证明。

## Windows

真实环境为 Windows 11 企业版 10.0.26200，双屏排列如下：

| 显示器 | 物理边界 | DPI / 缩放 | 最终原生窗口 | 结论 |
| --- | --- | --- | --- | --- |
| `\\.\DISPLAY29` 主屏 | `0,0,3840,2160` | 144 DPI / 150% | `1605,0,630,120` | 物理顶部居中，尺寸按目标 DPI 放大 |
| `\\.\DISPLAY30` 副屏 | `3840,0,1920,1080` | 96 DPI / 100% | `4590,0,420,80` | 物理顶部居中，未沿用主屏 150% 缩放 |

验收使用 Per Monitor V2 查询窗口 DPI、物理矩形和所属 monitor。最初副屏曾出现 `6885,0,630,120 @ 144 DPI`，定位到两个数据丢失点：payload 默认转发只携带 display index/logical frame，以及 renderer refresh 清空 scale/physical frame。修复后完整 payload 从 runtime input 贯穿 host descriptor 和 scene sync，Windows renderer 刷新保留所选显示器上下文；新增测试同时覆盖完整 payload 与 legacy 无物理上下文调用的兼容路径。

NSIS 和 MSI 均成功生成。MSI 数据库复核结果：`ProductLanguage=2052`、`ProductVersion=0.7.0`、`InstallScope=perUser`、`InstallPrivileges=limited`、`INSTALLDIR` 位于 `LocalAppDataFolder`；主程序、Hook bridge 和 startup 组件 attributes 均为 260，KeyPath 是 HKCU 注册表项。Startup 值为带引号的 `"[!Path]"`。NSIS 预处理配置为 `currentUser` / `RequestExecutionLevel user`、`SimpChinese`，卸载只执行 `DeleteRegValue HKCU ... Run ... EchoIsland`。

最终产物：

| 产物 | 字节 | SHA-256 | 签名 |
| --- | ---: | --- | --- |
| `target/release/bundle/nsis/EchoIsland_0.7.0_x64-setup.exe` | 6,325,941 | `198b027e1b6784dcb611708220788391d4d2f3d5772e293e6c833b64ecec378e` | NotSigned |
| `target/release/bundle/msi/EchoIsland_0.7.0_x64_zh-CN.msi` | 8,634,368 | `e8cbd913441b455a7a32cdc3ec16a811629b38bf32075fb5772f689663a7fc1e` | NotSigned |
| `apps/desktop/dist/EchoIsland_0.7.0_x64_portable.zip` | 8,285,515 | `84c46eb60b8dbac7b4b3d3daad95725afe903fc2226f8828d7cf93d2fcefd22f` | ZIP；内含未签名 EXE |
| `apps/desktop/dist/EchoIsland_0.7.0_integrations.zip` | 19,428 | `df1725f2a4144b73e0ba6119ebdf0123a6f471db8604feb244beb57322d63c7a` | ZIP |

便携包内含 `EchoIsland.exe`、`echoisland-hook-bridge.exe` 和 `EchoIsland.portable`；集成包内含 Watcher、ZCode bridge、OpenClaw 插件、中文说明及中英 locale。发布目录另有 `SHA256SUMS-0.7.0.txt`。当前版本没有代码签名，不能把本地构建视为公开发布包。

## Agent

1. Codex App 扫描使用 SQLite 主库 + WAL 联合指纹、100 ms busy timeout 与锁重试，单列时间查询使用真实时间戳；原生 Rust 为默认 owner，旧 Python Codex App 分支需显式开启，避免双扫描。
2. Codex TOML Hook 配置改用 `toml_edit` 结构化更新，保留注释和其他 profile；无效 TOML、错误字段类型或读取失败时不覆盖原文件。
3. OpenClaw 的受管 Adapter 和独立插件共用本机 HTTP 规则，只接受 literal loopback `/event`、禁止重定向、每次读取轮换 token，并设置超时和 1 MiB 输入上限。
4. ZCode bridge 保持观察型事件，只转发非阻塞 Hook，不执行事件给出的命令。Watcher 对锁定、截断、partial JSONL、超大记录和失败重试有专项回归。
5. 新增保守的 `AdapterCapabilities` 注册模型，区分 session scan、process detection、realtime hook、approval、question、completion、history 和 terminal focus；当前作为描述 API，尚未据此重构全部 UI。
6. 最终便携版通过 `/agent/events` 接收 16/16 个 raw 顶层事件，16 个 `qa-session-*` ID 在原子持久化状态中全部存在且唯一；标题包含简体中文、ZWJ/肤色 Emoji 和国旗。运行时还只读发现了本机另一来源的 18 个会话，因此总数组 34 条，验收按指定 QA ID 集合断言，没有把环境数据误计为失败。

## 测试

| 门禁 | 最终结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace --locked --quiet` | 通过 |
| `cargo clippy --workspace --all-targets --no-deps --locked --quiet -- -D warnings` | 通过，零 warning |
| `cargo test --workspace --locked --quiet` | **753 passed，0 failed，0 ignored**；含 632 项 desktop、IPC 集成、Persistence、Runtime、Adapter、Core、i18n 等 |
| `npm run check` | 通过；137 个中英词条、252 个生产文件，Node 集成 7 passed |
| `npm run test:integrations` | 7 passed，0 failed |
| Python `unittest` Watcher | 15 passed；覆盖 fail→retry→dedupe、partial、truncate、lock、WAL、超大记录和不可信 receiver |
| PowerShell parser | 仓库全部 `.ps1` 通过 |
| `git diff --check` 与冲突标记扫描 | 通过 |
| `npm run desktop:build` | 通过；生成 NSIS 与 zh-CN MSI |
| `npm run desktop:build:portable` | 通过 |
| 最终 Windows runtime smoke | 主/副屏物理几何通过；16/16 Agent events 接收并持久化 |

副屏 16 会话运行后的 30 秒观测样本为 **5.41% 单核口径 CPU、66.56 MiB working set**。这是单机短样本，用于排除空转失控，不代表跨设备性能基准。

## 修改文件

| 文件 / 模块 | 原因 |
| --- | --- |
| `apps/desktop/src-tauri/src/native_panel_*`、`windows_native_panel/*`、`macos_native_panel/*` | 吸收上游 renderer/runtime/animation/cache/test 模块化；保留 CN Windows DPI、16 会话、中文布局、字体和退出生命周期 |
| `apps/desktop/src-tauri/resources/mascot/default/*` | 同步上游 sprite 资源，并将用户可见资源名改为中文 |
| `crates/core`、`crates/runtime`、`crates/ipc`、`crates/persistence` | 迁移上游底层结构；补统一 Agent 投影、IPC 边界测试与原子持久化 |
| `crates/adapters` | 同步 Agent 来源/Codex/OpenClaw；补能力模型、WAL/锁处理、结构化 TOML 和 loopback 信任边界 |
| `crates/i18n`、Native scene/renderer、tray、updater、hook bridge | 建立 zh-CN 默认/en-US fallback 文案层和 grapheme 安全文本处理 |
| `integrations/*` | 保留并加固 Kimi/Antigravity/ZCode Watcher、OpenClaw 插件、当前用户路径与安全传输 |
| `apps/desktop/src-tauri/installer*`、Tauri 配置、构建脚本 | 0.7.0、中文 per-user NSIS/MSI、HKCU startup/uninstall、portable 和 Hook bridge 资源准备 |
| `.github/workflows/*`、`scripts/*` | 完善 Windows/macOS CI、词库/版本/更新源/集成检查，以及只审计不自动合并的 upstream 跟踪 |
| `README.md`、`README.en.md`、`MIGRATION_GUIDE.md`、`CHANGELOG.zh-CN.md`、`technical-notes/*` | 中文产品定位、能力边界、三方审计、Patch Ledger、中文覆盖、迁移与最终验收证据 |

上表按职责归组。逐文件 name-status 和完整提交差异以 [UPSTREAM_DIFF_2026-09-05.txt](UPSTREAM_DIFF_2026-09-05.txt) 及最终 Git merge diff 为准，避免在报告中复制数百条资源帧路径。

## 风险

1. 产物未进行 Authenticode 签名，CNDDVP updater 也没有独立生产签名材料；当前设计主动退化为手动更新。
2. 本机已有用户安装，为避免覆盖真实应用，没有执行安装器的安装→升级→卸载生命周期；当前证据是成功构建、生成脚本检查和 MSI 数据库静态审阅。该流程需在干净 Windows 用户或 VM 完成。
3. Codex 的 Computer Use surface 未暴露这个 Native Win32 窗口，因此没有可审计的界面截图；中文/英文/数字/Emoji 的实际像素级遮挡和 glyph fallback 仍需人工视觉矩阵。
4. 真实硬件验证覆盖 4K 150% + 1080p 100%。4K 200% + 2K 125%、负 X/Y、主屏在右、副屏在左、竖屏、休眠恢复、Explorer 重启和虚拟桌面当前只有相关逻辑/数值测试或尚无环境证据。
5. macOS 共享代码已合并，并配置 CI compile job；本轮 Windows 主机没有完成 macOS 实机运行、打包和视觉验证。
6. `AdapterCapabilities` 尚未完全驱动 UI，Gemini/GLM/VS Code/Cursor/Trae 的 process detection 不能被解释为完整 session/hook/approval 支持。

## 后续建议

1. 在干净 Windows 10 与 Windows 11 VM 执行 NSIS/MSI 新装、旧管理员版迁移、0.7.0 同版修复、升级、开机自启、卸载及用户数据保留矩阵，并保存安装日志。
2. 建立 CNDDVP 独立代码签名与 Tauri updater 密钥的离线保管、CI 注入、轮换和回滚流程；完成签名包验证后再开启自动安装。
3. 补齐 200%/125%、负坐标、竖屏、左右主副屏交换、休眠恢复、Explorer 重启和虚拟桌面实际硬件/VM 测试；为 Native panel 增加可访问的 UI Automation 语义与截图基准。
4. 让设置与诊断 UI 消费 `AdapterCapabilities`，展示“进程发现 / 会话扫描 / Hook / 批准 / 问题 / 焦点”等真实能力，避免按 Agent 名称硬编码功能。
5. 继续用 upstream audit workflow 跟踪 FunplayAI 新提交；每次同步先更新 Patch Ledger，再在 `upstream-sync/YYYYMMDD` 分支做三方语义合并和完整门禁。
