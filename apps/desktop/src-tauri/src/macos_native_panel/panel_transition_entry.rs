use std::sync::atomic::Ordering;

use echoisland_runtime::RuntimeSnapshot;
use tauri::AppHandle;

use super::panel_constants::COLLAPSED_PANEL_HEIGHT;
use super::panel_globals::NATIVE_TEST_PANEL_ANIMATION_ID;
use super::panel_refs::native_panel_state;
use super::panel_runtime_dispatch::take_pending_native_panel_transition_after_completion;
use super::panel_scene_adapter::resolve_snapshot_render_plan;
use super::panel_types::NativePanelHandles;
use super::panel_view_updates::apply_snapshot_values_to_panel;
use super::transition_logic::{
    finish_transition_state, set_transition_cards_state,
    take_skip_close_card_exit_and_begin_transition,
};
use super::transition_runner::animate_transition_request;
use super::transition_ui::{
    finalize_close_transition, finalize_open_transition, finalize_surface_switch_transition,
    prepare_close_transition, prepare_open_transition, prepare_surface_switch_transition,
    resolve_native_transition_context, resolved_expanded_target_height_for_plan,
};
use crate::native_panel_renderer::facade::renderer::{
    NativePanelClosePresentationInput, NativePanelClosePresentationPlan, NativePanelCloseTrigger,
    NativePanelStatusClosePreservationInput, NativePanelTransitionLifecyclePlan,
    resolve_native_panel_close_presentation_plan,
    resolve_native_panel_status_close_preservation_plan,
    resolve_native_panel_transition_lifecycle_plan,
};
use crate::native_panel_renderer::facade::{
    presentation::NativePanelSnapshotRenderPlan, transition::NativePanelTransitionRequest,
};

#[derive(Clone, Copy)]
enum NativePanelTransitionFinalizeKind {
    Open,
    Close,
    SurfaceSwitch,
}

#[derive(Clone, Copy)]
struct NativePanelPreparedTransition {
    request: NativePanelTransitionRequest,
    target_height: f64,
    card_count: usize,
    lifecycle: NativePanelTransitionLifecyclePlan,
    finalize: NativePanelTransitionFinalizeKind,
}

#[allow(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn begin_native_panel_transition<R: tauri::Runtime + 'static>(
    app: AppHandle<R>,
    handles: NativePanelHandles,
    snapshot: RuntimeSnapshot,
    expanded: bool,
) {
    let animation_id = next_animation_id();
    apply_snapshot_values_to_panel(handles, &snapshot);
    let skip_close_card_exit = take_skip_close_card_exit_and_begin_transition(expanded);
    let context = resolve_native_transition_context(handles);
    let start_height = context.refs.panel.frame().size.height;
    let render_plan = resolve_snapshot_render_plan(&snapshot);
    let shared_body_height = current_shared_body_height();
    let prepared = if expanded {
        prepare_open_entry_transition(context, &render_plan, shared_body_height)
    } else {
        prepare_close_entry_transition(context, skip_close_card_exit)
    };

    spawn_prepared_transition(
        app,
        handles,
        animation_id,
        start_height,
        render_plan,
        prepared,
    );
}

#[allow(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn begin_native_panel_surface_transition<R: tauri::Runtime + 'static>(
    app: AppHandle<R>,
    handles: NativePanelHandles,
    snapshot: RuntimeSnapshot,
) {
    let animation_id = next_animation_id();
    apply_snapshot_values_to_panel(handles, &snapshot);
    let _ = take_skip_close_card_exit_and_begin_transition(true);
    let context = resolve_native_transition_context(handles);
    let start_height = context.refs.panel.frame().size.height;
    let render_plan = resolve_snapshot_render_plan(&snapshot);
    let prepared = prepare_surface_switch_entry_transition(
        context,
        &render_plan,
        current_shared_body_height(),
    );

    spawn_prepared_transition(
        app,
        handles,
        animation_id,
        start_height,
        render_plan,
        prepared,
    );
}

fn next_animation_id() -> u64 {
    NATIVE_TEST_PANEL_ANIMATION_ID.fetch_add(1, Ordering::SeqCst) + 1
}

