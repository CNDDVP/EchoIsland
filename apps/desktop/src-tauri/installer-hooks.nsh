; EchoIsland NSIS installer hooks (Tauri 2 bundle.windows.nsis.installerHooks)
; 安装后注册 HKCU 开机自启，卸载时清理。仅写当前用户注册表，无需管理员权限。

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "EchoIsland" "$INSTDIR\echoisland-desktop.exe"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "EchoIsland"
  DeleteRegKey /ifempty HKCU "Software\Microsoft\Windows\CurrentVersion\Run"
!macroend
