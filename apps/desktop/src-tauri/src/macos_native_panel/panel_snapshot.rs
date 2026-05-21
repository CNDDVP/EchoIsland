use std::thread;

use crate::notification_sound::play_message_card_sound;
use chrono::Utc;
use echoisland_runtime::RuntimeSnapshot;
use tauri::AppHandle;

use super::card_animation::apply_card_stack_transition;
use super::panel_entry::native_ui_enabled;
use super::panel_refs::native_panel_state;
use super::panel_render::apply_panel_geometry;
use super::panel_runtime_dispatch::{
    dispatch_native_panel_render_payload,
    dispatch_native_panel_transition_request_or_apply_render_payload,
};
use super::panel_runtime_input::native_panel_runtime_input_descriptor;
use super::panel_scene_adapter::resolve_snapshot_render_plan;
use super::panel_types::{
    NativePanelHandles, NativePanelRenderPayload, NativePanelState, NativePanelTransitionFrame,
};
use super::panel_view_updates::apply_snapshot_values_to_panel;

#[cfg(test)]
use super::panel_types::NativeStatusQueueSyncResult;
use super::transition_ui::{
    render_transition_cards_with_plan, reset_collapsed_cards, resolve_native_transition_context,
    resolved_snapshot_panel_height_for_plan,
};
use crate::native_panel_renderer::facade::{
    host::sync_runtime_host_shared_body_height_in_state,
    presentation::NativePanelSnapshotRenderPlan,
    renderer::{
        cache_runtime_scene_sync_result, native_panel_status_close_scene_has_cards,
        sync_runtime_scene_bundle_from_state_input,
    },
    runtime::{
        NativePanelRuntimeRenderPayloadState, NativePanelRuntimeRenderPayloadStateBridge,
        NativePanelRuntimeSceneSyncResult,
    },
    transition::{NativePanelTransitionRequest, native_panel_transition_request_for_snapshot_sync},
};
use crate::native_panel_scene::PanelScene;

struct NativeSnapshotUpdate {
    snapshot: RuntimeSnapshot,
    play_message_sound: bool,
    transition_request: Option<NativePanelTransitionRequest>,
    apply_state: NativePanelRuntimeRenderPayloadState,
}

struct NativeSharedBodyHeightUpdate {
    rerender_payload: Option<NativePanelRenderPayload>,
}

#[derive(Clone, Copy)]
struct NativeCardTransitionState {
    progress: f64,
    entering: bool,
}

enum NativeSnapshotApplyMode {
    TransitioningExpanded(NativeCardTransitionState),
    TransitioningCollapsed,
    Static { expanded: bool, total_height: f64 },
}

pub(crate) fn update_native_island_snapshot<R: tauri::Runtime>(
    app: &AppHandle<R>,
    snapshot: &RuntimeSnapshot,
) -> Result<(), String> {
    if !native_ui_enabled() {
        return Ok(());
    }

    let Some(update) = sync_native_snapshot_update(snapshot)? else {
        return Ok(());
    };
    let play_message_sound = update.play_message_sound;
    dispatch_snapshot_update(app, update)?;
    play_message_sound_if_needed(play_message_sound);
    Ok(())
}

pub(crate) fn set_shared_expanded_body_height<R: tauri::Runtime>(
    app: &AppHandle<R>,
    body_height: f64,
) -> Result<(), String> {
    if !native_ui_enabled() {
        return Ok(());
    }

    let Some(update) = sync_shared_expanded_body_height_update(body_height)? else {
        return Ok(());
    };
    dispatch_shared_expanded_body_height_update(app, update)
}

pub(super) fn native_panel_render_payload(
    state: &NativePanelState,
) -> Option<NativePanelRenderPayload> {
    state.render_payload()
}

#[allow(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn apply_native_panel_render_payload(
    handles: NativePanelHandles,
    payload: NativePanelRenderPayload,
) {
    apply_snapshot_to_panel(handles, &payload);
}

