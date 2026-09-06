use base64::Engine;
use serde::Serialize;
use std::future::Future;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use tracing::warn;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AppUpdatePhase {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Installing,
    Installed,
    Failed,
    UnsupportedPortable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppUpdateStatus {
    pub(crate) phase: AppUpdatePhase,
    pub(crate) label: String,
    pub(crate) value_text: String,
    pub(crate) version: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) can_check: bool,
    pub(crate) can_install: bool,
    pub(crate) can_open_release_page: bool,
}

impl AppUpdateStatus {
    pub(crate) fn idle() -> Self {
        Self {
            phase: AppUpdatePhase::Idle,
            label: echoisland_i18n::t("update.check").to_string(),
            value_text: echoisland_i18n::t("update.check_action").to_string(),
            version: None,
            notes: None,
            error: None,
            can_check: true,
            can_install: false,
            can_open_release_page: true,
        }
    }

    fn checking() -> Self {
        Self {
            phase: AppUpdatePhase::Checking,
            label: echoisland_i18n::t("update.checking").to_string(),
            value_text: echoisland_i18n::t("update.checking_action").to_string(),
            version: None,
            notes: None,
            error: None,
            can_check: false,
            can_install: false,
            can_open_release_page: false,
        }
    }

    fn up_to_date() -> Self {
        Self {
            phase: AppUpdatePhase::UpToDate,
            label: echoisland_i18n::t("update.check").to_string(),
            value_text: echoisland_i18n::t("update.latest").to_string(),
            version: None,
            notes: None,
            error: None,
            can_check: true,
            can_install: false,
            can_open_release_page: true,
        }
    }

    fn available(version: String, notes: Option<String>) -> Self {
        Self {
            phase: AppUpdatePhase::Available,
            label: echoisland_i18n::format("update.available", &[("version", &version)]),
            value_text: echoisland_i18n::t("update.install_action").to_string(),
            version: Some(version),
            notes,
            error: None,
            can_check: true,
            can_install: true,
            can_open_release_page: true,
        }
    }

    fn downloading(version: Option<String>) -> Self {
        Self {
            phase: AppUpdatePhase::Downloading,
            label: echoisland_i18n::t("update.downloading").to_string(),
            value_text: echoisland_i18n::t("update.downloading_action").to_string(),
            version,
            notes: None,
            error: None,
            can_check: false,
            can_install: false,
            can_open_release_page: false,
        }
    }

    fn installing(version: Option<String>) -> Self {
        Self {
            phase: AppUpdatePhase::Installing,
            label: echoisland_i18n::t("update.installing").to_string(),
            value_text: echoisland_i18n::t("update.installing_action").to_string(),
            version,
            notes: None,
            error: None,
            can_check: false,
            can_install: false,
            can_open_release_page: false,
        }
    }

