# CNDDVP EchoIsland 维护约定

本仓库是基于 FunplayAI/EchoIsland 的简体中文增强版，Windows 优先，MIT 许可。用户本轮明确指令优先于本维护约定。

## 同步前

1. 检查 status、remote、branch、最近提交；origin 保持 CNDDVP，upstream 为 FunplayAI。
2. fetch 后找 merge-base，分别审阅 BASE→upstream 与 BASE→Fork 的提交和完整文件差异。
3. 修改代码前输出中文上游差异报告，逐重要提交分类 A 必须 / B 建议 / C 可选 / D 不可直接覆盖。
4. 检查 `technical-notes/CN_FORK_PATCHES.md`，在独立同步分支进行语义合并。

禁止 reset --hard 到上游、整目录覆盖、未经审阅的大合并，以及为了消除冲突删除 CN 能力。保留上游历史与来源信息；避免将不相关改动压成一个不可追踪的 squash。

## 保护区

- Windows 目标显示器物理边界、目标 DPI、SetWindowPos、Hit Test 与绘制缓存保持一致。不得根据旧 HWND DPI 猜测所选屏。
- Watcher 只有推送成功后才写去重状态；失败、文件部分写入和暂时锁定要允许后续重试。
- 所有 Python 候选都排除 WindowsApps stub，后台程序无窗口启动。
- 当前用户自启仅使用 HKCU，路径加引号，卸载只清理本应用条目。
- 更新元数据和下载都限定 CNDDVP；未建立专用签名时手动更新，绝不回退下载官方版。
- 中文、16 会话、动态徽章、多位计数、正向设置开关、国内观察型集成需迁移而非覆盖。

## 中文与架构

用户文案优先使用 `crates/i18n` 的共享 zh-CN / en-US 词库；默认 zh-CN。不要翻译变量、枚举、JSON key、协议字段、事件、路径、URL、CLI argument 或用户内容。Windows 使用系统 CJK 字体和 glyph fallback。文案截断保持 Unicode grapheme 完整。

通过 Adapter → Normalize → Unified Event → Runtime → UI 接入工具。来源发现与完整会话/Hook/审批能力必须区分；能力模型保守声明，不在 UI/Runtime 堆叠 Agent 名称判断。保持轻量和不抢焦点。

## 验证与交付

- 准备 hook bridge 资源后运行 cargo fmt --all -- --check、cargo clippy --workspace --all-targets --no-deps -- -D warnings、cargo test --workspace、cargo check --workspace 和 npm run check。
- 修改 DPI 要覆盖用户的混合 DPI/负 X/Y/竖屏矩阵；修改 watcher 要验证失败重试、去重、partial/truncate/lock；修改通信要验证认证/畸形/超大/断开重连。
- Windows 打包用 npm run desktop:build 和 npm run desktop:build:portable；WiX 校验失败要修结构，不关闭验证。
- 更新 ledger、中文 CHANGELOG、迁移指南、README 和最终中文报告。分别记录自动测试、实际运行、硬件视觉、安装生命周期与跨平台验证；不将未执行、跳过或缺 fixture 的测试说成通过。
- 上游监测只生成差异产物，不能自动 merge main；本地构建不代表已经发布。
