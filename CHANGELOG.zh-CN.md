# 中文变更记录

## 0.7.0 — 开发中，未发布

- Upstream Base：`a7ebfff5887f7edae285f71fac049d025361880f`（上游清单 `0.6.1`）。
- CNDDVP Version：`0.7.0`，拟沿用 `v0.7.0-cn` 标签格式。
- Upstream Changes Included：共同祖先 `938dd18` 后的 16 个提交，详见差异审计。实际验证结果见本轮最终报告。

### 新增

统一中文 locale 薄层、Fork Patch Ledger、完整差异与中文覆盖审计；来源注册、Codex App 索引和 OpenClaw 插件能力跟随上游。

### 优化 / 修复

吸收 native runtime、渲染缓存、动画和测试拆分；恢复缺 fixture 时被静默跳过的测试。Codex SQLite WAL、TOML section 更新和 OpenClaw 接收地址信任边界按审计补强。会话状态改为落盘后原子替换，Windows 替换失败时保留上一份可读状态。

### 上游同步

Runtime/Core 保持与上游同方向，继续使用现有事件协议；IPC 与 Persistence 本体没有上游变动，不将其列为上游修复。

### 简体中文

中文主 README 与英文入口、CLI/安装器/原生场景文案统一，保留协议、命令和产品名英文。DirectWrite 采用 Windows 系统中文字体，不打包大型 CJK 字体。

### Windows

保留目标屏物理顶部居中、16 会话、动态徽章和正向开关；完整 reposition/sync payload 与 renderer 刷新持续携带目标屏物理边界及缩放。真实 4K 150% 主屏和 1080p 100% 副屏分别得到 `1605,0,630,120` 与 `4590,0,420,80`，均按目标屏 DPI 在物理顶部居中。NSIS/MSI 使用中文当前用户安装、自启和卸载清理；便携版保持手动更新。

### Agent 支持

保留 Kimi/Antigravity/ZCode 观察接入、真实标题与 Watcher 重试；Codex App 避免 Python/Rust 双扫描。进程发现不等于完整 Hook/approval/question 支持。

### 已知问题 / 发布门禁

Windows 完整测试、安装器/便携版构建、真实混合 DPI 几何验证和 1080p 100% 展开态中文/英文/数字/Emoji 像素截图已通过。当前产物未进行代码签名，且 CNDDVP 专用 updater 签名材料未确立，因此更新保持 CN 发布页手动下载。干净用户安装/升级/卸载生命周期、macOS 编译以外的运行验证，以及 125%/150%/200% 展开态、负坐标/竖屏/休眠恢复/Explorer 重启等扩展硬件视觉矩阵仍待专用环境验证。旧管理员级 MSI 需先卸载再迁移当前用户安装。

## 0.6.3-cn

原 Fork `27c48cf`：物理显示器顶部居中修复。

## 0.6.2-cn

原 Fork `7bb22ae`：混合 DPI、Watcher 发送失败重试、Python Store stub 排除、NSIS 自启及 CNDDVP 更新源。

## 0.6.1-cn

原 Fork `b821c4e` 起：中文化、16 会话与自适应、真实标题及国内工具观察接入。本轮以 ledger 逐项记录仍需保留和可被上游替代的 patch。
