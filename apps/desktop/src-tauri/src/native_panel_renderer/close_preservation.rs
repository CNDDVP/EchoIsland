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
        native_panel_core::ExpandedSurface,
        native_panel_renderer::test_fixtures::{test_panel_scene, test_runtime_snapshot},
        native_panel_scene::{PanelRuntimeRenderState, SceneCard, SceneMascotPose},
    };

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
    fn preserved_scene_runtime_state_uses_shell_scene_fields() {
        let mut scene = test_panel_scene(&test_runtime_snapshot("status"));
        scene.compact_bar.headline.emphasized = true;
        scene.compact_bar.actions_visible = false;
        scene.mascot_pose = SceneMascotPose::Complete;

        let runtime = native_panel_runtime_render_state_from_preserved_scene(true, &scene);

        assert_eq!(
            runtime,
            PanelRuntimeRenderState {
                transitioning: true,
                shell_scene: crate::native_panel_scene::PanelShellSceneState {
                    headline_emphasized: true,
                    edge_actions_visible: false,
                },
            }
        );
    }
}
