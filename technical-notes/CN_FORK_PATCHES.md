# CNDDVP Fork Patch Ledger

定位：EchoIsland 简体中文增强版，基于 FunplayAI/EchoIsland，Windows 优先、MIT 许可。每次同步先检查本表，状态必须以实际代码和测试更新。当前审计 Fork `27c48cf`，BASE `938dd18`，目标 upstream `a7ebfff`。

| Patch ID | 涉及文件 / 逻辑 | 目的及上游对应 | 仍需保留 | 上游已解决 / 未来删除条件 |
| --- | --- | --- | --- | --- |
| CN-PATCH-001 | windows_native_panel/platform_loop.rs、runtime_input.rs、renderer.rs；native_panel_renderer/traits.rs；descriptor/host/surface DPI | 目标屏物理顶部居中，4K/混合缩放/负坐标；原 7bb22ae/27c48cf | 是；完整 reposition/sync payload 保留目标屏缩放与物理边界，刷新不再清空屏幕上下文；单元数值矩阵与真实 4K 150% + 1080p 100% 双屏定位通过 | 仅在上游同时覆盖完整 payload、刷新路径、目标屏边界、SetWindowPos/绘制/Hit Test 同 DPI 与失败回滚后退役 |
| CN-PATCH-002 | integrations/ei-session-watcher.py | 推送成功才去重；失败下周期重试 | 是 | 上游无对应 watcher；不得提前写状态 |
| CN-PATCH-003 | integrations/setup-integrations.ps1 | 排除 WindowsApps python/pythonw stub，后台启动 | 是 | 上游无对应功能；可靠原生替代才删除 |
| CN-PATCH-004 | installer-hooks.nsh；installer/*.wxs；tauri.windows.conf.json | NSIS/MSI 当前用户 HKCU 自启与卸载清理，无管理员权限 | 是；NSIS 路径加引号，MSI 用 HKCU KeyPath 与固定组件 GUID | 上游未解决；不能回退到管理员级注册或删除整个 Run key |
| CN-PATCH-005 | tauri.conf.json；commands.rs；updater_service.rs | 仅 CNDDVP releases；可信元数据、独立签名与 30 秒检查超时，缺失时手动更新 | 永久保留发行身份；当前独立签名未配置，自动安装关闭 | 不能被上游官方更新源替代；只有建立并实测 CNDDVP 专用密钥和签名发布流程后才启用自动安装 |
| CN-PATCH-006 | crates/i18n；native_panel_scene；renderer；tray；updater；hook-bridge | 默认 zh-CN、en-US 回退、中文错误边界和正向开关语义 | 是；已迁移共享 locale，动态用户/Agent 内容保持原文 | 上游无 locale；新增用户文案必须进入共享词库 |
| CN-PATCH-007 | native_panel_core/constants/geometry.rs；queue；visual_plan | 最多 16 会话、820px 上限、动态徽章宽度/双位数计数 | 是 | 上游未完整解决；移除错误 22/23 特例 |
| CN-PATCH-008 | main.rs 的退出事件 | 无 WebView native 模式持续驻留 | 是，限定隐式退出 | 上游 native-only 消除旧 WebView，但仍需生命周期保护；显式退出不可拦截 |
| CN-PATCH-009 | integrations watcher 与 zcode-bridge.mjs | Kimi/Antigravity/ZCode 观察集成；真实标题、2h历史和120s闲置 | 是 | 未来原生 Adapter 达到同能力后可分项退役；非完整审批支持 |
| CN-PATCH-010 | hook-bridge/main.rs；core/tests/sample_events.rs | 旧版缺 fixture 直接 return 避免失败 | **否；本轮已退役** | 采用上游 0e58674 仓库内 fixtures，缺失 fixture 会真实失败，测试零 ignored |
| CN-PATCH-011 | integrations/openclaw-plugin；setup-integrations.ps1；adapters/openclaw/install.rs | OpenClaw 独立和受管插件接入 | 功能保留；固定个人路径实现已退役 | 复用共享 HTTP helper、动态当前用户路径和本机地址校验；上游具备同等边界后可收薄 |
| CN-PATCH-012 | watcher Codex App 分支；adapters/codex/scan.rs | Codex App 观察能力避免丢失 | 已迁移 Rust；Python 分支默认关闭，仅供旧版显式兼容 | 已补 SQLite/WAL 联合指纹、短忙超时、锁重试与单时间列查询；稳定后可删除旧 Python 分支 |
| CN-PATCH-013 | adapters/codex/install.rs | hooks 配置 section 安全迁移 | 是；本轮已实施 toml_edit 结构化更新 | 保留注释、其他 profile、现代 hooks 优先且无效 TOML 不覆盖；上游同等实现后可删除 |
| CN-PATCH-014 | adapters/openclaw/install.rs；独立插件；zcode-bridge.mjs；hook-bridge | 本机 receiver、禁重定向、超时、token 轮换与输入上限 | 是；本轮已实施并补真实本机 HTTP/畸形输入测试 | 上游当前信任 event_url；只有达到同等信任边界后可删除 |
| CN-PATCH-015 | windows_native_panel/directwrite.rs | 系统中文字体、zh-CN locale、系统 glyph fallback | 是；本轮已实施 | 上游字体字段不足以证明中国 Windows 环境 CJK/Emoji fallback；硬件视觉验证前不退役 |
| CN-PATCH-016 | crates/i18n/text.rs；core/agent.rs；native queue/visual_plan | 按 Unicode grapheme 裁剪、换行和省略，避免拆 ZWJ Emoji、肤色、国旗、组合重音与 keycap | 是；保持原 ASCII/CJK 预算 | 上游所有展示层均采用 grapheme 边界并有等价回归后可收敛 |
| CN-PATCH-017 | adapters/capabilities.rs；agent_sources.rs | 用保守能力模型区分进程发现、会话、Hook、批准、问题、历史和焦点 | 是；当前为注册表描述 API，尚未驱动 UI | UI 接入能力模型后继续复用；不得将进程发现宣传为完整 Adapter |
| CN-PATCH-018 | crates/persistence/src/lib.rs | 会话状态同目录写入、flush 后原子替换；替换失败不先删除旧状态 | 是；Windows 使用 MoveFileExW replace/write-through，其他平台使用 rename | 上游提供等价的跨平台原子替换和失败保留测试后可收敛 |

保护区验证包括：目标屏/DPI/Hit Test/SetWindowPos；Watcher push fail→retry→dedupe、partial/truncate/lock；Python stub；NSIS/MSI 自启/卸载；更新源和下载地址。以运行结果记录完成状态，不能把计划当作验证。
