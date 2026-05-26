use std::time::Instant;

use echoisland_runtime::RuntimeSnapshot;

use super::{
    build_native_panel_runtime_render_state_for_state_with_input,
    build_native_panel_scene_for_core_state_with_input,
    build_native_panel_scene_for_state_with_input,
    current_snapshot_render_plan_for_state_and_snapshot, native_panel_runtime_input_descriptor,
    resolve_and_cache_native_panel_presentation,
    resolve_native_panel_presentation_model_for_state_and_snapshot,
    resolve_native_panel_render_command_bundle_for_state_and_snapshot,
    resolve_native_panel_runtime_render_state_for_state_with_input,
    resolve_native_panel_scene_for_state_and_snapshot,
    resolve_native_panel_scene_for_state_with_input, resolve_or_build_native_panel_scene,
    resolve_snapshot_card_stack_command, resolve_snapshot_render_plan,
};
use crate::{
    macos_native_panel::{mascot::NativeMascotRuntime, panel_types::NativeExpandedSurface},
    native_panel_core::ExpandedSurface,
    native_panel_renderer::facade::{
        descriptor::{NativePanelHostWindowDescriptor, NativePanelRuntimeInputDescriptor},
        renderer::{
            NativePanelRenderCommandBundle, NativePanelRuntimeSceneCache,
            cache_render_command_bundle, cache_render_command_bundle_with_key,
            cache_scene_runtime_with_key, native_panel_runtime_scene_cache_key,
            native_panel_runtime_scene_cache_key_for_state_bridge,
            resolve_native_panel_render_command_bundle,
        },
        testing::{
            test_native_panel_runtime_input_descriptor as runtime_input_descriptor,
            test_preserved_status_close_scene as preserved_status_close_scene,
            test_runtime_snapshot as test_snapshot,
        },
    },
    native_panel_scene::{
        PanelRuntimeRenderState, PanelScene, PanelSceneBuildInput, SceneMascotPose,
        build_panel_scene,
    },
};

fn panel_state() -> crate::macos_native_panel::panel_types::NativePanelState {
    crate::macos_native_panel::panel_types::NativePanelState {
        expanded: false,
        transitioning: false,
        transition_cards_progress: 0.0,
        transition_cards_entering: false,
        skip_next_close_card_exit: false,
        pending_transition: None,
        last_raw_snapshot: None,
        last_snapshot: Some(test_snapshot("idle")),
        scene_cache: NativePanelRuntimeSceneCache::default(),
        status_queue: Vec::new(),
        completion_badge_items: Vec::new(),
        pending_permission_card: None,
        pending_question_card: None,
        status_auto_expanded: false,
        surface_mode: NativeExpandedSurface::Default,
        shared_body_height: None,
        host_window_descriptor: NativePanelHostWindowDescriptor::default(),
        pointer_inside_since: None,
        pointer_outside_since: None,
        primary_mouse_down: false,
        ignores_mouse_events: true,
        last_focus_click: None,
        pointer_regions: Vec::new(),
        mascot_runtime: NativeMascotRuntime::new(Instant::now()),
        transition_collapsed_chrome_alpha: None,
    }
}

fn cache_scene_for_state(
    state: &mut crate::macos_native_panel::panel_types::NativePanelState,
    input: &NativePanelRuntimeInputDescriptor,
    scene: crate::native_panel_scene::PanelScene,
    runtime_render_state: PanelRuntimeRenderState,
) {
    let cache_key = native_panel_runtime_scene_cache_key(&state.to_core_panel_state(), input);
    let bridge_key = native_panel_runtime_scene_cache_key_for_state_bridge(state, input);
    assert_eq!(cache_key, bridge_key);
    cache_scene_runtime_with_key(
        &mut state.scene_cache,
        Some(cache_key),
        scene,
        runtime_render_state,
    );
}

fn cache_bundle_for_state(
    state: &mut crate::macos_native_panel::panel_types::NativePanelState,
    input: &NativePanelRuntimeInputDescriptor,
    bundle: &NativePanelRenderCommandBundle,
) {
    let cache_key = native_panel_runtime_scene_cache_key(&state.to_core_panel_state(), input);
    let bridge_key = native_panel_runtime_scene_cache_key_for_state_bridge(state, input);
    assert_eq!(cache_key, bridge_key);
    cache_render_command_bundle_with_key(&mut state.scene_cache, Some(cache_key), bundle);
}

