use super::window_enum::WindowCandidate;
use crate::terminal_focus::{
    CodexSessionKind, SessionFocusTarget, codex_session_kind, focus_tokens, host_app_aliases,
    normalized_token,
};

pub(super) fn select_window_candidate<'a>(
    windows: &'a [WindowCandidate],
    target: &SessionFocusTarget,
) -> (Option<&'a WindowCandidate>, Vec<String>, Vec<String>, usize) {
    let tokens = focus_tokens(target);
    let terminal_window_count = windows
        .iter()
        .filter(|window| window.is_terminal_like)
        .count();
    let target_window_title = target.window_title.as_deref().and_then(normalized_token);
    let terminal_app = target.terminal_app.as_deref().and_then(normalized_token);
    let host_app = target.host_app.as_deref().and_then(normalized_token);
    let host_aliases = host_app_aliases(&target.source, target.host_app.as_deref())
        .into_iter()
        .filter_map(|value| normalized_token(&value))
        .collect::<Vec<_>>();
    let host_alias_window_count = windows
        .iter()
        .filter(|candidate| candidate_matches_host_aliases(candidate, &host_aliases))
        .count();
    let source = normalized_token(&target.source);
    let allow_non_terminal_host_app = !host_aliases.is_empty()
        && target.terminal_pid.is_none()
        && (codex_session_kind(target) == CodexSessionKind::App
            || matches!(
                source.as_deref(),
                Some("vscode" | "cursor" | "trae" | "gemini" | "glm" | "antigravity" | "zcode")
            ));

    let mut candidate_logs = Vec::new();
    let mut best: Option<(i32, &WindowCandidate)> = None;
    for candidate in windows {
        if !candidate.is_terminal_like
            && target.terminal_pid != Some(candidate.pid)
            && !allow_non_terminal_host_app
        {
            continue;
        }

        let mut score = 0i32;
        let title = candidate.title.to_ascii_lowercase();
        let process_name = candidate.process_name.to_ascii_lowercase();
        let mut matched = false;

        if target.terminal_pid == Some(candidate.pid) {
            score += 500;
            matched = true;
        }

        if let Some(window_title) = &target_window_title {
            if title == *window_title {
                score += 240;
                matched = true;
            } else if title.contains(window_title) {
                score += 180;
                matched = true;
            }
        }

        for token in &tokens {
            if title.contains(token) {
                score += 95;
                matched = true;
            }
        }

        if let Some(app) = &terminal_app
            && (process_name.contains(app) || title.contains(app))
        {
            score += 70;
            matched = true;
        }

        if let Some(app) = &host_app
            && (process_name.contains(app) || title.contains(app))
        {
            score += 45;
            matched = true;
        }

        for alias in &host_aliases {
            if process_name == *alias {
                score += 120;
                matched = true;
            } else if process_name.contains(alias) || title.contains(alias) {
                score += 80;
                matched = true;
            }
        }

        if host_alias_window_count == 1 && candidate_matches_host_aliases(candidate, &host_aliases)
        {
            score += 90;
            matched = true;
        }

        if let Some(source) = &source
            && title.contains(source)
        {
            score += 20;
            matched = true;
        }

        if candidate.is_terminal_like {
            score += 10;
        }

        if !matched && candidate.is_terminal_like && terminal_window_count == 1 {
            score += 25;
            matched = true;
        }

        if matched {
            candidate_logs.push(format!(
                "score={score} pid={} proc={} title={}",
                candidate.pid, candidate.process_name, candidate.title
            ));
            if best
                .as_ref()
                .is_none_or(|(best_score, _)| score > *best_score)
            {
                best = Some((score, candidate));
            }
        }
    }

    (
        best.map(|(_, candidate)| candidate),
        candidate_logs,
        host_aliases,
        tokens.len(),
    )
}

fn candidate_matches_host_aliases(candidate: &WindowCandidate, aliases: &[String]) -> bool {
    let process_name = candidate.process_name.to_ascii_lowercase();
    let title = candidate.title.to_ascii_lowercase();
    aliases.iter().any(|alias| {
        process_name == *alias || process_name.contains(alias) || title.contains(alias)
    })
}

#[cfg(test)]
mod tests {
    use super::{WindowCandidate, select_window_candidate};
    use crate::terminal_focus::{CODEX_APP_BUNDLE_ID, SessionFocusTarget};

    fn codex_target() -> SessionFocusTarget {
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

    fn codex_window() -> WindowCandidate {
        WindowCandidate {
            hwnd: std::ptr::null_mut(),
            pid: 42,
            title: "Codex".to_string(),
            process_name: "codex".to_string(),
            is_terminal_like: false,
        }
    }

    #[test]
    fn codex_app_can_match_non_terminal_window() {
        let mut target = codex_target();
        target.host_app = Some(CODEX_APP_BUNDLE_ID.to_string());
        let windows = [codex_window()];

        let (best, _, _, _) = select_window_candidate(&windows, &target);

        assert_eq!(best.map(|window| window.pid), Some(42));
    }

    #[test]
    fn codex_cli_does_not_match_non_terminal_app_window() {
        let mut target = codex_target();
        target.cli_pid = Some(7);
        let windows = [codex_window()];

        let (best, _, _, _) = select_window_candidate(&windows, &target);

        assert!(best.is_none());
    }

    #[test]
    fn antigravity_session_matches_its_desktop_window() {
        let mut target = codex_target();
        target.source = "antigravity".to_string();
        target.host_app = Some("cli".to_string());
        target.project_name = Some("Refining EchoIsland Chinese Translation".to_string());
        let window = WindowCandidate {
            hwnd: std::ptr::null_mut(),
            pid: 23344,
            title: "Refining EchoIsland Chinese Translation".to_string(),
            process_name: "Antigravity".to_string(),
            is_terminal_like: false,
        };
        let windows = [window];

        let (best, _, _, _) = select_window_candidate(&windows, &target);

        assert_eq!(best.map(|window| window.pid), Some(23344));
    }

    #[test]
    fn zcode_session_matches_its_desktop_window() {
        let mut target = codex_target();
        target.source = "zcode".to_string();
        target.host_app = Some("cli".to_string());
        target.project_name = Some("ai-programs".to_string());
        let window = WindowCandidate {
            hwnd: std::ptr::null_mut(),
            pid: 40872,
            title: "ZCode".to_string(),
            process_name: "ZCode".to_string(),
            is_terminal_like: false,
        };
        let windows = [window];

        let (best, _, _, _) = select_window_candidate(&windows, &target);

        assert_eq!(best.map(|window| window.pid), Some(40872));
    }
}