    fn installed(version: Option<String>) -> Self {
        Self {
            phase: AppUpdatePhase::Installed,
            label: echoisland_i18n::t("update.installed").to_string(),
            value_text: echoisland_i18n::t("update.restarting").to_string(),
            version,
            notes: None,
            error: None,
            can_check: false,
            can_install: false,
            can_open_release_page: false,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            phase: AppUpdatePhase::Failed,
            label: echoisland_i18n::t("update.failed").to_string(),
            value_text: echoisland_i18n::t("update.release_page").to_string(),
            version: None,
            notes: None,
            error: Some(error),
            can_check: true,
            can_install: false,
            can_open_release_page: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PortableUpdatePolicy {
    Installed,
    Portable,
}

#[derive(Default)]
pub(crate) struct AppUpdateState {
    status: Mutex<AppUpdateStatus>,
    pending_update: Mutex<Option<Update>>,
}

impl Default for AppUpdateStatus {
    fn default() -> Self {
        Self::idle()
    }
}

static FALLBACK_UPDATE_STATUS: OnceLock<Mutex<AppUpdateStatus>> = OnceLock::new();

// Historical upstream key is not a CNDDVP release identity. Never enable installation
// merely because someone has uploaded latest.json to the fork.
const UPSTREAM_PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ2NDEzNzI5MDlGQkY2OApSV1JvdjUrUWNoTmtCR0d4bWVkSGs0WHA5dE9wZTlFc0hrU0FJSUMwaS9qd016c3lvcnhGeGlNNgo=";

fn dedicated_signing_is_configured(config: &serde_json::Value) -> bool {
    config["bundle"]["createUpdaterArtifacts"] == true
        && config["plugins"]["updater"]["pubkey"]
            .as_str()
            .and_then(signing_key_material)
            .is_some_and(|key| Some(key) != signing_key_material(UPSTREAM_PUBLIC_KEY))
}

fn signing_key_material(encoded: &str) -> Option<Vec<u8>> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()?;
    let text = std::str::from_utf8(&decoded).ok()?;
    let mut lines = text.lines();
    lines.next()?.strip_prefix("untrusted comment:")?;
    let key = base64::engine::general_purpose::STANDARD
        .decode(lines.next()?.trim())
        .ok()?;
    // Minisign's public packet is algorithm (2), key id (8), Ed25519 key (32).
    // The comment and key id do not establish a distinct signing identity.
    (key.len() == 42 && &key[..2] == b"Ed").then(|| key[10..].to_vec())
}

async fn bounded_update_check<T, E: std::fmt::Display>(
    check: impl Future<Output = Result<T, E>>,
    timeout: Duration,
) -> Result<T, String> {
    match tokio::time::timeout(timeout, check).await {
        Ok(result) => result.map_err(|error| error.to_string()),
        Err(_) => Err("update metadata request timed out".to_string()),
    }
}

async fn check_update<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<Option<Update>, String> {
    let updater = app.updater().map_err(|error| error.to_string())?;
    bounded_update_check(updater.check(), Duration::from_secs(30)).await
}

fn cn_signing_ready() -> bool {
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        serde_json::from_str(include_str!("../tauri.conf.json"))
            .map(|config| dedicated_signing_is_configured(&config))
            .unwrap_or(false)
    })
}

fn is_cn_release_asset(url: &tauri::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url
            .path()
            .starts_with("/CNDDVP/EchoIsland/releases/download/")
        && !url.path().contains('%')
        && url.path().split('/').count() == 7
        && !url.path().ends_with('/')
}

fn update_installation_error(url: &tauri::Url, signing_ready: bool) -> Option<&'static str> {
    if !is_cn_release_asset(url) {
        Some(echoisland_i18n::t("update.untrusted_source"))
    } else if !signing_ready {
        Some(echoisland_i18n::t("update.manual_signing"))
    } else {
        None
    }
}

pub(crate) fn current_update_status() -> AppUpdateStatus {
    fallback_update_status()
        .lock()
        .map(|status| status.clone())
        .unwrap_or_else(|_| {
            AppUpdateStatus::failed(echoisland_i18n::t("update.status_unavailable").to_string())
        })
}

pub(crate) fn update_status_for_portable_policy(policy: PortableUpdatePolicy) -> AppUpdateStatus {
    match policy {
        PortableUpdatePolicy::Installed => AppUpdateStatus::idle(),
        PortableUpdatePolicy::Portable => AppUpdateStatus {
            phase: AppUpdatePhase::UnsupportedPortable,
            label: echoisland_i18n::t("update.portable").to_string(),
            value_text: echoisland_i18n::t("update.release_page").to_string(),
            version: None,
            notes: None,
            error: Some(echoisland_i18n::t("update.portable_help").to_string()),
            can_check: false,
            can_install: false,
            can_open_release_page: true,
        },
    }
}

pub(crate) fn detect_update_policy() -> PortableUpdatePolicy {
    if std::env::var("ECHOISLAND_PORTABLE")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
    {
        return PortableUpdatePolicy::Portable;
    }
    if portable_marker_exists_near_current_exe() {
        return PortableUpdatePolicy::Portable;
    }

    if cfg!(debug_assertions) {
        return PortableUpdatePolicy::Installed;
    }

    if tauri::utils::platform::bundle_type().is_none() {
        PortableUpdatePolicy::Portable
    } else {
        PortableUpdatePolicy::Installed
    }
}

