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
$localeSource = Join-Path $scriptDir "..\crates\i18n\locales"
if (-not (Test-Path -LiteralPath $localeSource)) { $localeSource = Join-Path $scriptDir "locales" }
$watcherLocales = Join-Path $echoIslandBin "locales"
New-Item -ItemType Directory -Path $watcherLocales -Force | Out-Null
Copy-Item (Join-Path $localeSource "*.json") $watcherLocales -Force
Write-Host "[✓] 已将监听脚本部署至: $echoIslandBin" -ForegroundColor Green

# 3. 配置 OpenClaw 插件 (若存在)
$openclawDir = Join-Path $userHome ".openclaw"
if (Test-Path $openclawDir) {
    $openclawPluginDir = Join-Path $openclawDir "echoisland-plugin"
    New-Item -ItemType Directory -Path $openclawPluginDir -Force | Out-Null
    Copy-Item (Join-Path $scriptDir "openclaw-plugin\*") $openclawPluginDir -Force
    $pluginLocales = Join-Path $openclawPluginDir "locales"
    New-Item -ItemType Directory -Path $pluginLocales -Force | Out-Null
    Copy-Item (Join-Path $localeSource "*.json") $pluginLocales -Force
    Write-Host "[✓] 已复制 OpenClaw 插件: $openclawPluginDir；请运行 cargo run -p desktop-host -- install-openclaw 完成显式安装。" -ForegroundColor Green
}

# 4. 配置开机自启监听 (使用 pythonw 静默无窗口运行)
# 注意：PATH 里的 python/pythonw 可能是微软商店的假 stub（运行无输出且退出），
# 必须优先找真实安装的解释器，否则开机自启会静默失败。
function Find-RealPythonw {
    # 1) 常见安装目录（官方安装包默认位置）
    $found = Get-ChildItem "$env:LOCALAPPDATA\Programs\Python\Python3*\pythonw.exe", `
        "C:\Program Files\Python3*\pythonw.exe", `
        "C:\Program Files (x86)\Python3*\pythonw.exe", `
        "C:\Python3*\pythonw.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) { return $found.FullName }

    # 2) py 启动器反查真实解释器路径
    $py = Get-Command "py.exe" -ErrorAction SilentlyContinue
    if ($py) {
        try {
            $resolved = & $py.Source -c "import os, sys; print(os.path.join(os.path.dirname(sys.executable), 'pythonw.exe'))" 2>$null
            if ($resolved) { $resolved = ([string]$resolved).Trim(); if ($resolved -and ($resolved -notmatch 'WindowsApps') -and (Test-Path $resolved)) { return $resolved } }
        } catch {}
    }

    # 3) where.exe 查找，排除商店 stub
    $fromWhere = where.exe pythonw.exe 2>$null | Where-Object { $_ -and ($_ -notmatch 'WindowsApps') } | Select-Object -First 1
    if ($fromWhere) { return $fromWhere }

    return $null
}

$pythonw = Find-RealPythonw

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
    Start-Process $pythonw -ArgumentList "`"$echoIslandBin\ei-session-watcher.py`"" -WindowStyle Hidden
    Write-Host "[✓] Antigravity & Kimi 会话监听器已在后台运行！" -ForegroundColor Green
} else {
    Write-Host "[!] 未检测到系统中的 Python，请安装 Python 3 以启用 Antigravity / Kimi 自动监听。" -ForegroundColor Yellow
}

Write-Host "`n全部配置完成！请确保 EchoIsland 主程序已启动。" -ForegroundColor Cyan
