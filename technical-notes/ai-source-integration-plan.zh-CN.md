# 多 IDE / CLI AI 来源接入方案

最近更新：`2026-05-14`

## 目标

将 EchoIsland 从当前偏 Codex 会话识别的实现，扩展为统一的 AI 工作状态中心。

目标接入对象包括：

- CLI：Codex CLI、Gemini CLI、GLM CLI、Claude Code 等
- IDE：Trae、Cursor、VS Code、JetBrains 系列
- IDE 插件：VS Code Codex 插件、Cursor / Trae 内置 AI 或插件式 AI

最终效果：

- 统一展示不同 AI 工具的会话状态
- 统一展示来源、项目、任务标题、运行状态
- 支持点击聚焦终端或 IDE
- 支持可靠来源的完成提醒
- 支持后续 IDE 插件主动上报状态

核心原则：UI 不直接关心具体工具，只消费统一会话模型。

## 设计原则

### 1. 统一模型优先

不要每接入一个 IDE 或 CLI 就单独堆一套 UI 和状态逻辑。

所有来源都先转换成统一模型，再进入 runtime、UI、宠物、提醒和聚焦逻辑。

### 2. 显式事件优先，推断兜底

状态可靠性排序：

1. 插件或 CLI 明确上报状态
2. CLI 进程、日志、状态文件推断
3. IDE 窗口标题或面板状态推断

完成提醒只应该依赖高可靠或中可靠信号，避免误报。

### 3. IDE 与 CLI 分层接入

CLI 更容易拿到进程、cwd、退出状态，适合优先做运行态和完成提醒。

IDE 第一阶段只做：

- 识别窗口
- 识别工作区
- 点击聚焦

IDE 内部 AI 状态建议通过插件事件协议解决，不建议长期依赖窗口标题猜测。

## 核心模型

### AgentSource

表示一个 AI 来源。

```rust
pub struct AgentSource {
    pub source_id: String,
    pub source_type: AgentSourceType,
    pub display_name: String,
    pub icon: Option<String>,
    pub priority: i32,
    pub enabled: bool,
}

pub enum AgentSourceType {
    Cli,
    Ide,
    Extension,
}
```

示例：

```text
codex-cli
gemini-cli
glm-cli
vscode
cursor
trae
vscode-codex-extension
```

### AgentSession

表示一个可展示、可聚焦、可提醒的 AI 会话。

```rust
pub struct AgentSession {
    pub source_id: String,
    pub session_id: String,
    pub workspace_path: Option<PathBuf>,
    pub title: String,
    pub status: AgentSessionStatus,
    pub pid: Option<u32>,
    pub window_id: Option<String>,
    pub terminal_tab: Option<String>,
    pub ide_bundle_id: Option<String>,
    pub started_at: Option<SystemTime>,
    pub last_activity_at: Option<SystemTime>,
    pub metadata: serde_json::Value,
}

pub enum AgentSessionStatus {
    Idle,
    Running,
    Waiting,
    Completed,
    Error,
}
```

## 适配器架构

建议新增统一 adapter trait：

```rust
pub trait AgentSourceAdapter {
    fn source(&self) -> AgentSource;
    fn detect_sessions(&self) -> Vec<AgentSession>;
    fn focus_session(&self, session: &AgentSession) -> anyhow::Result<()>;
}
```

推荐模块结构：

```text
agent_sources/
  mod.rs
  registry.rs
  model.rs
  cli/
    mod.rs
    codex.rs
    gemini.rs
    glm.rs
    claude.rs
  ide/
    mod.rs
    vscode.rs
    cursor.rs
    trae.rs
    jetbrains.rs
  extension/
    mod.rs
    protocol.rs
    local_event_server.rs
    event_store.rs
```

`registry` 负责：

- 注册来源
- 调度 adapter
- 合并会话
- 去重
- 排序
- 过滤禁用来源

