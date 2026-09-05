use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const CODEX_APP_BUNDLE_ID: &str = "com.openai.codex";

#[derive(Clone, Debug)]
#[cfg_attr(target_os = "windows", allow(dead_code))]
pub struct SessionFocusTarget {
    pub session_id: String,
    pub source: String,
    pub project_name: Option<String>,
    pub cwd: Option<String>,
    pub terminal_app: Option<String>,
    pub terminal_bundle: Option<String>,
    pub host_app: Option<String>,
    pub window_title: Option<String>,
    pub tty: Option<String>,
    pub terminal_pid: Option<u32>,
    pub cli_pid: Option<u32>,
    pub iterm_session_id: Option<String>,
    pub kitty_window_id: Option<String>,
    pub tmux_env: Option<String>,
    pub tmux_pane: Option<String>,
    pub tmux_client_tty: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionTabCache {
    pub terminal_pid: u32,
    pub window_hwnd: i64,
    pub runtime_id: String,
    pub title: String,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub struct ForegroundTabInfo {
    pub cache: SessionTabCache,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub struct ObservedTab {
    pub cache: SessionTabCache,
    pub observed_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct SessionObservation {
    pub status: String,
    pub last_user_prompt: Option<String>,
    pub last_activity: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct FocusOutcome {
    pub focused: bool,
    pub selected_tab: Option<SessionTabCache>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexSessionKind {
    App,
    Cli,
    Unknown,
}

pub fn codex_session_kind(target: &SessionFocusTarget) -> CodexSessionKind {
    if target.host_app.as_deref().is_some_and(is_codex_app_host)
        || target
            .terminal_bundle
            .as_deref()
            .is_some_and(is_codex_app_host)
    {
        return CodexSessionKind::App;
    }

    if !target.source.eq_ignore_ascii_case("codex") {
        return CodexSessionKind::Unknown;
    }

    if target.cli_pid.is_some()
        || target.tty.as_deref().is_some_and(|value| !value.is_empty())
        || target.terminal_pid.is_some()
        || target
            .terminal_bundle
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        || target
            .terminal_app
            .as_deref()
            .is_some_and(|value| !value.is_empty())
    {
        return CodexSessionKind::Cli;
    }

    CodexSessionKind::Unknown
}

pub fn is_codex_app_host(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized == CODEX_APP_BUNDLE_ID
}

#[cfg(test)]
mod tests {
    use super::{CODEX_APP_BUNDLE_ID, CodexSessionKind, SessionFocusTarget, codex_session_kind};

    fn target() -> SessionFocusTarget {
        SessionFocusTarget {
            session_id: "session-1".to_string(),
            source: "codex".to_string(),
            project_name: None,
            cwd: None,
            terminal_app: None,
            terminal_bundle: None,
            host_app: None,
            window_title: None,
            tty: None,
            terminal_pid: None,
            cli_pid: None,
            iterm_session_id: None,
            kitty_window_id: None,
            tmux_env: None,
            tmux_pane: None,
            tmux_client_tty: None,
        }
    }

    #[test]
    fn codex_app_requires_explicit_app_bundle() {
        let mut target = target();
        target.host_app = Some(CODEX_APP_BUNDLE_ID.to_string());

        assert_eq!(codex_session_kind(&target), CodexSessionKind::App);
    }

    #[test]
    fn codex_source_host_token_with_cli_pid_is_cli() {
        let mut target = target();
        target.host_app = Some("codex".to_string());
        target.cli_pid = Some(42);

        assert_eq!(codex_session_kind(&target), CodexSessionKind::Cli);
    }

    #[test]
    fn codex_cli_uses_terminal_metadata() {
        let mut target = target();
        target.cli_pid = Some(42);
        target.terminal_bundle = Some("com.apple.Terminal".to_string());

        assert_eq!(codex_session_kind(&target), CodexSessionKind::Cli);
    }

    #[test]
    fn codex_terminal_bundle_marks_codex_app() {
        let mut target = target();
        target.terminal_bundle = Some(CODEX_APP_BUNDLE_ID.to_string());

        assert_eq!(codex_session_kind(&target), CodexSessionKind::App);
    }

    #[test]
    fn codex_terminal_bundle_marks_app_even_with_generic_host_token_and_cli_pid() {
        let mut target = target();
        target.terminal_bundle = Some(CODEX_APP_BUNDLE_ID.to_string());
        target.host_app = Some("codex".to_string());
        target.cli_pid = Some(42);

        assert_eq!(codex_session_kind(&target), CodexSessionKind::App);
    }

    #[test]
    fn codex_without_host_or_terminal_metadata_is_unknown() {
        assert_eq!(codex_session_kind(&target()), CodexSessionKind::Unknown);
    }
}
