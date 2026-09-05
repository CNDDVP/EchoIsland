# Codex App 点击跳转方案

最近更新：`2026-05-20`

状态：已确认 Codex App 私有 deeplink 格式，并在 EchoIsland 中实现最佳努力跳转；仍需在打包版本中做端到端验证。

## 目标

当 EchoIsland 中出现 Codex App 会话消息时，用户点击消息区域后：

- 基础目标：把 Codex App 拉到前台。
- 增强目标：尽量让 Codex App 回到对应的具体 thread。
- 失败兜底：如果不能恢复具体 thread，仍然正常激活 Codex App，不影响点击体验。

## 已确认信息

Codex App 的 macOS bundle id 是：

```text
com.openai.codex
```

EchoIsland 当前的 macOS 聚焦链路已经可以按 bundle id 激活原生 App：

- 点击面板命中 `FocusSession`
- runtime 根据 `session_id` 找到 `SessionFocusTarget`
- macOS focus 后端根据 `host_app` / `terminal_bundle` 解析目标
- 对 `com.openai.codex` 调用 App 激活逻辑

当前相关代码：

- `apps/desktop/src-tauri/src/native_panel_renderer/descriptors.rs`
- `apps/desktop/src-tauri/src/terminal_focus_service.rs`
- `apps/desktop/src-tauri/src/terminal_focus/macos.rs`
- `apps/desktop/src-tauri/src/terminal_focus/macos/native_apps.rs`
- `apps/desktop/src-tauri/src/terminal_focus/macos/target.rs`
- `crates/adapters/src/codex/scan.rs`

## Codex App 观察

OpenAI Codex 开源仓库中能看到的 URL Scheme 主要包括：

```text
codex://threads/new
codex://plugins/<plugin-name>?marketplacePath=<path>
```

其中 `codex://threads/new` 用于打开新会话，不适合跳回已有会话。

不过在已安装的 `Codex.app` 中可以看到桌面端注册了 `codex://`，并且“Copy deeplink”功能复制的格式是：

```text
codex://threads/<conversation-id>
```

因此当前最直接的 thread 级跳转方式是打开这个 deeplink。

开源仓库还暴露了 `codex app-server`：

- 传输协议：JSON-RPC 2.0，wire 上省略 `"jsonrpc": "2.0"`
- 默认 unix socket：`$CODEX_HOME/app-server-control/app-server-control.sock`
- unix socket 上跑的是 WebSocket Upgrade 后的一帧一条 JSON-RPC 消息
- 相关方法：`thread/resume`

示例请求：

```json
{
  "method": "thread/resume",
  "id": 11,
  "params": {
    "threadId": "thr_123",
    "excludeTurns": true
  }
}
```

本地 app-server 可以作为兜底方向。但当前安装版 Codex App 运行的是 `codex app-server --listen stdio://` 子进程，外部应用不能直接连接这个 stdio 通道；默认 unix socket 不一定存在。

## 推荐实现

### 1. 扫描阶段保留 Codex App 归属

从 Codex App 的 state sqlite 中读取 thread 信息时：

- `session_id` 使用 Codex thread id
- `source` 仍为 `codex`
- `host_app` 设为 `com.openai.codex`
- `cwd`、`model`、`title`、`first_user_message` 用于展示
- `rollout_path` 作为后续增强字段保留，但第一版优先用 `threadId`

这样即使不做具体 thread 恢复，也能通过 `host_app` 正常激活 Codex App。

### 2. 点击阶段先尝试 Codex deeplink

当点击目标满足以下条件时：

- `target.source == "codex"`
- `target.host_app == Some("com.openai.codex")`
- `target.session_id` 非空

执行：

1. 打开：

   ```text
   codex://threads/<session_id>
   ```

2. 如果系统 `open` 成功，认为已把跳转请求交给 Codex App。
3. 如果 deeplink 打开失败，再查找 Codex Home，默认 `~/.codex`，或者尊重 `CODEX_HOME`。
4. 拼出 socket 路径：

   ```text
   $CODEX_HOME/app-server-control/app-server-control.sock
   ```