pub(crate) fn app_update_status_from_state(state: &AppUpdateState) -> AppUpdateStatus {
    state
        .status
        .lock()
        .map(|status| status.clone())
        .unwrap_or_else(|_| {
            AppUpdateStatus::failed(echoisland_i18n::t("update.status_unavailable").to_string())
        })
}

pub(crate) async fn check_for_update<R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &AppUpdateState,
) -> AppUpdateStatus {
    if detect_update_policy() == PortableUpdatePolicy::Portable {
        let status = update_status_for_portable_policy(PortableUpdatePolicy::Portable);
        set_update_status(state, status.clone());
        return status;
    }

    // A failed/new check must not leave a previous installable update behind.
    let _ = take_pending_update(state);
    set_update_status(state, AppUpdateStatus::checking());
    let update_check = check_update(app).await;

    match update_check {
        Ok(Some(update)) => {
            if let Some(error) = update_installation_error(&update.download_url, cn_signing_ready())
            {
                let status = AppUpdateStatus::failed(error.to_string());
                set_update_status(state, status.clone());
                return status;
            }
            let status = AppUpdateStatus::available(update.version.clone(), update.body.clone());
            if let Ok(mut pending_update) = state.pending_update.lock() {
                pending_update.replace(update);
            }
            set_update_status(state, status.clone());
            status
        }
        Ok(None) => {
            let status = AppUpdateStatus::up_to_date();
            set_update_status(state, status.clone());
            status
        }
        Err(error) => {
            let status =
                AppUpdateStatus::failed(echoisland_i18n::error("update.failure_help", error));
            set_update_status(state, status.clone());
            status
        }
    }
}

pub(crate) async fn download_and_install_update<R: tauri::Runtime>(
    app: &AppHandle<R>,
    state: &AppUpdateState,
) -> AppUpdateStatus {
    if detect_update_policy() == PortableUpdatePolicy::Portable {
        let status = update_status_for_portable_policy(PortableUpdatePolicy::Portable);
        set_update_status(state, status.clone());
        return status;
    }

    if !cn_signing_ready() {
        let _ = take_pending_update(state);
        let status =
            AppUpdateStatus::failed(echoisland_i18n::t("update.manual_signing").to_string());
        set_update_status(state, status.clone());
        return status;
    }

    let update = match take_pending_update(state) {
        Some(update) => update,
        None => match check_update(app).await {
            Ok(Some(update)) => update,
            Ok(None) => {
                let status = AppUpdateStatus::up_to_date();
                set_update_status(state, status.clone());
                return status;
            }
            Err(error) => {
                let status =
                    AppUpdateStatus::failed(echoisland_i18n::error("update.failure_help", error));
                set_update_status(state, status.clone());
                return status;
            }
        },
    };

    if let Some(error) = update_installation_error(&update.download_url, cn_signing_ready()) {
        let status = AppUpdateStatus::failed(error.to_string());
        set_update_status(state, status.clone());
        return status;
    }
    let version = Some(update.version.clone());
    set_update_status(state, AppUpdateStatus::downloading(version.clone()));
    let installing_version = version.clone();
    let result = update
        .download_and_install(
            |_chunk_length, _content_length| {},
            || set_update_status(state, AppUpdateStatus::installing(installing_version)),
        )
        .await;

    match result {
        Ok(()) => {
            let status = AppUpdateStatus::installed(version);
            set_update_status(state, status.clone());
            status
        }
        Err(error) => {
            let status =
                AppUpdateStatus::failed(echoisland_i18n::error("update.failure_help", error));
            set_update_status(state, status.clone());
            status
        }
    }
}