fn sync_shared_expanded_body_height_update(
    body_height: f64,
) -> Result<Option<NativeSharedBodyHeightUpdate>, String> {
    let Some(state_mutex) = native_panel_state() else {
        return Ok(None);
    };
    let mut state = state_mutex
        .lock()
        .map_err(|_| "native panel state poisoned".to_string())?;
    let decision = crate::native_panel_core::resolve_shared_body_height_decision(
        crate::native_panel_core::SharedBodyHeightDecisionInput {
            current_height: state.shared_body_height,
            requested_height: body_height,
            has_snapshot: state.last_snapshot.is_some(),
            update_threshold: 1.0,
        },
    );
    if !decision.should_update {
        return Ok(None);
    }

    let next_height = Some(decision.next_height);
    state.shared_body_height = next_height;
    sync_runtime_host_shared_body_height_in_state(&mut *state, next_height);
    Ok(Some(NativeSharedBodyHeightUpdate {
        rerender_payload: decision
            .should_rerender
            .then(|| native_panel_render_payload(&state))
            .flatten(),
    }))
}

fn dispatch_shared_expanded_body_height_update<R: tauri::Runtime>(
    app: &AppHandle<R>,
    update: NativeSharedBodyHeightUpdate,
) -> Result<(), String> {
    if let Some(payload) = update.rerender_payload {
        dispatch_native_panel_render_payload(app, payload)?;
    }

    Ok(())
}

#[cfg(test)]
pub(super) type NativeStatusSurfaceTransition = crate::native_panel_core::StatusSurfaceTransition;

#[cfg(test)]
pub(super) fn sync_native_status_surface_policy(
    state: &mut NativePanelState,
    status_queue_sync: NativeStatusQueueSyncResult,
) -> NativeStatusSurfaceTransition {
    let mut core = state.to_core_panel_state();
    let transition =
        crate::native_panel_core::sync_status_surface_policy(&mut core, status_queue_sync);
    state.apply_core_panel_state(core);
    transition
}

fn sync_native_snapshot_update(
    snapshot: &RuntimeSnapshot,
) -> Result<Option<NativeSnapshotUpdate>, String> {
    let Some(state) = native_panel_state() else {
        return Ok(None);
    };
    let mut state = state
        .lock()
        .map_err(|_| "native panel state poisoned".to_string())?;
    let input = native_panel_runtime_input_descriptor();
    let close_active_before_sync = state.transitioning && !state.expanded;
    let preserved_close_scene = capture_status_close_scene(&state);
    let mut sync_result =
        sync_runtime_scene_bundle_from_state_input(&mut *state, snapshot, &input, Utc::now());
    let snapshot = sync_result.snapshot_sync.displayed_snapshot.clone();
    let transition_request =
        native_panel_transition_request_for_snapshot_sync(&sync_result.snapshot_sync);
    preserve_status_close_scene_if_needed(
        &mut sync_result,
        preserved_close_scene,
        close_active_before_sync
            || state.skip_next_close_card_exit
            || transition_request == Some(NativePanelTransitionRequest::Close),
    );
    let update = NativeSnapshotUpdate {
        transition_request,
        play_message_sound: sync_result.snapshot_sync.reminder.play_sound,
        apply_state: state.runtime_render_payload_state(),
        snapshot: snapshot.clone(),
    };
    state.last_snapshot = Some(snapshot);
    cache_runtime_scene_sync_result(&mut state.scene_cache, sync_result);
    Ok(Some(update))
}

fn capture_status_close_scene(state: &NativePanelState) -> Option<PanelScene> {
    state
        .scene_cache
        .last_scene
        .clone()
        .filter(native_panel_status_close_scene_has_cards)
}

fn preserve_status_close_scene_if_needed(
    sync_result: &mut NativePanelRuntimeSceneSyncResult,
    preserved_scene: Option<PanelScene>,
    should_preserve: bool,
) {
    if !should_preserve {
        return;
    }
    let Some(preserved_scene) = preserved_scene.filter(native_panel_status_close_scene_has_cards)
    else {
        return;
    };
    sync_result.bundle.scene = preserved_scene;
}