5. 如果 socket 存在，建立 unix socket 连接。
6. 执行 WebSocket Upgrade。
7. 发送 `initialize`。
8. 发送 `initialized` notification。
9. 发送 `thread/resume`：

   ```json
   {
     "method": "thread/resume",
     "id": 2,
     "params": {
       "threadId": "<session_id>",
       "excludeTurns": true
     }
   }
   ```

10. 读取响应，成功则记录诊断日志，失败也只记录日志。
11. 激活 `com.openai.codex`。

### 3. 激活 App 作为固定兜底

不应该让 deeplink 或 `thread/resume` 的失败阻断用户点击。

以下情况都应继续执行 App 激活：

- Codex App 未运行
- deeplink 协议未注册或被系统拒绝
- socket 不存在
- WebSocket Upgrade 失败
- initialize 失败
- `thread/resume` 返回错误
- 协议版本变化

也就是说，用户体验上始终是：

```text
点击消息 -> 尝试打开 Codex thread deeplink -> 必要时尝试 app-server resume -> 拉起 Codex App
```

## 风险

### 1. deeplink 是桌面端私有协议

`codex://threads/<id>` 来自当前安装版 Codex App 的桌面端实现，不是公开仓库里稳定承诺的协议。后续 Codex App 版本可能调整格式，所以实现必须保留兜底。

### 2. app-server resume 不一定驱动桌面 UI 切换

`thread/resume` 能恢复 app-server 内的 thread，但 Codex App 桌面 UI 是否会因为另一个本地客户端的 resume 请求而自动切换到该 thread，需要实际验证。

如果桌面 UI 不监听这个状态，兜底只能做到：

- 唤起 Codex App
- 让 app-server 具备该 thread 的活跃上下文

但不保证 UI 选中该 thread。

### 3. Codex App 桌面壳源码可能不在当前开源仓库

当前开源仓库主要包含 CLI、TUI、app-server、SDK、协议和登录页面等内容。没有看到 macOS App 的完整 shell 工程，例如 `Info.plist` 或 Xcode 工程。

因此 EchoIsland 依赖安装版桌面壳中实际暴露的 `codex://threads/<id>` 行为。

### 4. 协议是可演进的

app-server 的 README 标注 WebSocket transport 为 experimental / unsupported。unix socket control-plane 是本地控制入口，但协议细节仍可能随 Codex 版本变化。

实现时需要：

- 短超时
- 明确日志
- 严格兜底
- 不把失败暴露成用户可见错误

## 验证计划

在安装了 Codex App 的 macOS 机器上验证：

1. 打开 Codex App，并创建至少两个 thread。
2. 启动 EchoIsland。
3. 确认扫描结果中 Codex App thread 的 `host_app` 是 `com.openai.codex`。
4. 点击 EchoIsland 中的 Codex App 会话。
5. 验证 Codex App 是否被拉到前台。
6. 观察是否切到对应 thread。
7. 查看 EchoIsland 诊断日志：

   - deeplink 是否成功交给系统
   - socket 是否存在
   - 如果走到 app-server fallback，initialize / `thread/resume` 是否成功
   - App 激活是否成功

## 推荐阶段

第一阶段：只做稳定窗口激活。

- 成本低
- 失败少
- 当前代码基础已经具备

第二阶段：加入 `codex://threads/<id>` deeplink 跳转。

- 来自 Codex App 自己的“Copy deeplink”格式
- 成功后通常应由 Codex App 自己处理 thread 切换
- 失败不影响激活

第三阶段：保留 app-server `thread/resume` 作为兜底观察项。

可选方向：

- 监听 Codex App state sqlite 变化后再激活
- 与 Codex App 后续公开协议对齐

## 结论

当前实现路径是“点击消息 -> 打开 `codex://threads/<session_id>` -> 兜底激活 Codex App”。

`codex://threads/<id>` 已从安装版 Codex App 的“Copy deeplink”逻辑确认；app-server `thread/resume` 保留为失败兜底和后续观察项。
