use chrono::Utc;
use echoisland_runtime::{
    PendingPermissionView, PendingQuestionView, RuntimeSnapshot, SessionSnapshotView,
};

use crate::{
    native_panel_core::{ExpandedSurface, PanelState},
    native_panel_renderer::facade::descriptor::NativePanelRuntimeInputDescriptor,
    native_panel_scene::{
        PanelScene, PanelSceneBuildInput, SceneCard, SceneMascotPose, build_panel_scene,
    },
};

pub(crate) fn test_runtime_snapshot(status: &str) -> RuntimeSnapshot {
    test_runtime_snapshot_with_counts(status, "codex", 0, 0)
}

pub(crate) fn test_runtime_snapshot_with_counts(
    status: &str,
    primary_source: &str,
    active_session_count: usize,
    total_session_count: usize,
) -> RuntimeSnapshot {
    RuntimeSnapshot {
        status: status.to_string(),
        primary_source: primary_source.to_string(),
        active_session_count,
        total_session_count,
        pending_permission_count: 0,
        pending_question_count: 0,
        pending_permission: None,
        pending_question: None,
        pending_permissions: vec![],
        pending_questions: vec![],
        sessions: vec![],
    }
}

pub(crate) fn test_session_snapshot(
    source: &str,
    session_id: &str,
    status: &str,
) -> SessionSnapshotView {
    SessionSnapshotView {
        session_id: session_id.to_string(),
        source: source.to_string(),
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

pub(crate) fn test_pending_permission(
    source: &str,
    request_id: &str,
    session_id: &str,
) -> PendingPermissionView {
    PendingPermissionView {
        request_id: request_id.to_string(),
        session_id: session_id.to_string(),
        source: source.to_string(),
        tool_name: Some("Bash".to_string()),
        tool_description: Some("Run command".to_string()),
        requested_at: Utc::now(),
    }
}

pub(crate) fn test_pending_question(
    source: &str,
    request_id: &str,
    session_id: &str,
) -> PendingQuestionView {
    PendingQuestionView {
        request_id: request_id.to_string(),
        session_id: session_id.to_string(),
        source: source.to_string(),
        header: Some("Pick one".to_string()),
        text: "Choose the deployment target".to_string(),
        options: vec!["Local".to_string(), "Staging".to_string()],
        requested_at: Utc::now(),
    }
}

pub(crate) fn test_native_panel_runtime_input_descriptor() -> NativePanelRuntimeInputDescriptor {
    NativePanelRuntimeInputDescriptor {
        screen_scale_factor: None,
        screen_physical_frame: None,
        scene_input: PanelSceneBuildInput::default(),
        screen_frame: None,
    }
}

pub(crate) fn test_panel_scene(snapshot: &RuntimeSnapshot) -> PanelScene {
    build_panel_scene(
        &PanelState::default(),
        snapshot,
        &PanelSceneBuildInput::default(),
    )
}

pub(crate) fn test_preserved_status_close_scene(snapshot: &RuntimeSnapshot) -> PanelScene {
    let mut scene = test_panel_scene(snapshot);
    scene.surface = ExpandedSurface::Status;
    scene.cards = vec![SceneCard::Empty];
    scene.mascot_pose = SceneMascotPose::Complete;
    scene
}
