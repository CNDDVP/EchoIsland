#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![allow(
    clippy::field_reassign_with_default,
    clippy::large_enum_variant,
    clippy::misnamed_getters,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

use std::sync::Arc;

use echoisland_runtime::SharedRuntime;
use tauri::{RunEvent, WindowEvent};
use tracing_subscriber::{EnvFilter, fmt};

mod app_runtime;
mod app_settings;
mod claude_scan;
mod codex_scan;
mod command_services;
mod commands;
mod diagnostics;
mod display_settings;
mod feishu_sidecar;
mod focus_store;
mod http_receiver;
#[cfg(target_os = "macos")]
mod macos_lifecycle_diagnostics;
#[cfg(target_os = "macos")]
mod macos_native_panel;
#[cfg(not(target_os = "macos"))]
#[path = "macos_native_panel_stub.rs"]
mod macos_native_panel;
mod native_panel_core;
mod native_panel_renderer;
mod native_panel_runtime;
mod native_panel_scene;
mod native_panel_scene_input;
mod native_ui_refresh;
mod notification_sound;
mod panel_scene_service;
mod platform;
mod platform_stub;
mod process_source_scan;
mod session_scan_runner;
mod startup_service;
mod terminal_focus;
mod terminal_focus_service;
mod tray;
mod updater_service;
mod windows_native_panel;

use app_runtime::{AppRuntime, spawn_ipc_server};
use claude_scan::spawn_claude_scan_loop;
use codex_scan::spawn_codex_scan_loop;
use commands::{
    answer_question, approve_permission, bind_session_terminal, build_status_surface_scene,
    check_for_update, claude_status, codex_status, cycle_display, deny_permission,
    download_and_install_update, focus_session_terminal, get_app_settings, get_available_displays,
    get_snapshot, get_snapshot_status_surface_bundle, get_update_status, hide_main_window,
    http_receiver_status, ingest_sample, ipc_addr, open_release_page, open_settings_location,
    openclaw_status, platform_capabilities, platform_paths, quit_application,
    set_completion_sound_enabled, set_island_bar_stage, set_island_bar_stage_passive,
    set_island_expanded, set_island_expanded_passive, set_island_panel_stage,
    set_island_panel_stage_passive, set_mascot_enabled, set_preferred_display_index,
    show_main_window_interactive, skip_question,
};
use http_receiver::spawn_http_receiver;
use native_panel_renderer::facade::runtime::{
    NativePanelRuntimeBackend, current_native_panel_runtime_backend,
};
use panel_scene_service::PanelSceneState;
use process_source_scan::spawn_process_source_scan_loops;
use startup_service::AppStartupService;

#[cfg(target_os = "windows")]
static WINDOWS_SINGLE_INSTANCE_MUTEX: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
#[cfg(target_os = "macos")]
static MACOS_SINGLE_INSTANCE_REOPEN_TX: std::sync::OnceLock<std::sync::mpsc::Sender<()>> =
    std::sync::OnceLock::new();
