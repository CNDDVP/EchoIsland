# EchoIsland 上游同步分析报告

本报告同时作为《CNDDVP 与 FunplayAI EchoIsland 完整差异报告》。审计日期：2026-09-05。**在修改生产代码前完成**；依据完整 Git 历史、三方文件差异和源码审阅，而非仅比较 README。实现及验证结果另记在本轮优化报告，本文保留修改前事实。

## 当前版本与比较范围

| 项目 | 审计基线 |
| --- | --- |
| CNDDVP | `27c48cf9e926b9cb2b8fddafb7285cdb5bbe3f1d`，`v0.6.3-cn`，清洁工作区 |
| FunplayAI upstream | `a7ebfff5887f7edae285f71fac049d025361880f`，清单版本仍为 `0.6.1` |
| 共同祖先 BASE | `938dd186adb553292947e548551d0e7da986aec4` |
| Fork 独有提交 | 8 |
| Upstream 独有提交 | 16 |
| BASE → Fork | 45 文件，1285 行新增、305 行删除 |
| BASE → Upstream | 261 文件（含吉祥物图像和测试拆分），24791 行新增、16955 行删除 |
| origin | `https://github.com/CNDDVP/EchoIsland.git`，未修改 |
| upstream | `https://github.com/FunplayAI/EchoIsland.git`，本轮添加并 fetch |
| 实施分支 | `upstream-sync/20260905`，main 保持原样 |

已执行 `git status`、`git remote -v`、`git branch -vv`、`git log --oneline --decorate -n 30`、`git fetch --all --prune`、`git merge-base main upstream/main`、`git log --left-right --cherry-pick --oneline upstream/main...main`，以及双向三点 `git diff --stat/--name-status` 和模块源码差异。完整逐文件清单见 [机器生成的三方差异](UPSTREAM_DIFF_2026-09-05.txt)。未发现已有 AGENTS.md、Fork patch ledger 或中文 CHANGELOG。

## 模块分析

| 模块 | 上游新增 / 修改 | Fork 独有 / 冲突与影响 | 同步决定 |
| --- | --- | --- | --- |
| Runtime | 统一 AgentSession 查询、Default、Path 参数和排序整理 | 生产代码无 Fork 分叉；持久化线程调度未变 | 同步，保留事件系统 |
| Core | AgentSource / AgentSession 是 SessionRecord 的投影视图；protocol/state 为 lint 整理 | Fork 将缺失样本测试直接跳过，不能保留这种假成功 | 同步投影视图和严格 fixtures；不重写协议 |
| IPC | **没有上游改动** | 双方相同，有 token 和 1 MiB 上限；无效 token、畸形输入等测试仍应加强 | 保留实现，不能宣称吸收了不存在的修复 |
| Persistence | **没有上游改动** | 审计发现双方共享“先删除旧文件再 rename”的既存风险 | 不冒充上游修复；审计后以 CN-PATCH-018 补充原子替换和失败保留测试 |
| Adapter | 来源注册表；Gemini/GLM/VS Code/Cursor/Trae 进程发现；Codex App SQLite 索引；hooks 标志迁移；OpenClaw 工具审批插件 | Fork Kimi/Antigravity/ZCode 观察接入保留；重复 Codex watcher 会覆盖 Rust 状态；SQLite WAL 缓存、TOML section、receiver URL 要补修 | 吸收并做语义修复；进程发现不等于完整 hook/approval 支持 |
| Desktop | 删去旧 WebView 窗口宿主、统一 native 刷新/状态处理，单实例、终端返回和 Codex App 定位增强 | 保留中文、16 会话、正向开关、退出保护的产品意图；无条件 prevent_exit 需精确限定隐式退出 | 语义吸收，生命周期单独验证 |
| Windows Native | host runtime / tests / shared renderer 拆分；动画稳定、资源缓存；目标显示器 scale_factor 数据流 | Fork 物理屏顶部居中、中文动态宽度和两位数计数必须迁移到新模块；当前计数还错误排除 22/23 | 同步架构，保留 CN 薄层并补回归测试 |
| macOS | 精灵渲染、卡片/动画/文本布局、Codex App 返回、单实例重开 | 中文共享文案需保留；本机不能做 macOS 实机验证 | 随共享架构同步，CI 编译检查 |
| Installer | 精确打包 bridge 资源到 resources 子目录 | Fork NSIS HKCU 自启保留；原 Run 路径没引号；MSI 无对应自启；无安装器中文配置 | 语义保留和增强，使用官方语言机制 |
| Updater | upstream 无新增修复 | CNDDVP endpoint/发布页不可覆盖；pubkey 沿用历史配置，不能凭空宣称拥有 CN 私钥 | 严格限定 CN 更新源，metadata 不可用则手动更新 |
| Scripts / Integrations | upstream 无新增独立 integrations 脚本 | CN watcher 成功后去重/失败重试、真实标题、历史过滤；Python WindowsApps stub 排除；旧插件硬编码个人路径 | 保留有效 patch，退役个人路径和双扫描 |
| CI | 新增 Windows fmt/check/test/clippy，bridge 资源准备；macOS check | Fork 无 CI | 同步并补前端/中文及保护区检查；禁止自动 merge main |
| Documentation | bridge 构建前置说明；来源规划和 Codex App 返回中英文技术文档 | 默认 README 仍英文；MIGRATION_GUIDE 是 CN 独有中文文档 | 中文主 README + README.en.md；完善 patch ledger / changelog |
| Dependencies | rusqlite 0.32.1 bundled 与相关传递依赖；Win32 特性调整 | 保留 Fork 独立版本；npm 无上游依赖变化 | 按锁文件吸收，不做无关依赖大升级 |

