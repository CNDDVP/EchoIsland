# Completion Reminder 语义

更新时间：2026-05-08

## 目标

Completion reminder 是 Agent 完成回复后的统一提醒语义。macOS 和 Windows 必须共享同一套触发、展示、已读和清除规则。平台代码只负责把 shared 状态渲染成声音、卡片、宠物气泡和 glow，不允许各自重新判断“是否完成”或“是否已读”。

## 核心对象

- `CompletionBadgeItem`：未读 completion reminder 的持久状态。它驱动 compact 状态下的宠物右上角绿色气泡和 completion glow。
- `StatusQueuePayload::Completion`：自动弹出的 completion 消息卡片。它是展示动作，不等同于已读。
- `PanelReminderState`：shared core 输出给平台层的提醒状态，包含 `completion_badge_count`、`show_completion_glow`、`show_status_card`、`play_sound` 和 mascot 状态。
- `mark_completion_reminders_viewed`：shared core 中唯一的“标记 completion 已读”入口。

## 触发规则

一次 completion reminder 只能由 shared `detect_completed_sessions` 产生。

触发条件：
- session 从 `Processing` 或 `Running` 变为 `Idle`。
- 这类主动运行态完成即使没有 `last_assistant_message`，也会触发通用 completion reminder。

补充触发条件：
- session 已经是 `Idle`。
- 最近 `last_assistant_message` 更新。
- 更新发生在最近窗口内。
- 新消息 trim 后非空。

不触发条件：
- 子 agent / child task 完成，但父 session 仍在运行。
- 只有 task signal 变化，没有可展示的 assistant 输出。
- stale session 重新扫描，但没有新的完成事件。

## 展示规则

新增 completion reminder 时，shared core 同时产生两类效果：
- 添加 `CompletionBadgeItem`，用于 compact 未读气泡和 glow。
- 添加 `StatusQueuePayload::Completion`，用于自动弹出的 completion 消息卡片。

声音规则：
- 只在新增 approval / question / completion status item 时播放。
- 重复扫描同一个 completion 不应重复播放。
- 卡片关闭动画、hover 展开、settings 切换不应单独播放 completion 声音。

气泡和 glow 规则：
- 表示未读 completion reminder。
- 只在 compact 未展开状态显示。
- 自动 completion status 卡片展开时仍保持未读。
- 展开状态不显示气泡和 glow，但不代表已读，除非是主动交互触发的已读行为。

## 已读和清除规则

会标记 completion 已读：
- 用户手动 hover 展开灵动岛。
- 用户主动打开或切换 Settings surface。
- 其他明确的主动查看行为，必须通过 `mark_completion_reminders_viewed` 接入。

不会标记 completion 已读：
- 自动 completion status 卡片弹出。
- completion status 卡片关闭动画结束。
- status queue 过期移除卡片。
- 平台重绘、窗口 reposition、DPI 变化。

会清除旧 completion reminder：
- 新一轮对话开始。
- session 的 `last_user_prompt`、`last_assistant_message` 或 status 在 completion 之后发生有效变化。
- session 从 snapshot 中消失。

## 平台边界

shared core 负责：
- completion 检测。
- status queue 新增和过期。
- completion badge 持有和清除。
- reminder state 输出。
- mascot base state 的产品语义。

macOS / Windows 平台层负责：
- 声音播放 API。
- 原生窗口、命中测试和动画调度。
- 将 shared visual plan 绘制为平台原生 UI。
- 将点击、hover、settings action 分发回 shared command。

平台层禁止：
- 自己判断 completion 是否完成。
- 自己判断 completion 是否已读。
- 在平台私有代码里直接清空 completion badge，除非通过 shared command/helper。
- 因为卡片动画结束而清除未读气泡。

## 测试要求

shared core 必须覆盖：
- 有效 assistant message 才触发 completion。
- 无 assistant message 不触发声音、卡片、badge、glow。
- 自动 completion status 展开不清除 badge。
- hover 手动展开清除 badge。
- Settings 主动交互清除 badge。
- 新对话开始清除旧 badge。

平台 wrapper 必须覆盖：
- macOS wrapper 没有绕过 shared completion badge 规则。
- Windows runtime 没有绕过 shared completion badge 规则。
- Settings 和 hover action 都会走 shared command/helper。

## 当前实现入口

- `apps/desktop/src-tauri/src/native_panel_core/queue.rs`
- `apps/desktop/src-tauri/src/native_panel_core/reminder.rs`
- `apps/desktop/src-tauri/src/native_panel_core/interaction.rs`
- `apps/desktop/src-tauri/src/native_panel_core/settings.rs`
- `apps/desktop/src-tauri/src/macos_native_panel/queue_logic.rs`
- `apps/desktop/src-tauri/src/windows_native_panel/host_runtime.rs`