## CLI 接入方案

### 识别方式

CLI 第一版通过以下信息识别：

- 进程名
- 命令参数
- cwd
- 父进程终端
- terminal window / tab 信息
- 可选状态文件或日志

### 推荐优先接入

第一批：

- Codex CLI
- Gemini CLI
- GLM CLI

第二批：

- Claude Code
- 其他 OpenAI / Anthropic / 本地模型 CLI

### CLI 状态判断

状态判断优先级：

1. 明确状态文件或事件：`running` / `waiting` / `completed` / `error`
2. 进程仍在运行：`running`
3. 进程退出且最近有活动：`completed`
4. 输出或日志出现错误特征：`error`
5. 无活动：`idle`

### CLI 聚焦

macOS：

- 根据 pid / cwd / terminal metadata 找到终端窗口
- 使用现有 terminal focus 能力聚焦 tab 或窗口

Windows：

- 根据进程树、窗口标题、终端 tab 信息聚焦
- 复用现有 Windows terminal focus 逻辑

## IDE 接入方案

### 第一阶段能力

IDE 第一阶段只做基础能力：

- 识别 IDE 是否打开
- 识别当前工作区
- 识别窗口标题
- 点击卡片时聚焦 IDE

不建议第一阶段做 IDE AI 完成提醒。

### VS Code

识别信息：

- 进程名：`Code`
- Bundle ID：`com.microsoft.VSCode`
- 窗口标题
- 命令行参数
- 最近 workspace 记录

### Cursor

识别信息：

- 进程名：`Cursor`
- Bundle ID
- 窗口标题
- workspace 信息

### Trae

识别信息：

- 进程名
- Bundle ID
- 窗口标题
- workspace 信息

Trae 需要单独调研它的进程结构和窗口标题格式。

### JetBrains

可作为后续扩展：

- IntelliJ IDEA
- WebStorm
- PyCharm
- RustRover

第一版可以不做 JetBrains，只保留模型扩展能力。

## 插件事件协议

长期建议为 IDE 插件提供本地事件协议。

推荐使用本地 HTTP 或 WebSocket：

```text
127.0.0.1:<port>/agent/events
```

事件示例：

```json
{
  "version": 1,
  "source_id": "vscode-codex",
  "source_type": "extension",
  "ide": "vscode",
  "workspace_path": "/path/to/project",
  "session_id": "abc",
  "title": "Refactor pet animation",
  "status": "running",
  "updated_at": 1778755200,
  "metadata": {}
}
```

支持事件：

```text
session_started
session_updated
session_waiting
session_completed
session_failed
session_closed
```

插件事件进入 `event_store` 后，再转换为统一 `AgentSession`。

## 会话合并策略

同一个项目可能同时存在多个来源：

- Cursor 打开项目
- Codex CLI 正在执行
- Gemini CLI 正在执行
- VS Code Codex 插件正在运行

合并规则：

- `workspace_path` 相同则归为同一项目组
- 插件事件优先级最高
- CLI 运行态优先于 IDE idle 态
- IDE 可作为项目容器入口
- 同一来源内按 `session_id` 去重

展示排序：

1. `Waiting`
2. `Running`
3. `Completed`
4. `Error`
5. `Idle`

同级按 `last_activity_at` 倒序。

## UI 调整

需要新增来源展示能力：

- 来源图标
- 来源名称 badge
- 多来源组合状态
- 项目分组
- 点击行为区分 CLI / IDE / Extension

点击行为：

- CLI：聚焦终端窗口或 tab
- IDE：聚焦 IDE 窗口
- Extension：优先聚焦 IDE + workspace

设置项：

- 启用 / 禁用来源
- 来源优先级
- 是否显示 idle IDE
- 是否启用完成提醒
- 是否启用本地事件服务
- 本地事件服务端口

## 完成提醒策略

完成提醒必须按可靠性分级。

高可靠：

