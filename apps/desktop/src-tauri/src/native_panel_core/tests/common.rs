use std::time::{Duration, Instant};

use chrono::Utc;
use echoisland_runtime::{
    PendingPermissionView, PendingQuestionView, RuntimeSnapshot, SessionSnapshotView,
};

use super::super::*;

pub(super) fn snapshot(active: usize, total: usize) -> RuntimeSnapshot {
    RuntimeSnapshot {
        status: "Idle".to_string(),
        primary_source: "claude".to_string(),
        active_session_count: active,
        total_session_count: total,
        pending_permission_count: 0,
        pending_question_count: 0,
        pending_permission: None,
        pending_question: None,
        pending_permissions: Vec::new(),
        pending_questions: Vec::new(),
        sessions: Vec::new(),
    }
}

pub(super) fn session(status: &str) -> SessionSnapshotView {
    SessionSnapshotView {
        session_id: "session-1".to_string(),
        source: "claude".to_string(),
        project_name: None,
        cwd: None,
        model: None,
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
        status: status.to_string(),
        current_tool: None,
        tool_description: None,
        last_user_prompt: None,
        last_assistant_message: None,
        tool_history_count: 0,
        tool_history: Vec::new(),
        last_activity: Utc::now(),
    }
}

pub(super) fn pending_permission(request_id: &str, session_id: &str) -> PendingPermissionView {
    PendingPermissionView {
        request_id: request_id.to_string(),
        session_id: session_id.to_string(),
        source: "claude".to_string(),
        tool_name: Some("Bash".to_string()),
        tool_description: Some("Run command".to_string()),
        requested_at: Utc::now(),
    }
}

pub(super) fn snapshot_with_permission(request_id: &str, session_id: &str) -> RuntimeSnapshot {
    let mut snapshot = snapshot(1, 1);
    let pending = pending_permission(request_id, session_id);
    snapshot.pending_permission_count = 1;
    snapshot.pending_permission = Some(pending.clone());
    snapshot.pending_permissions = vec![pending];
    snapshot.sessions = vec![session("WaitingApproval")];
    snapshot
}

pub(super) fn pending_question(request_id: &str, session_id: &str) -> PendingQuestionView {
    PendingQuestionView {
        request_id: request_id.to_string(),
        session_id: session_id.to_string(),
        source: "claude".to_string(),
        header: Some("Pick one".to_string()),
        text: "Choose the deployment target".to_string(),
        options: vec!["Local".to_string(), "Staging".to_string()],
        requested_at: Utc::now(),
    }
}

pub(super) fn snapshot_with_question(request_id: &str, session_id: &str) -> RuntimeSnapshot {
    let mut snapshot = snapshot(1, 1);
    let pending = pending_question(request_id, session_id);
    snapshot.pending_question_count = 1;
    snapshot.pending_question = Some(pending.clone());
    snapshot.pending_questions = vec![pending];
    snapshot.sessions = vec![session("WaitingQuestion")];
    snapshot
}

pub(super) fn test_card_metrics() -> PanelCardMetricConstants {
    PanelCardMetricConstants {
        card_inset_x: 10.0,
        chat_prefix_width: 15.0,
        chat_line_height: 14.0,
        header_height: 52.0,
        content_bottom_inset: 6.0,
        chat_gap: 4.0,
        tool_gap: 7.0,
        pending_action_y: 9.0,
        pending_action_height: 18.0,
        pending_action_gap: 6.0,
    }
}