pub(crate) fn spawn_native_update_flow<R: tauri::Runtime + 'static>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppUpdateState>();
        let status = app_update_status_from_state(&state);
        // A previous failure stays retryable: re-run the check instead of only
        // bouncing to the release page.
        if open_release_page_on_portable(&status) {
            let _ = crate::commands::open_release_page();
            return;
        }
        let next_status = if status.can_install {
            download_and_install_update(&app, &state).await
        } else {
            check_for_update(&app, &state).await
        };
        if open_release_page_on_portable_or_failed(&next_status) {
            warn!(
                error = ?next_status.error,
                "native update flow falling back to release page"
            );
            let _ = crate::commands::open_release_page();
        }
        refresh_native_panel_after_update_status_change(&app);
    });
}

pub(crate) fn open_release_page_on_portable(status: &AppUpdateStatus) -> bool {
    matches!(status.phase, AppUpdatePhase::UnsupportedPortable)
}

pub(crate) fn open_release_page_on_portable_or_failed(status: &AppUpdateStatus) -> bool {
    matches!(
        status.phase,
        AppUpdatePhase::UnsupportedPortable | AppUpdatePhase::Failed
    )
}

fn set_update_status(state: &AppUpdateState, status: AppUpdateStatus) {
    if let Ok(mut current) = state.status.lock() {
        *current = status.clone();
    }
    if let Ok(mut fallback) = fallback_update_status().lock() {
        *fallback = status;
    }
}

fn take_pending_update(state: &AppUpdateState) -> Option<Update> {
    state
        .pending_update
        .lock()
        .ok()
        .and_then(|mut pending_update| pending_update.take())
}

fn fallback_update_status() -> &'static Mutex<AppUpdateStatus> {
    FALLBACK_UPDATE_STATUS.get_or_init(|| Mutex::new(AppUpdateStatus::idle()))
}

fn portable_marker_exists_near_current_exe() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|parent| parent.join("EchoIsland.portable"))
        })
        .is_some_and(|marker| marker.exists())
}