fn seed_preserved_status_close_scene(
    state: &mut crate::macos_native_panel::panel_types::NativePanelState,
    snapshot: RuntimeSnapshot,
    scene: PanelScene,
) {
    state.last_snapshot = Some(snapshot.clone());
    state.scene_cache.last_snapshot = Some(snapshot);
    state.scene_cache.last_scene = Some(scene);
    state.transitioning = true;
    state.expanded = false;
    state.surface_mode = NativeExpandedSurface::Default;
}

#[test]
fn current_scene_resolution_prefers_shared_cache() {
    let mut state = panel_state();
    let input = runtime_input_descriptor();
    let cached_scene = build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &test_snapshot("cached"),
        &PanelSceneBuildInput::default(),
    );
    cache_scene_for_state(
        &mut state,
        &input,
        cached_scene.clone(),
        PanelRuntimeRenderState::default(),
    );

    let resolved =
        resolve_native_panel_scene_for_state_with_input(&state, &input).expect("cached scene");

    assert_eq!(
        resolved.compact_bar.headline.text,
        cached_scene.compact_bar.headline.text
    );
}

#[test]
fn current_runtime_state_resolution_prefers_shared_cache() {
    let mut state = panel_state();
    let input = runtime_input_descriptor();
    cache_scene_for_state(
        &mut state,
        &input,
        build_panel_scene(
            &crate::native_panel_core::PanelState::default(),
            &test_snapshot("cached"),
            &PanelSceneBuildInput::default(),
        ),
        PanelRuntimeRenderState {
            transitioning: true,
            ..PanelRuntimeRenderState::default()
        },
    );

    let resolved = resolve_native_panel_runtime_render_state_for_state_with_input(&state, &input);

    assert!(resolved.transitioning);
}

#[test]
fn current_scene_resolution_prefers_render_command_bundle_cache() {
    let mut state = panel_state();
    let input = runtime_input_descriptor();
    let bundle = test_render_command_bundle("bundle", false);
    cache_bundle_for_state(&mut state, &input, &bundle);

    let resolved =
        resolve_native_panel_scene_for_state_with_input(&state, &input).expect("bundle scene");

    assert_eq!(
        resolved.compact_bar.headline.text,
        bundle.scene.compact_bar.headline.text
    );
}

#[test]
fn current_runtime_state_resolution_prefers_render_command_bundle_cache() {
    let mut state = panel_state();
    let input = runtime_input_descriptor();
    let bundle = test_render_command_bundle("bundle", true);
    cache_bundle_for_state(&mut state, &input, &bundle);

    let resolved = resolve_native_panel_runtime_render_state_for_state_with_input(&state, &input);

    assert!(resolved.transitioning);
}

#[test]
fn visual_scene_build_ignores_stale_shared_cache() {
    let mut state = panel_state();
    state.expanded = true;
    state.surface_mode = NativeExpandedSurface::Settings;
    state.scene_cache.last_scene = Some(build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &test_snapshot("cached"),
        &PanelSceneBuildInput::default(),
    ));

    let input = runtime_input_descriptor();
    let resolved = build_native_panel_scene_for_state_with_input(
        &state,
        state.last_snapshot.as_ref().expect("snapshot"),
        &input,
    );

    assert_eq!(
        resolved.surface,
        crate::native_panel_core::ExpandedSurface::Settings
    );
    assert!(matches!(
        resolved.cards.first(),
        Some(crate::native_panel_scene::SceneCard::Settings { .. })
    ));
}

#[test]
fn visual_runtime_state_build_ignores_stale_shared_cache() {
    let mut state = panel_state();
    state.expanded = true;
    state.scene_cache.last_runtime_render_state = Some(PanelRuntimeRenderState {
        transitioning: true,
        shell_scene: crate::native_panel_scene::PanelShellSceneState {
            edge_actions_visible: false,
            ..crate::native_panel_scene::PanelShellSceneState::default()
        },
    });

    let input = runtime_input_descriptor();
    let resolved = build_native_panel_runtime_render_state_for_state_with_input(&state, &input);

    assert!(!resolved.transitioning);
    assert!(resolved.shell_scene.edge_actions_visible);
}

#[test]
fn explicit_scene_build_reuses_shared_cache_for_current_snapshot() {
    let mut state = panel_state();
    let input = runtime_input_descriptor();
    let cached_scene = build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &test_snapshot("cached"),
        &PanelSceneBuildInput::default(),
    );
    let current_snapshot = test_snapshot("idle");
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_scene_for_state(
        &mut state,
        &input,
        cached_scene.clone(),
        PanelRuntimeRenderState::default(),
    );

    let resolved = if state.last_snapshot.as_ref() == Some(&current_snapshot) {
        resolve_native_panel_scene_for_state_with_input(&state, &input).expect("cached scene")
    } else {
        unreachable!("current snapshot should match state snapshot");
    };

    assert_eq!(
        resolved.compact_bar.headline.text,
        cached_scene.compact_bar.headline.text
    );
}

