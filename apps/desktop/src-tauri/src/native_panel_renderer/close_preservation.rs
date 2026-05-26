use echoisland_runtime::RuntimeSnapshot;

use crate::{
    native_panel_core::ExpandedSurface,
    native_panel_scene::{PanelRuntimeRenderState, PanelScene, resolve_panel_shell_scene_state},
};

use super::{
    presentation_model::NativePanelPresentationModel,
    render_commands::NativePanelRenderCommandBundle,
    runtime_scene_cache::NativePanelRuntimeSceneCache,
};

pub(crate) fn native_panel_status_close_preservation_active(
    transitioning: bool,
    expanded: bool,
    skip_next_close_card_exit: bool,
) -> bool {
    (transitioning && !expanded) || skip_next_close_card_exit
}

pub(crate) fn native_panel_status_close_scene_has_cards(scene: &PanelScene) -> bool {
    scene.surface == ExpandedSurface::Status && !scene.cards.is_empty()
}

pub(crate) fn resolve_native_panel_preserved_status_close_scene(
    cache: &NativePanelRuntimeSceneCache,
    active: bool,
) -> Option<PanelScene> {
    if !active {
        return None;
    }
    cache
        .last_render_command_bundle
        .as_ref()
        .map(|bundle| bundle.scene.clone())
        .or_else(|| cache.last_scene.clone())
        .filter(native_panel_status_close_scene_has_cards)
}

pub(crate) fn resolve_native_panel_preserved_status_close_scene_for_snapshot(
    cache: &NativePanelRuntimeSceneCache,
    current_snapshot: Option<&RuntimeSnapshot>,
    snapshot: &RuntimeSnapshot,
    active: bool,
) -> Option<PanelScene> {
    if current_snapshot != Some(snapshot) && cache.last_snapshot.as_ref() != Some(snapshot) {
        return None;
    }
    resolve_native_panel_preserved_status_close_scene(cache, active)
}

pub(crate) fn native_panel_runtime_render_state_from_preserved_scene(
    transitioning: bool,
    scene: &PanelScene,
) -> PanelRuntimeRenderState {
    PanelRuntimeRenderState {
        transitioning,
        shell_scene: resolve_panel_shell_scene_state(scene),
    }
}