fn refresh_native_panel_after_update_status_change<R: tauri::Runtime>(app: &AppHandle<R>) {
    use crate::native_panel_renderer::facade::runtime::{
        NativePanelRuntimeBackend, current_native_panel_runtime_backend,
    };

    let backend = current_native_panel_runtime_backend();
    if backend.native_ui_enabled() {
        let _ = backend.refresh_from_last_snapshot(app);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppUpdatePhase, AppUpdateStatus, PortableUpdatePolicy, UPSTREAM_PUBLIC_KEY,
        bounded_update_check, dedicated_signing_is_configured, is_cn_release_asset,
        open_release_page_on_portable, update_installation_error,
        update_status_for_portable_policy,
    };
    use base64::Engine;

    #[test]
    fn failed_status_stays_retryable_instead_of_release_page_bounce() {
        let status = AppUpdateStatus::failed("network down".to_string());

        assert!(!open_release_page_on_portable(&status));
        assert!(status.can_check);
        assert_eq!(status.phase, AppUpdatePhase::Failed);
    }

    #[test]
    fn portable_policy_reports_manual_download_fallback() {
        let status = update_status_for_portable_policy(PortableUpdatePolicy::Portable);

        assert_eq!(status.phase, AppUpdatePhase::UnsupportedPortable);
        assert_eq!(status.label, "便携版");
        assert_eq!(status.value_text, "打开发布页");
        assert!(status.can_open_release_page);
        assert!(!status.can_install);
    }

    #[test]
    fn idle_status_is_installable_only_after_update_is_available() {
        let status = AppUpdateStatus::idle();

        assert_eq!(status.phase, AppUpdatePhase::Idle);
        assert_eq!(status.label, "检查更新");
        assert_eq!(status.value_text, "检查");
        assert!(!status.can_install);
    }

    #[test]
    fn updater_rejects_official_release_and_confusable_download_urls() {
        let valid = tauri::Url::parse(
            "https://github.com/CNDDVP/EchoIsland/releases/download/v0.7.0-cn/EchoIsland.exe",
        )
        .unwrap();
        assert!(is_cn_release_asset(&valid));
        assert!(update_installation_error(&valid, true).is_none());
        assert!(update_installation_error(&valid, false).is_some());
        for input in [
            "https://github.com/FunplayAI/EchoIsland/releases/download/v0.7.0/app.exe",
            "https://github.com.evil.test/CNDDVP/EchoIsland/releases/download/v1/app.exe",
            "https://github.com@evil.test/CNDDVP/EchoIsland/releases/download/v1/app.exe",
            "http://github.com/CNDDVP/EchoIsland/releases/download/v1/app.exe",
            "https://github.com:444/CNDDVP/EchoIsland/releases/download/v1/app.exe",
            "https://github.com/CNDDVP/EchoIsland/releases/download/v1/",
            "https://github.com/CNDDVP/EchoIsland/releases/download/v1/app.exe?redirect=upstream",
            "https://github.com/CNDDVP/EchoIsland/releases/download/v1/%2E%2E/app.exe",
            "https://github.com/CNDDVP/EchoIsland/releases/download/v1/extra/app.exe",
        ] {
            assert!(
                update_installation_error(&tauri::Url::parse(input).unwrap(), true).is_some(),
                "{input}"
            );
        }
    }

    #[test]
    fn upstream_key_and_disabled_artifacts_never_authorize_cn_installation() {
        let mut config = serde_json::json!({
            "bundle": { "createUpdaterArtifacts": true },
            "plugins": { "updater": { "pubkey": UPSTREAM_PUBLIC_KEY } }
        });
        assert!(!dedicated_signing_is_configured(&config));
        config["plugins"]["updater"]["pubkey"] = "".into();
        assert!(!dedicated_signing_is_configured(&config));
        config["plugins"]["updater"]["pubkey"] = "dedicated-signing-key-fixture".into();
        assert!(!dedicated_signing_is_configured(&config));
        let original = base64::engine::general_purpose::STANDARD
            .decode(UPSTREAM_PUBLIC_KEY)
            .unwrap();
        let packet = std::str::from_utf8(&original)
            .unwrap()
            .lines()
            .nth(1)
            .unwrap();
        let altered_comment =
            format!("untrusted comment: CN label cannot change a key\n{packet}\n");
        config["plugins"]["updater"]["pubkey"] = base64::engine::general_purpose::STANDARD
            .encode(altered_comment)
            .into();
        assert!(!dedicated_signing_is_configured(&config));
        let mut distinct_key = base64::engine::general_purpose::STANDARD
            .decode(packet)
            .unwrap();
        distinct_key[10] ^= 1;
        let fixture = format!(
            "untrusted comment: test fixture, no private key\n{}\n",
            base64::engine::general_purpose::STANDARD.encode(distinct_key)
        );
        config["plugins"]["updater"]["pubkey"] = base64::engine::general_purpose::STANDARD
            .encode(fixture)
            .into();
        assert!(dedicated_signing_is_configured(&config));
        config["bundle"]["createUpdaterArtifacts"] = false.into();
        assert!(!dedicated_signing_is_configured(&config));
        let current: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            current["plugins"]["updater"]["endpoints"],
            serde_json::json!([
                "https://github.com/CNDDVP/EchoIsland/releases/latest/download/latest.json"
            ])
        );
    }

    #[tokio::test]
    async fn stalled_update_check_becomes_retryable_manual_fallback() {
        let error = bounded_update_check(
            std::future::pending::<Result<(), String>>(),
            std::time::Duration::from_millis(10),
        )
        .await
        .unwrap_err();
        let status = AppUpdateStatus::failed(echoisland_i18n::error("update.failure_help", error));
        assert_eq!(status.phase, AppUpdatePhase::Failed);
        assert!(status.can_check);
        assert!(status.can_open_release_page);
        assert!(!status.can_install);
        assert!(super::open_release_page_on_portable_or_failed(&status));
        assert_eq!(
            bounded_update_check(
                async { Ok::<_, String>(42) },
                std::time::Duration::from_secs(1)
            )
            .await
            .unwrap(),
            42
        );
    }
}