fn play_message_sound_if_needed(enabled: bool) {
    if enabled {
        thread::spawn(play_message_card_sound);
    }
}

fn dispatch_snapshot_update<R: tauri::Runtime>(
    app: &AppHandle<R>,
    update: NativeSnapshotUpdate,
) -> Result<(), String> {
    let apply_state = update.apply_state;
    dispatch_native_panel_transition_request_or_apply_render_payload(
        app,
        update.transition_request,
        Some(update.snapshot.clone()),
        NativePanelRenderPayload {
            snapshot: update.snapshot,
            expanded: apply_state.expanded,
            shared_body_height: apply_state.shared_body_height,
            transitioning: apply_state.transitioning,
            transition_cards_progress: apply_state.transition_cards_progress,
            transition_cards_entering: apply_state.transition_cards_entering,
        },
    )
}

#[allow(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn apply_snapshot_to_panel(
    handles: NativePanelHandles,
    payload: &NativePanelRenderPayload,
) {
    apply_snapshot_values_to_panel(handles, &payload.snapshot);
    let context = resolve_native_transition_context(handles);
    let render_plan = resolve_snapshot_render_plan(&payload.snapshot);
    let mode = resolve_snapshot_apply_mode(context, &render_plan, payload);
    apply_snapshot_mode(handles, context, &render_plan, mode);
}

fn resolve_snapshot_apply_mode(
    context: super::transition_ui::NativeTransitionContext,
    render_plan: &NativePanelSnapshotRenderPlan,
    payload: &NativePanelRenderPayload,
) -> NativeSnapshotApplyMode {
    if let Some(mode) = resolve_transitioning_snapshot_apply_mode(payload) {
        return mode;
    }

    NativeSnapshotApplyMode::Static {
        expanded: payload.expanded,
        total_height: resolved_snapshot_panel_height_for_plan(
            context,
            render_plan,
            payload.expanded,
            payload.shared_body_height,
        ),
    }
}