pub(crate) fn apply_native_panel_preserved_close_presentation_slots(
    preserved: &NativePanelPresentationModel,
    scene: Option<&mut PanelScene>,
    bundle: Option<&mut NativePanelRenderCommandBundle>,
    presentation: Option<&mut NativePanelPresentationModel>,
) {
    if let Some(scene) = scene {
        scene.surface = preserved.card_stack.surface;
        scene.cards = preserved.card_stack.cards.clone();
        scene.mascot_pose = preserved.mascot.pose;
        scene.debug_mode_enabled = preserved.mascot.debug_mode_enabled;
        scene.glow = preserved.glow.as_ref().map(|glow| glow.glow.clone());
        scene.compact_bar.completion_count = preserved.compact_bar.completion_count;
    }
    if let Some(bundle) = bundle {
        bundle.scene.surface = preserved.card_stack.surface;
        bundle.shell.surface = preserved.card_stack.surface;
        bundle.scene.cards = preserved.card_stack.cards.clone();
        bundle.card_stack.surface = preserved.card_stack.surface;
        bundle.card_stack.cards = preserved.card_stack.cards.clone();
        bundle.card_stack.content_height = preserved.card_stack.content_height;
        bundle.card_stack.body_height = preserved.card_stack.body_height;
        bundle.card_stack.visible = true;
        bundle.scene.mascot_pose = preserved.mascot.pose;
        bundle.scene.debug_mode_enabled = preserved.mascot.debug_mode_enabled;
        bundle.scene.glow = preserved.glow.as_ref().map(|glow| glow.glow.clone());
        bundle.compact_bar.completion_count = preserved.compact_bar.completion_count;
        bundle.mascot = preserved.mascot.command();
        bundle.glow = preserved.glow.as_ref().map(|glow| glow.command());
    }
    if let Some(presentation) = presentation {
        presentation.shell.surface = preserved.card_stack.surface;
        presentation.card_stack.surface = preserved.card_stack.surface;
        presentation.card_stack.cards = preserved.card_stack.cards.clone();
        presentation.card_stack.content_height = preserved.card_stack.content_height;
        presentation.card_stack.body_height = preserved.card_stack.body_height;
        presentation.card_stack.visible = true;
        presentation.mascot = preserved.mascot.clone();
        presentation.glow = preserved.glow.clone();
        presentation.compact_bar.completion_count = preserved.compact_bar.completion_count;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        native_panel_core::{ExpandedSurface, StatusQueueItem, StatusQueuePayload},
        native_panel_renderer::test_fixtures::{
            test_panel_scene, test_pending_permission, test_pending_question,
            test_runtime_snapshot, test_session_snapshot,
        },
        native_panel_scene::{PanelRuntimeRenderState, SceneCard, SceneMascotPose},
    };
    use std::time::Instant;

    use super::{
        native_panel_runtime_render_state_from_preserved_scene,
        resolve_native_panel_preserved_status_close_scene,
    };
    use crate::native_panel_renderer::runtime_scene_cache::NativePanelRuntimeSceneCache;

    #[test]
    fn preserved_status_close_scene_requires_status_cards_and_active_close() {
        let mut cache = NativePanelRuntimeSceneCache::default();
        let mut scene = test_panel_scene(&test_runtime_snapshot("status"));
        scene.surface = ExpandedSurface::Status;
        scene.cards = vec![SceneCard::Empty];
        cache.last_scene = Some(scene.clone());

        assert!(resolve_native_panel_preserved_status_close_scene(&cache, false).is_none());
        assert_eq!(
            resolve_native_panel_preserved_status_close_scene(&cache, true)
                .expect("preserved scene")
                .surface,
            ExpandedSurface::Status
        );
    }

    #[test]
    fn preserved_status_close_scene_treats_all_active_popup_cards_the_same() {
        for card in active_popup_cards() {
            let mut cache = NativePanelRuntimeSceneCache::default();
            let mut scene = test_panel_scene(&test_runtime_snapshot("status"));
            scene.surface = ExpandedSurface::Status;
            scene.cards = vec![card];
            cache.last_scene = Some(scene);

            let preserved =
                resolve_native_panel_preserved_status_close_scene(&cache, true).expect("scene");

            assert_eq!(preserved.surface, ExpandedSurface::Status);
            assert_eq!(preserved.cards.len(), 1);
        }
    }

    #[test]
    fn preserved_scene_runtime_state_uses_shell_scene_fields() {
        let mut scene = test_panel_scene(&test_runtime_snapshot("status"));
        scene.surface = ExpandedSurface::Status;
        scene.compact_bar.headline.emphasized = true;
        scene.compact_bar.actions_visible = false;
        scene.mascot_pose = SceneMascotPose::Complete;

        let runtime = native_panel_runtime_render_state_from_preserved_scene(true, &scene);

        assert_eq!(
            runtime,
            PanelRuntimeRenderState {
                transitioning: true,
                shell_scene: crate::native_panel_scene::PanelShellSceneState {
                    surface: crate::native_panel_core::ExpandedSurface::Status,
                    headline_emphasized: true,
                    edge_actions_visible: false,
                },
            }
        );
    }

    fn active_popup_cards() -> Vec<SceneCard> {
        vec![
            SceneCard::StatusApproval {
                item: status_item(StatusQueuePayload::Approval(test_pending_permission(
                    "codex",
                    "request-1",
                    "session-1",
                ))),
            },
            SceneCard::StatusQuestion {
                item: status_item(StatusQueuePayload::Question(test_pending_question(
                    "codex",
                    "question-1",
                    "session-1",
                ))),
            },
            SceneCard::StatusCompletion {
                item: status_item(StatusQueuePayload::Completion(test_session_snapshot(
                    "codex",
                    "session-1",
                    "idle",
                ))),
            },
        ]
    }

    fn status_item(payload: StatusQueuePayload) -> StatusQueueItem {
        let session_id = match &payload {
            StatusQueuePayload::Approval(pending) => pending.session_id.clone(),
            StatusQueuePayload::Question(pending) => pending.session_id.clone(),
            StatusQueuePayload::Completion(session) => session.session_id.clone(),
        };
        StatusQueueItem {
            key: format!("status:{session_id}"),
            session_id,
            sort_time: chrono::Utc::now(),
            expires_at: Instant::now(),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload,
        }
    }
}