#[cfg(target_os = "macos")]
static MACOS_PENDING_SINGLE_INSTANCE_REOPEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn main() {
    ensure_single_instance_or_exit();
    setup_tracing();
    diagnostics::log_diagnostic_event(
        "app_start",
        &[
            diagnostics::current_process_fields(),
            vec![(
                "diagnostic_log",
                diagnostics::diagnostic_log_path().display().to_string(),
            )],
        ]
        .concat(),
    );
    if app_settings::current_app_settings().debug_mode_enabled {
        diagnostics::log_debug_mode_snapshot();
    }

    let runtime = Arc::new(SharedRuntime::new());
    let app_runtime = AppRuntime::new(runtime.clone());

    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_runtime.clone())
        .manage(updater_service::AppUpdateState::default())
        .manage(PanelSceneState::default())
        .on_window_event(|window, event| {
            diagnostics::log_diagnostic_event(
                "tauri_window_event",
                &[
                    ("label", window.label().to_string()),
                    ("event", window_event_name(event).to_string()),
                ],
            );
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            diagnostics::log_diagnostic_event("tauri_setup_begin", &[]);
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            #[cfg(target_os = "macos")]
            macos_lifecycle_diagnostics::install_macos_lifecycle_diagnostics();
            let app_handle = app.handle().clone();
            let native_panel_backend = current_native_panel_runtime_backend();
            if native_panel_backend.native_ui_enabled() {
                native_panel_backend
                    .create_panel()
                    .map_err(std::io::Error::other)?;
                native_panel_backend
                    .hide_legacy_app_window(&app_handle)
                    .map_err(std::io::Error::other)?;
                native_panel_runtime::spawn_native_snapshot_loop(
                    app_handle.clone(),
                    app_runtime.clone(),
                );
                native_panel_backend.spawn_platform_loops(app_handle.clone());
                #[cfg(target_os = "macos")]
                install_macos_single_instance_reopen_handler(app_handle.clone());
                #[cfg(target_os = "macos")]
                terminal_focus::prewarm_codex_app_deeplink_handler();
            } else {
                tracing::warn!(
                    "native panel backend is disabled; no WebView fallback is available"
                );
            }
            AppStartupService::new(app)
                .initialize()
                .map_err(std::io::Error::other)?;
            spawn_codex_scan_loop(runtime.clone());
            spawn_claude_scan_loop(runtime.clone());
            spawn_process_source_scan_loops(runtime.clone());
            spawn_ipc_server(app_handle, app_runtime.clone());
            spawn_http_receiver(app.handle().clone(), runtime.clone());
            feishu_sidecar::spawn_feishu_sidecar(app.handle().clone(), runtime.clone());
            diagnostics::log_diagnostic_event("tauri_setup_complete", &[]);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_snapshot_status_surface_bundle,
            build_status_surface_scene,
            get_app_settings,
            get_available_displays,
            ingest_sample,
            ipc_addr,
            codex_status,
            claude_status,
            openclaw_status,
            http_receiver_status,
            platform_capabilities,
            platform_paths,
            approve_permission,
            deny_permission,
            answer_question,
            skip_question,
            set_island_bar_stage,
            set_island_bar_stage_passive,
            set_island_panel_stage,
            set_island_panel_stage_passive,
            set_island_expanded,
            set_island_expanded_passive,
            show_main_window_interactive,
            hide_main_window,
            open_settings_location,
            open_release_page,
            get_update_status,
            check_for_update,
            download_and_install_update,
            set_completion_sound_enabled,
            set_mascot_enabled,
            set_preferred_display_index,
            cycle_display,
            quit_application,
            focus_session_terminal,
            bind_session_terminal
        ])
        .build(tauri::generate_context!())
        .expect("failed to build tauri app")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = &event
            {
                api.prevent_exit();
            }
            log_tauri_run_event(&event);
        });
}

fn window_event_name(event: &WindowEvent) -> &'static str {
    match event {
        WindowEvent::Resized(_) => "resized",
        WindowEvent::Moved(_) => "moved",
        WindowEvent::CloseRequested { .. } => "close_requested",
        WindowEvent::Destroyed => "destroyed",
        WindowEvent::Focused(true) => "focused_true",
        WindowEvent::Focused(false) => "focused_false",
        WindowEvent::ScaleFactorChanged { .. } => "scale_factor_changed",
        WindowEvent::DragDrop(_) => "drag_drop",
        WindowEvent::ThemeChanged(_) => "theme_changed",
        _ => "unknown",
    }
}

fn log_tauri_run_event(event: &RunEvent) {
    let Some((event_name, mut fields)) = run_event_diagnostic_fields(event) else {
        return;
    };
    fields.extend(diagnostics::current_process_fields());
    diagnostics::log_diagnostic_event(event_name, &fields);
}

