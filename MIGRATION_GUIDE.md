# EchoIsland 简体中文增强版 —— 新电脑迁移与配置指南

本项目是基于 EchoIsland v0.6.1 的深度定制版本，由 GitHub 仓库 [CNDDVP/EchoIsland](https://github.com/CNDDVP/EchoIsland) 维护发布。

---

## v0.6.2 更新内容（2026-09-05）

1. **修复：多显示器混合 DPI 下悬浮条飞出屏幕外的问题（重要）**
   - 现象：主屏 4K 150% 缩放 + 副屏 1080p 100% 缩放时，选择副屏作为首选显示器后，悬浮条窗口被定位到桌面范围之外（实测 x≈10300 物理像素，超出 5760 的桌面总宽），两块屏都看不到。
   - 修复：面板定位改用**目标显示器**的有效 DPI 换算坐标，并在 `SetWindowPos` 前把窗口钳制进目标显示器的物理边界内——无论上游数学如何出错，悬浮条永远落在所选显示器上。点击命中区域同步走同一路径，不会出现点击偏移。
   - 附带单元测试覆盖（越界钳制、负坐标、超大面板、退化尺寸）。
2. **修复：会话监听器（watcher）推送失败后不再丢事件**
   - 之前推送失败也会记入去重状态缓存，导致失败的那条会话永远不会重试（表现为悬浮条上缺卡片）。现在只有推送成功才记录状态，失败会在下个轮询周期（5 秒）自动重试。
3. **修复：`setup-integrations.ps1` 不再绑到微软商店的假 Python**
   - PATH 里的 `python.exe`/`pythonw.exe` 可能是商店 stub（运行无输出且退出），之前脚本会把它绑进开机自启，导致监听器开机后静默失败。现在按顺序查找：真实安装目录 → `py` 启动器反查 → `where.exe`（排除 WindowsApps）。
4. **新增：安装器自动注册开机自启**
   - 通过 NSIS 安装钩子写入 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`（仅当前用户，无需管理员），卸载时自动清理。旧版迁移用户可手动补一次或运行安装包升级。
5. **安全：应用内更新与"打开发布页"改指 CNDDVP 仓库**
   - 之前应用内"检查更新"指向原作 FunplayAI 的 Release，误点会下载原版覆盖汉化版。现在指向本仓库（本仓库暂未发布 updater 元数据，检查更新会提示失败，不会再覆盖安装；新版本请以 Release 页为准）。

---

## 一、安装包介绍与选择

在 Release 发布包或 `EchoIsland-Release-v0.6.1-CN` 目录中，为你提供了三种格式：

1. **`EchoIsland_0.6.2_x64-setup.exe`（强烈推荐）**
   - Windows 官方标准安装程序。
   - 自动解压安装到 `%LOCALAPPDATA%\EchoIsland`，自动生成开始菜单快捷方式、桌面图标，并自动注册开机自启。
2. **`EchoIsland_0.6.2_x64.msi`**
   - 企业级 MSI 安装包，支持批量静默部署。
3. **`EchoIsland_v0.6.2_Windows_Portable.zip`**
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
