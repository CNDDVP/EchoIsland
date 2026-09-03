# ==============================================================================
# EchoIsland 扩展 AI 工具接入配置脚本 (适用于新电脑一键迁移)
# 支持: Antigravity, Kimi CLI, ZCode, OpenClaw
# ==============================================================================
$ErrorActionPreference = "Stop"

Write-Host ">>> 正在为当前电脑配置 EchoIsland 多 AI 监听组件..." -ForegroundColor Cyan

$userHome = [System.Environment]::GetFolderPath("UserProfile")
$echoIslandBin = Join-Path $userHome ".echoisland\bin"
$startupDir = [System.IO.Path]::Combine($env:APPDATA, "Microsoft\Windows\Start Menu\Programs\Startup")

# 1. 确保目录存在
if (-not (Test-Path $echoIslandBin)) {
    New-Item -ItemType Directory -Path $echoIslandBin -Force | Out-Null
}

$scriptDir = $PSScriptRoot

# 2. 安装监听脚本
Copy-Item (Join-Path $scriptDir "ei-session-watcher.py") $echoIslandBin -Force
Copy-Item (Join-Path $scriptDir "zcode-bridge.mjs") $echoIslandBin -Force
Write-Host "[✓] 已将监听脚本部署至: $echoIslandBin" -ForegroundColor Green

# 3. 配置 OpenClaw 插件 (若存在)
$openclawDir = Join-Path $userHome ".openclaw"
if (Test-Path $openclawDir) {
    $openclawPluginDir = Join-Path $openclawDir "echoisland-plugin"
    New-Item -ItemType Directory -Path $openclawPluginDir -Force | Out-Null
    Copy-Item (Join-Path $scriptDir "openclaw-plugin\*") $openclawPluginDir -Force
    Write-Host "[✓] 已配置 OpenClaw 插件: $openclawPluginDir" -ForegroundColor Green
}

# 4. 配置开机自启监听 (使用 pythonw 静默无窗口运行)
$pythonw = (Get-Command "pythonw.exe" -ErrorAction SilentlyContinue)?.Source
if (-not $pythonw) {
    $pythonw = (Get-Command "python.exe" -ErrorAction SilentlyContinue)?.Source
}

if ($pythonw) {
    $wsh = New-Object -ComObject WScript.Shell
    $shortcutPath = Join-Path $startupDir "EchoIslandSessionWatcher.lnk"
    $shortcut = $wsh.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $pythonw
    $shortcut.Arguments = "`"$echoIslandBin\ei-session-watcher.py`""
    $shortcut.WorkingDirectory = $echoIslandBin
    $shortcut.WindowStyle = 7 # Minimized
    $shortcut.Save()
    Write-Host "[✓] 已添加开机自启快捷方式: $shortcutPath" -ForegroundColor Green

    # 启动当前监听进程
    Start-Process $pythonw -ArgumentList "`"$echoIslandBin\ei-session-watcher.py`""
    Write-Host "[✓] Antigravity & Kimi 会话监听器已在后台运行！" -ForegroundColor Green
} else {
    Write-Host "[!] 未检测到系统中的 Python，请安装 Python 3 以启用 Antigravity / Kimi 自动监听。" -ForegroundColor Yellow
}

Write-Host "`n全部配置完成！请确保 EchoIsland 主程序已启动。" -ForegroundColor Cyan