fn run_event_diagnostic_fields(
    event: &RunEvent,
) -> Option<(&'static str, Vec<(&'static str, String)>)> {
    match event {
        RunEvent::Ready => Some(("tauri_run_event", vec![("event", "ready".to_string())])),
        RunEvent::Resumed => Some(("tauri_run_event", vec![("event", "resumed".to_string())])),
        RunEvent::Exit => Some(("tauri_run_event", vec![("event", "exit".to_string())])),
        RunEvent::ExitRequested { code, .. } => Some((
            "tauri_run_event",
            vec![
                ("event", "exit_requested".to_string()),
                (
                    "code",
                    code.map(|value| value.to_string()).unwrap_or_default(),
                ),
            ],
        )),
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => Some((
            "tauri_run_event",
            vec![
                ("event", "opened".to_string()),
                ("url_count", urls.len().to_string()),
                (
                    "urls",
                    urls.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                ),
            ],
        )),
        #[cfg(target_os = "macos")]
        RunEvent::Reopen {
            has_visible_windows,
            ..
        } => Some((
            "tauri_run_event",
            vec![
                ("event", "reopen".to_string()),
                ("has_visible_windows", has_visible_windows.to_string()),
            ],
        )),
        _ => None,
    }
}

fn setup_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).with_target(false).try_init();
}

#[cfg(target_os = "windows")]
fn ensure_single_instance_or_exit() {
    use std::{iter, ptr};
    use windows_sys::Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError, SetLastError},
        System::Threading::CreateMutexW,
    };

    let name: Vec<u16> = "Local\\com.echoisland.desktop.single-instance"
        .encode_utf16()
        .chain(iter::once(0))
        .collect();
    unsafe {
        SetLastError(0);
        let handle = CreateMutexW(ptr::null(), 0, name.as_ptr());
        if handle.is_null() {
            return;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            std::process::exit(0);
        }
        let _ = WINDOWS_SINGLE_INSTANCE_MUTEX.set(handle as usize);
    }
}

#[cfg(target_os = "macos")]
fn ensure_single_instance_or_exit() {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    let socket_path = macos_single_instance_socket_path();
    if let Ok(mut stream) = UnixStream::connect(&socket_path) {
        let _ = stream.write_all(b"reopen\n");
        std::process::exit(0);
    }

    let _ = std::fs::remove_file(&socket_path);
    match UnixListener::bind(&socket_path) {
        Ok(listener) => {
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else {
                        break;
                    };
                    let mut buffer = [0_u8; 64];
                    let _ = stream.read(&mut buffer);
                    if let Some(tx) = MACOS_SINGLE_INSTANCE_REOPEN_TX.get() {
                        let _ = tx.send(());
                    } else {
                        MACOS_PENDING_SINGLE_INSTANCE_REOPEN
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
            });
        }
        Err(_) => {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                let _ = stream.write_all(b"reopen\n");
                std::process::exit(0);
            }
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn ensure_single_instance_or_exit() {}

#[cfg(target_os = "macos")]
fn macos_single_instance_socket_path() -> std::path::PathBuf {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let exe_key = std::env::current_exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "unknown-exe".to_string());
    let mut hasher = DefaultHasher::new();
    exe_key.hash(&mut hasher);
    let exe_hash = hasher.finish();
    std::env::temp_dir().join(format!(
        "com.echoisland.desktop.single-instance.{profile}.{exe_hash:x}.sock"
    ))
}

#[cfg(target_os = "macos")]
fn install_macos_single_instance_reopen_handler<R: tauri::Runtime + 'static>(
    app_handle: tauri::AppHandle<R>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    if MACOS_SINGLE_INSTANCE_REOPEN_TX.set(tx).is_err() {
        return;
    }
    if MACOS_PENDING_SINGLE_INSTANCE_REOPEN.swap(false, std::sync::atomic::Ordering::SeqCst)
        && let Some(tx) = MACOS_SINGLE_INSTANCE_REOPEN_TX.get()
    {
        let _ = tx.send(());
    }

    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            let run_result = app_handle.run_on_main_thread(|| {
                if let Err(error) = macos_native_panel::show_existing_or_create_native_panel() {
                    tracing::warn!(error, "failed to reopen existing macOS native panel");
                }
            });
            if let Err(error) = run_result {
                tracing::warn!(error = %error, "failed to dispatch macOS reopen request");
            }
        }
    });
}
