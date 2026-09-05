# Windows 安装器维护

NSIS 使用 Tauri 官方 `SimpChinese` 语言、`currentUser` 模式和 HKCU 自启 hook。
MSI 使用官方 `zh-CN` UI 和 `zh-CN.wxl` 中的 Tauri 扩展文案，并采用 WiX 当前用户安装机制；自启由 `startup.wxs` 的独立组件管理，卸载只清除 EchoIsland 对应的注册值。

`main.wxs` 基于 Tauri CLI 2.10.1 的 [官方模板](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.10.1/crates/tauri-bundler/src/bundle/windows/msi/main.wxs)，沿用上游 Apache-2.0 / MIT 双许可。CN 修改仅涉及：`perUser` / `limited` 安装、LocalAppData 默认目录、协议注册使用 HKCU、可见快捷方式与功能标题中文。升级 Tauri CLI 时须对照同版本模板语义同步，不跳过 WiX 校验。

保留 `productName`、应用 identifier 和既有升级码算法，避免因文案变化生成另一应用身份。旧的管理员级 MSI 安装与当前用户 MSI 属于不同安装上下文；迁移前应通过 Windows“已安装的应用”卸载旧 MSI，再安装新版。NSIS 与 MSI 二选一，避免互相覆盖同一安装目录及自启值。

用户目录下的文件组件使用独立 HKCU 安装标记作为 KeyPath；主程序与 Hook 桥接组件分别使用按架构固定的新 GUID，这与应用的 UpgradeCode 是不同层级的身份。程序和附加二进制在主模板声明，唯一的 Windows 资源 `resources/echoisland-hook-bridge.exe` 在 `startup.wxs` 显式声明。该资源目录有对应的卸载 `RemoveFolder`，以通过 ICE38 / ICE64 校验。以后增加 Windows 资源时，须同时扩展此 fragment 与 `componentRefs`，不能恢复 Tauri 默认的文件 KeyPath 资源片段。

便携版不注册安装器自启，依靠 `EchoIsland.portable` 标识采用手动更新策略。