### 能力边界

上游新 AgentSessionStatus 虽有 Completed/Error，现有 AgentStatus 尚无对应投影来源；不能把这称为完整事件模型升级。新注册表尚无 AdapterCapabilities。Windows 进程扫描主要依赖可执行文件名，无法可靠从通用 node.exe 推断 npm 安装的所有 Agent。后续通过 Adapter/capability 层扩展，避免在 UI/Runtime 堆叠 Agent 名称判断。

## 上游提交分类

A：必须同步（缺陷/可靠性/验证修复）；B：建议同步（能力/维护/性能）；C：可选同步（macOS）；D：提交中不能原样接收的冲突段。

| 提交 | 分类 | 内容与决定 |
| --- | --- | --- |
| `59ffb39` | B | 吉祥物精灵渲染和来源规划；吸收资源/架构，保护 CN 卡片文案与布局 |
| `257ce18` | A/B | native runtime 稳定和 HTTP 接收可靠性；吸收并验证认证、请求边界 |
| `05d8541` | B | 原生测试模块拆分；迁移 CN 断言，不能丢失测试 |
| `cc2319c` | B | Windows host 职责拆分；保留 CN 平台行为 |
| `20d189b` | B | shared renderer 拆分；CN visual plan patch 随逻辑迁移 |
| `cf41323` | B/D | 会话/Codex App/OpenClaw 集成；D：本地地址盲信、中文/startup/DPI 不直接覆盖 |
| `5c36f82` | A | 弹层动画缺陷修复，保留 CN 大会话容量 |
| `f634ba7` | B | 渲染性能/缓存；检查中文字体及缩放后缓存有效性 |
| `0e58674` | A/D | 状态/点击/Codex 识别/hooks/fixtures 修复；D：SQLite 只看主库 mtime，TOML 文本全文件匹配 |
| `426db44` | A | CI 提前准备 hook bridge 资源 |
| `ebcf17b` | B | clippy 只检查 workspace 自身 |
| `37534b6` | B | CI 展示 clippy 错误 |
| `0a52a4d` | A | 新版 clippy 排序兼容 |
| `e5882f7` | A | 新版 clippy 队列排序兼容 |
| `f31dc66` | A/D | 目标屏 DPI 数据流；D：虚拟桌面总包围盒 clamp 不保证目标屏，不能覆盖 CN 顶部居中 |
| `a7ebfff` | C | macOS 编译门禁，随共享架构保留 |

