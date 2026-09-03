# EchoIsland 简体中文增强版 —— 新电脑迁移与配置指南

本项目是基于 EchoIsland v0.6.1 的深度定制版本，由 GitHub 仓库 [CNDDVP/EchoIsland](https://github.com/CNDDVP/EchoIsland) 维护发布。

---

## 一、安装包介绍与选择

在 Release 发布包或 `EchoIsland-Release-v0.6.1-CN` 目录中，为你提供了三种格式：

1. **`EchoIsland_0.6.1_x64-setup.exe`（强烈推荐）**
   - Windows 官方标准安装程序。
   - 自动解压安装到 `%LOCALAPPDATA%\EchoIsland`，自动生成开始菜单快捷方式、桌面图标，并自动注册开机自启。
2. **`EchoIsland_0.6.1_x64.msi`**
   - 企业级 MSI 安装包，支持批量静默部署。
3. **`EchoIsland_v0.6.1_Windows_Portable.zip`**
   - 绿色免安装便携版。解压至任意目录（如 `D:\Tools\EchoIsland`），双击 `EchoIsland.exe` 即可直接运行，不写入系统注册表。

---

## 二、多 AI 会话接入说明

### 1. 官方原生工具（全自动，零配置）
- **Codex CLI**：新电脑只要有 `~/.codex`，启动 EchoIsland 会自动写入 hook 配置。
- **Claude Code**：新电脑只要有 `~/.claude`，启动 EchoIsland 会自动注册桥接。

### 2. 扩展工具（Antigravity、Kimi、ZCode、OpenClaw）
在 `integrations/` 文件夹中，我们已预置了一键配置脚本：
- 在新电脑上打开 PowerShell，进入 `integrations` 目录，执行：
  ```powershell
  .\setup-integrations.ps1
  ```
- 脚本会自动：
  1. 将 `ei-session-watcher.py` 和 `zcode-bridge.mjs` 复制到用户目录下的 `.echoisland/bin`；
  2. 自动在 Windows 开机启动项中加入无窗口静默守护进程（`EchoIslandSessionWatcher.lnk`）；
  3. 自动探测并配置 OpenClaw 插件。

---

## 三、潜在 Bug 排查与使用避坑提示

### ⚠️ 1. 勿使用应用内的“检查更新”覆盖
- **原因**：官方原版的自动更新源指向原作者的 GitHub Release。
- **注意**：如果在设置面板中点击“检查更新”，可能会下载并覆盖掉当前已修复 Bug 且完整汉化的定制版本。新版本更新请直接以 [CNDDVP/EchoIsland](https://github.com/CNDDVP/EchoIsland) 发布的 Release 为准。

### ⚠️ 2. 端口占用排查（37892 端口）
- EchoIsland 后台通过本地 HTTP 端口 `37892` 接收来自各个 AI CLI（包括 Antigravity 监控器）的活动事件。
- 如果迁移到的新电脑上安装了特定开发环境占用了 `37892` 端口，会导致悬浮岛收不到任务状态。可通过 `netstat -ano | findstr 37892` 检查是否有其他进程冲突。

### ⚠️ 3. 笔记本小屏幕与多显示器
- 本版本已将展开面板最大高度扩展到 **820px**，并支持多达 **16 个会话**同时展示。在 1080p、2K 及 4K 显示器上体验最佳。
- 若在 13 寸等高分屏笔记本上使用 200% 或更高的 Windows 缩放比例，且垂直分辨率较小时，可点击左上角设置齿轮选择“紧凑 (Compact)”宽度预设。
- 若使用了多个显示器，可在设置卡片中点击“显示器”进行主副屏切换。

### ⚠️ 4. Antigravity 会话监听依赖 Python 环境
- 监听脚本 `ei-session-watcher.py` 依赖新电脑上安装有 Python 3.x（只要命令行能执行 `python` 或 `pythonw` 即可）。
- 如果新电脑未安装 Python，Codex 和 Claude Code 仍然正常工作，但 Antigravity 和 Kimi 需要在安装 Python 后才能被监听器捕获。