#[test]
fn current_snapshot_scene_resolution_reuses_cached_scene() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("idle");
    let input = runtime_input_descriptor();
    let cached_scene = build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &test_snapshot("cached"),
        &PanelSceneBuildInput::default(),
    );
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_scene_for_state(
        &mut state,
        &input,
        cached_scene.clone(),
        PanelRuntimeRenderState::default(),
    );

    let resolved =
        resolve_native_panel_scene_for_state_and_snapshot(&state, &current_snapshot, &input)
            .expect("current snapshot scene");

    assert_eq!(
        resolved.compact_bar.headline.text,
        cached_scene.compact_bar.headline.text
    );
}

#[test]
fn resolve_or_build_scene_uses_cached_current_snapshot() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("idle");
    let input = runtime_input_descriptor();
    let cached_scene = build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &test_snapshot("cached"),
        &PanelSceneBuildInput::default(),
    );
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_scene_for_state(
        &mut state,
        &input,
        cached_scene.clone(),
        PanelRuntimeRenderState::default(),
    );

    let resolved = resolve_or_build_native_panel_scene(&current_snapshot);

    assert_eq!(
        resolved.compact_bar.headline.text,
        cached_scene.compact_bar.headline.text
    );
}

#[test]
fn mismatched_snapshot_scene_resolution_rebuilds_from_snapshot() {
    let mut state = panel_state();
    state.expanded = true;
    state.surface_mode = NativeExpandedSurface::Settings;
    state.last_snapshot = Some(test_snapshot("current"));
    state.scene_cache.last_snapshot = Some(test_snapshot("current"));
    let cached_scene = build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &test_snapshot("cached"),
        &PanelSceneBuildInput::default(),
    );
    state.scene_cache.last_scene = Some(cached_scene.clone());
    let other_snapshot = test_snapshot("other");
    let expected = build_native_panel_scene_for_core_state_with_input(
        &other_snapshot,
        &state.to_core_panel_state(),
        &runtime_input_descriptor(),
    );

    let resolved = resolve_native_panel_scene_for_state_and_snapshot(
        &state,
        &other_snapshot,
        &runtime_input_descriptor(),
    )
    .expect("rebuilt scene");

    assert_eq!(
        resolved.compact_bar.headline.text,
        expected.compact_bar.headline.text
    );
    assert_eq!(
        resolved.surface,
        crate::native_panel_core::ExpandedSurface::Settings
    );
    assert_ne!(resolved.surface, cached_scene.surface);
}

#[test]
fn current_snapshot_scene_resolution_ignores_stale_surface_cache() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("current");
    state.expanded = true;
    state.surface_mode = NativeExpandedSurface::Settings;
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_scene = Some(build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &current_snapshot,
        &PanelSceneBuildInput::default(),
    ));

    let resolved = resolve_native_panel_scene_for_state_and_snapshot(
        &state,
        &current_snapshot,
        &runtime_input_descriptor(),
    )
    .expect("rebuilt scene");

    assert_eq!(
        resolved.surface,
        crate::native_panel_core::ExpandedSurface::Settings
    );
    assert!(matches!(
        resolved.cards.first(),
        Some(crate::native_panel_scene::SceneCard::Settings { .. })
    ));
}

#[test]
fn close_transition_scene_resolution_reuses_preserved_status_scene_after_key_change() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("current");
    seed_preserved_status_close_scene(
        &mut state,
        current_snapshot.clone(),
        preserved_status_close_scene(&current_snapshot),
    );

    let resolved = resolve_native_panel_scene_for_state_and_snapshot(
        &state,
        &current_snapshot,
        &runtime_input_descriptor(),
    )
    .expect("preserved status scene");

    assert_eq!(resolved.surface, ExpandedSurface::Status);
    assert_eq!(resolved.cards.len(), 1);
    assert_eq!(resolved.mascot_pose, SceneMascotPose::Complete);
}

#[test]
fn close_transition_render_plan_keeps_preserved_status_mascot_after_key_change() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("current");
    seed_preserved_status_close_scene(
        &mut state,
        current_snapshot.clone(),
        preserved_status_close_scene(&current_snapshot),
    );

    let resolved = current_snapshot_render_plan_for_state_and_snapshot(&state, &current_snapshot);
    let presentation =
        resolve_native_panel_presentation_model_for_state_and_snapshot(&state, &current_snapshot)
            .expect("preserved status presentation");

    assert_eq!(resolved.scene.surface, ExpandedSurface::Status);
    assert_eq!(resolved.surface(), ExpandedSurface::Status);
    assert_eq!(presentation.mascot.pose, SceneMascotPose::Complete);
}