fn resolve_transitioning_snapshot_apply_mode(
    payload: &NativePanelRenderPayload,
) -> Option<NativeSnapshotApplyMode> {
    if !payload.transitioning {
        return None;
    }

    Some(if payload.expanded {
        NativeSnapshotApplyMode::TransitioningExpanded(NativeCardTransitionState {
            progress: payload.transition_cards_progress,
            entering: payload.transition_cards_entering,
        })
    } else {
        NativeSnapshotApplyMode::TransitioningCollapsed
    })
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn apply_snapshot_mode(
    handles: NativePanelHandles,
    context: super::transition_ui::NativeTransitionContext,
    render_plan: &NativePanelSnapshotRenderPlan,
    mode: NativeSnapshotApplyMode,
) {
    match mode {
        NativeSnapshotApplyMode::TransitioningExpanded(cards) => {
            if context.refs.cards_container.subviews().is_empty() {
                render_transition_cards_with_plan(context, render_plan);
            }
            apply_card_stack_transition(
                context.refs.cards_container,
                cards.progress,
                cards.entering,
            );
        }
        NativeSnapshotApplyMode::TransitioningCollapsed => {}
        NativeSnapshotApplyMode::Static {
            expanded,
            total_height,
        } => {
            apply_snapshot_panel_geometry(handles, expanded, total_height);
            if expanded {
                render_transition_cards_with_plan(context, render_plan);
            } else {
                reset_collapsed_cards(context);
            }
        }
    }

    context.refs.panel.displayIfNeeded();
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn apply_snapshot_panel_geometry(
    handles: NativePanelHandles,
    expanded: bool,
    total_height: f64,
) {
    if expanded {
        apply_panel_geometry(handles, NativePanelTransitionFrame::expanded(total_height));
    } else {
        apply_panel_geometry(handles, NativePanelTransitionFrame::collapsed(total_height));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeSnapshotApplyMode, preserve_status_close_scene_if_needed,
        resolve_transitioning_snapshot_apply_mode,
    };
    use crate::{
        native_panel_core::{ExpandedSurface, PanelState},
        native_panel_renderer::facade::{
            renderer::sync_runtime_scene_bundle_from_state_input,
            testing::{
                test_native_panel_runtime_input_descriptor as runtime_input_descriptor,
                test_runtime_snapshot as snapshot,
            },
        },
        native_panel_scene::{SceneCard, SceneMascotPose, SurfaceSceneMode},
    };

    #[test]
    fn transitioning_snapshot_apply_mode_expanded_carries_card_transition_state() {
        let payload = crate::macos_native_panel::panel_types::NativePanelRenderPayload {
            snapshot: snapshot("running"),
            expanded: true,
            shared_body_height: Some(180.0),
            transitioning: true,
            transition_cards_progress: 0.42,
            transition_cards_entering: true,
        };
        let mode = resolve_transitioning_snapshot_apply_mode(&payload).expect("transitioning mode");

        match mode {
            NativeSnapshotApplyMode::TransitioningExpanded(cards) => {
                assert_eq!(cards.progress, 0.42);
                assert!(cards.entering);
            }
            NativeSnapshotApplyMode::TransitioningCollapsed
            | NativeSnapshotApplyMode::Static { .. } => {
                panic!("expected transitioning expanded mode")
            }
        }
    }

    #[test]
    fn transitioning_snapshot_apply_mode_collapsed_uses_collapsing_variant() {
        let payload = crate::macos_native_panel::panel_types::NativePanelRenderPayload {
            snapshot: snapshot("idle"),
            expanded: false,
            shared_body_height: Some(180.0),
            transitioning: true,
            transition_cards_progress: 0.18,
            transition_cards_entering: false,
        };
        let mode = resolve_transitioning_snapshot_apply_mode(&payload).expect("transitioning mode");

        match mode {
            NativeSnapshotApplyMode::TransitioningCollapsed => {}
            NativeSnapshotApplyMode::TransitioningExpanded(_)
            | NativeSnapshotApplyMode::Static { .. } => {
                panic!("expected transitioning collapsed mode")
            }
        }
    }

    #[test]
    fn status_close_preservation_keeps_mascot_scene_during_close_cache_refresh() {
        let input = runtime_input_descriptor();
        let mut state = PanelState::default();
        let mut sync_result = sync_runtime_scene_bundle_from_state_input(
            &mut state,
            &snapshot("idle"),
            &input,
            chrono::Utc::now(),
        );
        sync_result.bundle.scene.surface = ExpandedSurface::Default;
        sync_result.bundle.scene.cards.clear();
        sync_result.bundle.scene.mascot_pose = SceneMascotPose::Idle;
        sync_result.bundle.scene.compact_bar.completion_count = 0;

        let mut preserved_scene = sync_result.bundle.scene.clone();
        preserved_scene.surface = ExpandedSurface::Status;
        preserved_scene.surface_scene.mode = SurfaceSceneMode::Status;
        preserved_scene.cards = vec![SceneCard::Empty];
        preserved_scene.mascot_pose = SceneMascotPose::Complete;
        preserved_scene.compact_bar.completion_count = 1;

        preserve_status_close_scene_if_needed(&mut sync_result, Some(preserved_scene), true);

        assert_eq!(sync_result.bundle.scene.surface, ExpandedSurface::Status);
        assert_eq!(
            sync_result.bundle.scene.surface_scene.mode,
            SurfaceSceneMode::Status
        );
        assert_eq!(sync_result.bundle.scene.cards.len(), 1);
        assert_eq!(
            sync_result.bundle.scene.mascot_pose,
            SceneMascotPose::Complete
        );
        assert_eq!(sync_result.bundle.scene.compact_bar.completion_count, 1);
    }
}
