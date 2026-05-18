use echoisland_runtime::RuntimeSnapshot;

use crate::{
    native_panel_core::{ExpandedSurface, PanelState},
    native_panel_renderer::facade::descriptor::NativePanelRuntimeInputDescriptor,
    native_panel_scene::{
        PanelScene, PanelSceneBuildInput, SceneCard, SceneMascotPose, build_panel_scene,
    },
};

pub(crate) fn test_runtime_snapshot(status: &str) -> RuntimeSnapshot {
    RuntimeSnapshot {
        status: status.to_string(),
        primary_source: "codex".to_string(),
        active_session_count: 0,
        total_session_count: 0,
        pending_permission_count: 0,
        pending_question_count: 0,
        pending_permission: None,
        pending_question: None,
        pending_permissions: vec![],
        pending_questions: vec![],
        sessions: vec![],
    }
}

pub(crate) fn test_native_panel_runtime_input_descriptor() -> NativePanelRuntimeInputDescriptor {
    NativePanelRuntimeInputDescriptor {
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