#[test]
fn close_transition_runtime_state_uses_preserved_status_shell_scene() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("current");
    let mut preserved_scene = preserved_status_close_scene(&current_snapshot);
    preserved_scene.compact_bar.headline.emphasized = true;
    preserved_scene.compact_bar.actions_visible = false;
    seed_preserved_status_close_scene(&mut state, current_snapshot, preserved_scene);

    let runtime = build_native_panel_runtime_render_state_for_state_with_input(
        &state,
        &runtime_input_descriptor(),
    );

    assert!(runtime.transitioning);
    assert!(runtime.shell_scene.headline_emphasized);
    assert!(!runtime.shell_scene.edge_actions_visible);
}

#[test]
fn close_transition_presentation_cache_keeps_preserved_status_scene_on_animation_frame() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("current");
    seed_preserved_status_close_scene(
        &mut state,
        current_snapshot.clone(),
        preserved_status_close_scene(&current_snapshot),
    );

    let frame_bundle = test_render_command_bundle("frame", true);
    let resolved = resolve_and_cache_native_panel_presentation(
        &mut state,
        frame_bundle.layout,
        frame_bundle.render_state,
        None,
    )
    .expect("preserved close presentation");

    assert_eq!(resolved.bundle.scene.surface, ExpandedSurface::Status);
    assert_eq!(resolved.presentation.mascot.pose, SceneMascotPose::Complete);
    assert_eq!(
        state
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Status)
    );
}

#[test]
fn render_command_bundle_resolution_reuses_current_snapshot_cache() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("idle");
    let input = native_panel_runtime_input_descriptor();
    let bundle = test_render_command_bundle("bundle", true);
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_bundle_for_state(&mut state, &input, &bundle);

    let resolved = resolve_native_panel_render_command_bundle_for_state_and_snapshot(
        &state,
        &current_snapshot,
    )
    .expect("cached bundle");

    assert_eq!(
        resolved.compact_bar.headline.text,
        bundle.compact_bar.headline.text
    );
    assert!(resolved.runtime.transitioning);
}

#[test]
fn render_command_bundle_resolution_skips_mismatched_snapshot_cache() {
    let mut state = panel_state();
    state.last_snapshot = Some(test_snapshot("current"));
    state.scene_cache.last_snapshot = Some(test_snapshot("current"));
    cache_render_command_bundle(
        &mut state.scene_cache,
        &test_render_command_bundle("bundle", true),
    );

    let resolved = resolve_native_panel_render_command_bundle_for_state_and_snapshot(
        &state,
        &test_snapshot("other"),
    );

    assert!(resolved.is_none());
}

#[test]
fn presentation_model_resolution_reuses_current_snapshot_cache() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("idle");
    let input = native_panel_runtime_input_descriptor();
    let bundle = test_render_command_bundle_with_input("bundle", true, &input);
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_bundle_for_state(&mut state, &input, &bundle);

    let resolved =
        resolve_native_panel_presentation_model_for_state_and_snapshot(&state, &current_snapshot)
            .expect("cached presentation");

    assert_eq!(
        resolved.compact_bar.headline.text,
        bundle.compact_bar.headline.text
    );
    assert_eq!(
        resolved.compact_bar.completion_count,
        bundle.compact_bar.completion_count
    );
    assert_eq!(resolved.mascot.pose, bundle.mascot.pose);
}

#[test]
fn snapshot_render_plan_ignores_stale_surface_bundle_cache() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("current");
    state.expanded = true;
    state.surface_mode = NativeExpandedSurface::Settings;
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_render_command_bundle(
        &mut state.scene_cache,
        &test_render_command_bundle("bundle", false),
    );

    let resolved = current_snapshot_render_plan_for_state_and_snapshot(&state, &current_snapshot);

    assert_eq!(
        resolved.scene.surface,
        crate::native_panel_core::ExpandedSurface::Settings
    );
    assert!(matches!(
        resolved.scene.cards.first(),
        Some(crate::native_panel_scene::SceneCard::Settings { .. })
    ));
}

#[test]
fn snapshot_compact_bar_command_reuses_cached_bundle_and_overrides_frame() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("idle");
    let input = runtime_input_descriptor();
    let bundle = test_render_command_bundle("bundle", false);
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_bundle_for_state(&mut state, &input, &bundle);

    let frame = crate::native_panel_core::PanelRect {
        x: 11.0,
        y: 12.0,
        width: 13.0,
        height: 14.0,
    };

    let resolved = resolve_snapshot_render_plan(&current_snapshot).compact_bar_command(frame);

    assert_eq!(resolved.headline.text, bundle.compact_bar.headline.text);
    assert_eq!(resolved.frame, frame);
}

