use std::{process::Command, thread, time::Duration};

use anyhow::{Context, Result};
use tracing::warn;

use crate::terminal_focus::{
    CodexSessionKind, FocusOutcome, SessionFocusTarget, codex_session_kind,
};

const DEEPLINK_DELAY: Duration = Duration::from_millis(260);

pub(super) fn schedule_thread_resume(target: &SessionFocusTarget) -> Option<FocusOutcome> {
    if !should_resume_in_codex_app(target) {
        return None;
    }

    let deeplink = codex_thread_deeplink(&target.session_id).ok()?;
    thread::spawn(move || {
        thread::sleep(DEEPLINK_DELAY);
        if let Err(error) = open_thread_deeplink(&deeplink) {
            warn!(
                deeplink = %deeplink,
                error = %error,
                "failed to open Codex app thread deeplink"
            );
        }
    });

    Some(FocusOutcome {
        focused: true,
        selected_tab: None,
    })
}

fn should_resume_in_codex_app(target: &SessionFocusTarget) -> bool {
    codex_session_kind(target) == CodexSessionKind::App
}

fn codex_thread_deeplink(thread_id: &str) -> Result<String> {
    let thread_id = thread_id.trim();
    if thread_id.is_empty() {
        anyhow::bail!("missing Codex thread id");
    }
    Ok(format!("codex://threads/{thread_id}"))
}

fn open_thread_deeplink(deeplink: &str) -> Result<()> {
    Command::new("explorer.exe")
        .arg(deeplink)
        .spawn()
        .with_context(|| format!("open {deeplink}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{codex_thread_deeplink, should_resume_in_codex_app};
    use crate::terminal_focus::{
        CODEX_APP_BUNDLE_ID, CodexSessionKind, SessionFocusTarget, codex_session_kind,
    };

    fn target() -> SessionFocusTarget {
        SessionFocusTarget {
            session_id: "thr_123".to_string(),
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
    fn codex_app_target_requires_explicit_app_bundle() {
        let mut cli_target = target();
        assert_ne!(codex_session_kind(&cli_target), CodexSessionKind::App);

        cli_target.host_app = Some(CODEX_APP_BUNDLE_ID.to_string());
        assert_eq!(codex_session_kind(&cli_target), CodexSessionKind::App);
    }

    #[test]
    fn unknown_codex_target_without_terminal_binding_does_not_use_app_deeplink_fallback() {
        let cli_target = target();

        assert_eq!(codex_session_kind(&cli_target), CodexSessionKind::Unknown);
        assert!(!should_resume_in_codex_app(&cli_target));
    }

    #[test]
    fn codex_target_with_terminal_binding_does_not_use_app_fallback() {
        let mut cli_target = target();
        cli_target.cli_pid = Some(1234);

        assert_eq!(codex_session_kind(&cli_target), CodexSessionKind::Cli);
        assert!(!should_resume_in_codex_app(&cli_target));
    }

    #[test]
    fn codex_thread_deeplink_matches_codex_app_copy_link_format() {
        assert_eq!(
            codex_thread_deeplink("019e165d-f8e3-77c0-aa57-09b8479898e3").unwrap(),
            "codex://threads/019e165d-f8e3-77c0-aa57-09b8479898e3"
        );
    }

    #[test]
    fn codex_thread_deeplink_rejects_empty_ids() {
        assert!(codex_thread_deeplink("   ").is_err());
    }
}
