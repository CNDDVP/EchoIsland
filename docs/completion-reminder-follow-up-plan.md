# Completion Reminder 后续计划

更新时间：2026-05-08

## 目标

把 Codex / Claude 完成提醒链路继续收口到共享层，减少 macOS 和 Windows 的平台私有判断。后续平台代码只负责渲染、命中测试和系统能力适配，不再各自推断声音、消息卡片、未读气泡、glow 的生命周期。

## 当前状态

- Codex scanner 已按 `turn_id` 跟踪 active task，避免子 agent 完成导致父任务误判完成。
- Codex scanner 已支持完整 session 文件 task activity 扫描，避免父任务 `task_started` 被 tail window 截断后误判 Idle。
- shared `native_panel_core::detect_completed_sessions` 已要求完成会话必须存在有效 `last_assistant_message`，否则不触发 completion reminder。
- shared `mark_completion_reminders_viewed` 已作为 completion 已读清除入口。
- 手动 hover 展开会清除 completion badge / glow。
- 自动 completion status 展开不会清除 completion badge / glow。
- Settings 主动交互会清除 completion badge / glow。

## 推荐执行顺序

### 1. 文档化 Completion Reminder 规则

先把当前已经确认的产品语义固定下来，避免后续 macOS 和 Windows 开发时靠记忆对齐。

需要写清楚：
- completion 触发条件：从 Processing / Running 变 Idle，且有有效 `last_assistant_message`。
- idle message 更新触发条件：Idle 状态下最近 assistant message 更新，且消息非空。
- 声音触发：只在新增 approval / question / completion status item 时播放。
- 气泡和 glow：表示未读 completion reminder，只在 compact 未展开状态展示。
- 手动 hover 展开：标记 completion 已读。
- Settings 主动交互：标记 completion 已读。
- 自动 completion status 展开：不标记已读。
- 新一轮对话开始：清除旧 completion reminder。
- completion 卡片关闭动画结束：只移除卡片，不代表已读。

建议文档位置：
- `docs/completion-reminder-semantics.md`

### 2. Codex Completion 诊断日志

给 Codex scanner 增加低噪声 debug 日志，方便后续定位误判。

建议日志字段：
- `session_id`
- `active_task_count`
- `latest_task_signal`
- `open_task_started_at`
- `last_activity`
- `resolved_status`
- `has_valid_last_assistant_message`

约束：
- 默认只用 `debug!`，不要污染普通用户日志。
- 不打印完整 prompt / assistant message，避免隐私和日志噪声。
- 不改变现有状态判断。

### 3. CompletionReminderEvent 命名收口

当前 `PanelReminderState` 和 `CompletionBadgeItem` 已经承载大部分语义，但“新增 completion / 已读 / 自动展示 / 卡片关闭”还没有显式事件名。

建议新增 shared 类型，例如：

```rust
enum CompletionReminderEvent {
    Added,
    ViewedByManualExpansion,
    ViewedBySettings,
    ClearedByNewDialogue,
    StatusCardExpired,
}
```

注意：
- 不一定需要立刻暴露给平台层。
- 优先用于测试和内部状态流命名。
- 不要为了类型化而扩大改动范围，先替换最容易分叉的清除逻辑。

### 4. macOS / Windows Parity 测试补齐

在 shared core 已有测试的基础上，补平台薄测试，确保两端 wrapper 没有绕过 shared 语义。

建议覆盖：
- 自动 completion 卡片弹出不清除 badge。
- 手动 hover 展开清除 badge。
- Settings 点击清除 badge。
- 新对话开始清除旧 badge。
- 无有效 assistant message 不触发声音、卡片、badge、glow。

要求：
- 测试尽量调用 shared runtime / wrapper，不直接测试平台绘制细节。
- macOS 平台测试只验证状态桥接，不在 Windows 环境强依赖 AppKit 实机行为。

### 5. Codex Fallback 性能优化

当前为了正确性，Codex task activity 扫描会读取完整 session 文件。后续可以做增量化，降低大 session 高频扫描的 IO 成本。

推荐方向：
- 在 `CodexSessionScanner` 缓存每个 session 文件的 task activity tracker 状态。
- 文件 size / modified_at 未变化时复用解析结果。
- 文件增长时只扫描新增字节区间。
- 如果文件被截断或 rotation，回退完整扫描。

验收标准：
- 保持现有 Codex 子 agent / tail window 截断测试全部通过。
- 增加一个“大文件增量扫描不丢 active parent task”的测试。
- 不牺牲状态正确性换性能。

### 6. 阶段性提交整理

当前本地未提交内容较多，建议阶段性整理提交，避免后续 macOS 开发混入 Windows / Codex 历史改动。

建议拆分：
- `fix: harden codex task activity tracking`
- `fix: align native completion reminder semantics`
- `feat: share island width settings behavior`

如果需要合成一个阶段性提交，建议提交名：

```text
fix: align native panel completion and codex session semantics
```

提交前必须验证：
- `cargo test -p echoisland-adapters`
- `cargo test -p echoisland-desktop`
- `cargo check -p echoisland-desktop`
- `git diff --check`

## 暂不建议做的事

- 不建议在 Windows 平台单独补 completion reminder 判断。
- 不建议让 macOS 和 Windows 各自判断“完成是否已读”。
- 不建议为了性能回退到只看 tail window。
- 不建议把完整 prompt / assistant message 放入诊断日志。

## 下一步推荐

优先执行第 1 步：新增 `docs/completion-reminder-semantics.md`。文档确定后，再执行第 2 步 Codex debug 日志。这样后续每次遇到 macOS / Windows 差异，都可以先对照语义文档判断是共享逻辑问题还是平台渲染问题。