#[test]
fn snapshot_card_stack_command_reuses_cached_bundle_and_overrides_layout_inputs() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("idle");
    let input = runtime_input_descriptor();
    let bundle = test_render_command_bundle("bundle", false);
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_bundle_for_state(&mut state, &input, &bundle);

    let frame = crate::native_panel_core::PanelRect {
        x: 21.0,
        y: 22.0,
        width: 23.0,
        height: 24.0,
    };

    let resolved = resolve_snapshot_card_stack_command(&current_snapshot, frame, false);

    assert_eq!(resolved.cards.len(), bundle.card_stack.cards.len());
    assert_eq!(resolved.frame, frame);
    assert!(!resolved.visible);
}

#[test]
fn snapshot_render_plan_reuses_cached_bundle_scene_and_body_height() {
    let mut state = panel_state();
    let current_snapshot = test_snapshot("idle");
    let input = runtime_input_descriptor();
    let bundle = test_render_command_bundle("bundle", false);
    state.last_snapshot = Some(current_snapshot.clone());
    state.scene_cache.last_snapshot = Some(current_snapshot.clone());
    cache_bundle_for_state(&mut state, &input, &bundle);

    let resolved = resolve_snapshot_render_plan(&current_snapshot);

    assert_eq!(
        resolved.scene.compact_bar.headline.text,
        bundle.scene.compact_bar.headline.text
    );
    assert!(resolved.expanded_body_height() <= resolved.expanded_content_height());
}

fn test_render_command_bundle(status: &str, transitioning: bool) -> NativePanelRenderCommandBundle {
    test_render_command_bundle_with_input(status, transitioning, &runtime_input_descriptor())
}

fn test_render_command_bundle_with_input(
    status: &str,
    transitioning: bool,
    input: &NativePanelRuntimeInputDescriptor,
) -> NativePanelRenderCommandBundle {
    let scene = build_panel_scene(
        &crate::native_panel_core::PanelState::default(),
        &test_snapshot(status),
        &input.scene_input,
    );
    let layout = crate::native_panel_core::resolve_panel_layout(
        crate::native_panel_core::PanelLayoutInput {
            screen_frame: crate::native_panel_core::PanelRect {
                x: 0.0,
                y: 0.0,
                width: 1440.0,
                height: 900.0,
            },
            metrics: crate::native_panel_core::PanelGeometryMetrics {
                compact_height: crate::native_panel_core::DEFAULT_COMPACT_PILL_HEIGHT,
                compact_width: crate::native_panel_core::DEFAULT_COMPACT_PILL_WIDTH,
                expanded_width: crate::native_panel_core::DEFAULT_EXPANDED_PILL_WIDTH,
                panel_width: crate::native_panel_core::DEFAULT_PANEL_CANVAS_WIDTH,
            },
            canvas_height: 180.0,
            visible_height: 180.0,
            bar_progress: 1.0,
            height_progress: 1.0,
            drop_progress: 1.0,
            content_visibility: 1.0,
            collapsed_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
            drop_distance: crate::native_panel_core::PANEL_DROP_DISTANCE,
            content_top_gap: crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP,
            content_bottom_inset: crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET,
            cards_side_inset: crate::native_panel_core::EXPANDED_CARDS_SIDE_INSET,
            shoulder_size: crate::native_panel_core::COMPACT_SHOULDER_SIZE,
            separator_side_inset: crate::native_panel_core::EXPANDED_SEPARATOR_SIDE_INSET,
        },
    );
    let runtime = PanelRuntimeRenderState {
        transitioning,
        ..PanelRuntimeRenderState::default()
    };
    let render_state = crate::native_panel_core::resolve_panel_render_state(
        crate::native_panel_core::PanelRenderStateInput {
            shared_expanded_enabled: false,
            shell_visible: layout.shell_visible,
            separator_visibility: layout.separator_visibility,
            bar_progress: 1.0,
            height_progress: 1.0,
            chrome_transition_progress: 1.0,
            shoulder_progress: 0.0,
            cards_height: layout.cards_frame.height,
            status_surface_active: false,
            content_visibility: 1.0,
            transitioning,
            headline_emphasized: scene.compact_bar.headline.emphasized,
            edge_actions_visible: scene.compact_bar.actions_visible,
        },
    );

    resolve_native_panel_render_command_bundle(layout, &scene, runtime, render_state, None)
}