fn current_shared_body_height() -> Option<f64> {
    native_panel_state()
        .and_then(|state| state.lock().ok().and_then(|guard| guard.shared_body_height))
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn prepare_open_entry_transition(
    context: super::transition_ui::NativeTransitionContext,
    render_plan: &NativePanelSnapshotRenderPlan,
    shared_body_height: Option<f64>,
) -> NativePanelPreparedTransition {
    let lifecycle =
        resolve_native_panel_transition_lifecycle_plan(NativePanelTransitionRequest::Open);
    NativePanelPreparedTransition {
        request: NativePanelTransitionRequest::Open,
        target_height: resolved_expanded_target_height_for_plan(
            context,
            render_plan,
            shared_body_height,
        ),
        card_count: prepare_open_transition(
            context,
            render_plan,
            lifecycle.initial_card_phase.progress,
        ),
        lifecycle,
        finalize: NativePanelTransitionFinalizeKind::Open,
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn prepare_close_entry_transition(
    context: super::transition_ui::NativeTransitionContext,
    skip_close_card_exit: bool,
) -> NativePanelPreparedTransition {
    let lifecycle =
        resolve_native_panel_transition_lifecycle_plan(NativePanelTransitionRequest::Close);
    let close_plan = resolve_macos_close_presentation_plan(skip_close_card_exit);
    let skip_close_card_exit = skip_close_card_exit && close_plan.preserve_status_surface_only;
    NativePanelPreparedTransition {
        request: NativePanelTransitionRequest::Close,
        target_height: COLLAPSED_PANEL_HEIGHT,
        card_count: prepare_close_transition(
            context,
            skip_close_card_exit,
            lifecycle.initial_card_phase.progress,
        ),
        lifecycle,
        finalize: NativePanelTransitionFinalizeKind::Close,
    }
}

fn resolve_macos_close_presentation_plan(
    skip_close_card_exit: bool,
) -> NativePanelClosePresentationPlan {
    resolve_native_panel_close_presentation_plan(NativePanelClosePresentationInput {
        trigger: if skip_close_card_exit {
            NativePanelCloseTrigger::StatusAuto
        } else {
            NativePanelCloseTrigger::Hover
        },
        status_close: resolve_native_panel_status_close_preservation_plan(
            NativePanelStatusClosePreservationInput {
                last_transition_request: Some(NativePanelTransitionRequest::Close),
                skip_next_close_card_exit: skip_close_card_exit,
                transitioning: false,
                last_animation: None,
            },
        ),
        has_preserved_cards: true,
    })
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn prepare_surface_switch_entry_transition(
    context: super::transition_ui::NativeTransitionContext,
    render_plan: &NativePanelSnapshotRenderPlan,
    shared_body_height: Option<f64>,
) -> NativePanelPreparedTransition {
    let lifecycle =
        resolve_native_panel_transition_lifecycle_plan(NativePanelTransitionRequest::SurfaceSwitch);
    NativePanelPreparedTransition {
        request: NativePanelTransitionRequest::SurfaceSwitch,
        target_height: resolved_expanded_target_height_for_plan(
            context,
            render_plan,
            shared_body_height,
        ),
        card_count: prepare_surface_switch_transition(
            context,
            render_plan,
            lifecycle.initial_card_phase.progress,
        ),
        lifecycle,
        finalize: NativePanelTransitionFinalizeKind::SurfaceSwitch,
    }
}

fn spawn_prepared_transition<R: tauri::Runtime + 'static>(
    app: AppHandle<R>,
    handles: NativePanelHandles,
    animation_id: u64,
    start_height: f64,
    render_plan: NativePanelSnapshotRenderPlan,
    prepared: NativePanelPreparedTransition,
) {
    set_transition_cards_state(
        prepared.lifecycle.initial_card_phase.progress,
        prepared.lifecycle.initial_card_phase.entering,
    );

    tauri::async_runtime::spawn(async move {
        animate_transition_request(
            app.clone(),
            handles,
            animation_id,
            prepared.request,
            start_height,
            prepared.target_height,
            prepared.card_count,
        )
        .await;

        let app_after_finish = app.clone();
        let _ = app.run_on_main_thread(move || unsafe {
            if NATIVE_TEST_PANEL_ANIMATION_ID.load(Ordering::SeqCst) != animation_id {
                return;
            }

            finish_transition_state(
                prepared.lifecycle.final_card_phase.progress,
                prepared.lifecycle.final_card_phase.entering,
            );
            let context = resolve_native_transition_context(handles);
            finalize_prepared_transition(handles, context, &render_plan, prepared);
            begin_pending_transition_after_completion(app_after_finish, handles, prepared.request);
        });
    });
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn finalize_prepared_transition(
    handles: NativePanelHandles,
    context: super::transition_ui::NativeTransitionContext,
    render_plan: &NativePanelSnapshotRenderPlan,
    prepared: NativePanelPreparedTransition,
) {
    match prepared.finalize {
        NativePanelTransitionFinalizeKind::Open => {
            finalize_open_transition(handles, context, render_plan, prepared.target_height);
        }
        NativePanelTransitionFinalizeKind::Close => {
            finalize_close_transition(handles, context);
        }
        NativePanelTransitionFinalizeKind::SurfaceSwitch => {
            finalize_surface_switch_transition(
                handles,
                context,
                render_plan,
                prepared.target_height,
            );
        }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn begin_pending_transition_after_completion<R: tauri::Runtime + 'static>(
    app: AppHandle<R>,
    handles: NativePanelHandles,
    completed_request: NativePanelTransitionRequest,
) {
    let Some(pending) = take_pending_native_panel_transition_after_completion(completed_request)
    else {
        return;
    };
    match pending.request {
        NativePanelTransitionRequest::Open => {
            begin_native_panel_transition(app, handles, pending.snapshot, true);
        }
        NativePanelTransitionRequest::Close => {
            begin_native_panel_transition(app, handles, pending.snapshot, false);
        }
        NativePanelTransitionRequest::SurfaceSwitch => {
            begin_native_panel_surface_transition(app, handles, pending.snapshot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_macos_close_presentation_plan;

    #[test]
    fn macos_close_entry_maps_skip_close_to_shared_status_auto_policy() {
        let plan = resolve_macos_close_presentation_plan(true);

        assert!(plan.preserve_status_surface_only);
        assert!(plan.should_suppress_edge_actions);
        assert!(plan.should_remove_edge_action_hit_targets);
    }

    #[test]
    fn macos_close_entry_maps_normal_close_to_shared_hover_policy() {
        let plan = resolve_macos_close_presentation_plan(false);

        assert!(plan.preserve_any_surface);
        assert!(!plan.should_suppress_edge_actions);
        assert!(!plan.should_remove_edge_action_hit_targets);
    }
}