- 插件主动上报 `completed`
- CLI 有明确 session 记录并正常结束

中可靠：

- CLI 输出出现明确完成特征
- CLI 状态文件更新为完成

低可靠：

- IDE 窗口标题变化
- AI 面板消失
- 进程空闲

第一版只对高可靠和中可靠信号触发完成提醒。

## 实现阶段

### 阶段 1：统一模型

目标：

- 新增 `AgentSource`
- 新增 `AgentSession`
- 新增 adapter trait
- 将当前 Codex 会话映射到新模型
- UI 表现保持不变

验收：

- 当前 Codex 功能不回退
- runtime 内可以拿到统一 `AgentSession`

### 阶段 2：CLI 多来源

目标：

- 接入 Gemini CLI
- 接入 GLM CLI
- 保留 Codex CLI
- 支持来源 badge
- 支持 CLI 点击聚焦

验收：

- 多个 CLI 可同时显示
- 不同 CLI 来源能正确区分
- 点击可聚焦对应终端

### 阶段 3：IDE 基础识别

目标：

- 接入 VS Code
- 接入 Cursor
- 接入 Trae
- 支持 IDE 窗口识别和聚焦

验收：

- 打开对应 IDE 后 EchoIsland 能识别来源
- 点击能聚焦到对应窗口
- 不触发不可靠完成提醒

### 阶段 4：插件协议

目标：

- 新增本地事件 server
- 新增事件协议
- 新增事件存储
- 支持插件事件转换为 `AgentSession`

验收：

- 本地请求可创建 / 更新 / 完成 session
- UI 能展示插件上报的状态
- 事件过期后能自动清理

### 阶段 5：插件适配

目标：

- VS Code Codex 插件适配
- Cursor 插件或内置 AI 事件桥
- Trae 事件桥

验收：

- 插件能主动上报运行态和完成态
- 完成提醒不依赖窗口标题猜测

### 阶段 6：体验完善

目标：

- 设置页完善
- 来源图标完善
- 多来源合并展示
- 完成提醒策略优化
- 错误状态展示

验收：

- 用户能控制来源开关
- 多来源场景展示清晰
- 完成提醒准确率可接受

## 第一版推荐范围

第一版建议只做：

- 统一 `AgentSession` 模型
- Codex CLI 迁移到新模型
- Gemini CLI
- GLM CLI
- VS Code / Cursor / Trae 窗口识别
- 点击聚焦
- 预留插件事件协议

第一版不建议做：

- IDE AI 完成提醒
- JetBrains 深度适配
- 复杂插件生态
- 基于窗口标题的强状态判断

这样风险最低，也能为后续插件协议打好结构基础。

## 主要风险

### 1. IDE 状态误判

IDE 内部 AI 状态很难通过窗口标题稳定判断。

解决方式：

- 第一版只做 IDE 识别和聚焦
- 完成提醒依赖插件事件

### 2. 多来源重复展示

同一项目可能被多个来源同时识别。

解决方式：

- 按 `workspace_path` 分组
- 按来源优先级排序
- CLI / Extension 作为主要状态，IDE 作为入口

### 3. 聚焦不稳定

不同平台窗口聚焦能力差异较大。

解决方式：

- 复用现有 terminal focus 能力
- IDE 聚焦先按窗口级实现
- tab / workspace 级聚焦作为增强项

### 4. 插件协议安全

本地事件 server 不能接受任意远程请求。

解决方式：

- 只绑定 `127.0.0.1`
- 加本地 token
- 限制 payload 大小
- 记录来源和版本

## 后续待调研

- Gemini CLI 进程名、参数、状态文件格式
- GLM CLI 进程名、参数、状态文件格式
- Trae macOS Bundle ID、进程名、窗口标题格式
- Cursor 工作区识别方式
- VS Code Codex 插件可用事件能力
- Windows 下 Cursor / Trae 窗口类名与聚焦方式