所有 16 个提交均已评估；计划保留其历史，在独立分支进行三方语义合并，D 段按本报告和 ledger 修复。绝不使用 reset --hard upstream/main、整目录覆盖或删除 CN 功能来消解冲突。Git 自动无冲突不等于逻辑验证通过。

## Fork 独有提交与保护区

| 提交 | 现有能力 / 决定 |
| --- | --- |
| `b821c4e` | 中文场景/设置、16 会话、动态徽章、正向开关：保留；缺样本提前 return 的测试：由上游严格 fixture 方案替代 |
| `ae508d3` | 中文迁移、一键接入和国内 watcher：保留；个人路径硬编码插件需修正 |
| `ef9607f` | Antigravity protobuf 真实标题：保留 |
| `b0365bf` | 2 小时历史过滤、120 秒闲置：保留 |
| `bca7606` | 两位数计数：保留并修复 22/23 特例 |
| `45f0adf` | Windows 隐藏 WebView 不误退出：迁移为仅阻止隐式退出，显式退出正常 |
| `7bb22ae` | DPI/Watcher 重试/Python stub/NSIS 自启/CN 更新源：逐项保护 |
| `27c48cf` | 目标显示器物理边界顶部居中：移植到上游目标 DPI 链路 |

详见 [CN_FORK_PATCHES.md](CN_FORK_PATCHES.md)。

## 中文覆盖初步结论

主体 native 场景、设置、托盘、更新状态已中文，但无统一 locale 层。真实遗漏包括 `Approval Required`、未识别 status 原样显示、CLI 帮助/状态/错误、Adapter 安装 notes、命令失败详情、Watcher 的 `Idle`/`Antigravity conversation activity`、插件拒绝原因和 manifest 描述。占位 Web HTML 仍 lang=en；NSIS/WiX 未配置官方简体中文语言。

DirectWrite primary 为通常未安装的 Noto Sans SC，fallback 字段未参与字体创建，locale 为 en-us；需改为系统 CJK 字体和 zh-CN，保留系统 glyph fallback 与图标字体。纯英文开发日志、英文文档副本、协议字段/URL/路径/Agent 产品名不应机械翻译。完整清单和完成状态另见简体中文覆盖审计报告。

## 实施与验证计划

1. 先落地本报告、三方证据及 ledger；再合并经审阅上游技术底座，逐冲突语义处理。
2. 恢复严格 fixtures，修补 SQLite WAL / Codex TOML / OpenClaw receiver 信任边界，保留 Watcher 重试并消除双扫描。
3. Windows 保留目标物理屏顶部居中与 16 会话，修复计数/退出/中文字体和自启路径；增加 DPI/负坐标/竖屏/L 形屏测试。
4. 建立默认 zh-CN 的轻量 locale 层，覆盖 native/CLI/脚本/安装器用户文本；协议英文命名保持。
5. 更新独立 CN 版本、主 README/英文说明/MIGRATION_GUIDE/CHANGELOG/最终报告。
6. 运行 fmt、workspace clippy/test/check、前端检查、Windows 单独测试、NSIS/MSI/Portable 构建。记录真实命令和结果，测试缺样本不允许静默成功。

## 当前风险与验证条件

- 本机初始 PATH 无 Rust/Cargo/npm/MSVC。正在工作区旁准备隔离构建工具链，未修改系统 PATH；构建前置问题必须与代码失败分开记录。
- 混合 DPI 自动测试能证明坐标数学和数据流，不能替代 4K150%+1080p100%、4K200%+2K125%、负 X/Y、竖屏等真实硬件验证。
- Windows 安装/升级/卸载、自启注册以及不抢焦点/睡眠恢复/Explorer 重启需运行时验证；不能用静态搜索代替结论。
- macOS 本机不可运行；CI 配置本身不是已经通过的 CI 结果。
- 本轮本地改造不等于已经发布 GitHub Release；更新签名材料不从历史公钥推断，不生成冒充既有签名身份的密钥。
