use tauri::{
    AppHandle, Emitter,
    menu::{MenuBuilder, MenuId},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

#[cfg(not(target_os = "macos"))]
use crate::native_panel_renderer::facade::runtime::{
    NativePanelRuntimeBackend, current_native_panel_runtime_backend,
};

const TRAY_ID: &str = "main-tray";
const MENU_SHOW: &str = "tray_show";
const MENU_REFRESH: &str = "tray_refresh";
const MENU_QUIT: &str = "tray_quit";

pub fn build_tray<R: tauri::Runtime>(app: &mut tauri::App<R>) -> tauri::Result<()> {
    let show_id = MenuId::new(MENU_SHOW);
    let refresh_id = MenuId::new(MENU_REFRESH);
    let quit_id = MenuId::new(MENU_QUIT);

    let menu = MenuBuilder::new(app)
        .text(MENU_SHOW, echoisland_i18n::t("app.show"))
        .text(MENU_REFRESH, echoisland_i18n::t("app.refresh"))
        .separator()
        .text(MENU_QUIT, echoisland_i18n::t("app.quit"))
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound(echoisland_i18n::t("error.icon").into()))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(icon)
        .tooltip("EchoIsland")
        .show_menu_on_left_click(false)
        .on_menu_event(move |app: &AppHandle<_>, event: tauri::menu::MenuEvent| {
            let id = event.id();
            if id == &show_id {
                let _ = show_echoisland_surface(app);
            } else if id == &refresh_id {
                let _ = emit_refresh(app);
            } else if id == &quit_id {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray: &TrayIcon<_>, event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_echoisland_surface(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_echoisland_surface<R: tauri::Runtime>(_app: &AppHandle<R>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        crate::macos_native_panel::show_existing_or_create_native_panel_with_app(_app)
    }

    #[cfg(not(target_os = "macos"))]
    {
        show_echoisland_surface_for_native_backend(_app)
    }
}

#[cfg(not(target_os = "macos"))]
fn show_echoisland_surface_for_native_backend<R: tauri::Runtime>(
    _app: &AppHandle<R>,
) -> Result<(), String> {
    let native_panel_backend = current_native_panel_runtime_backend();
    if native_panel_backend.native_ui_enabled() {
        return native_panel_backend.create_panel();
    }

    Ok(())
}

fn emit_refresh<R: tauri::Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.emit("tray-refresh", true)
}
