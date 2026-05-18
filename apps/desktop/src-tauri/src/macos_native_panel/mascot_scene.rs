use super::mascot::NativeMascotState;
use super::panel_scene_adapter::{
    resolve_current_native_panel_presentation_model,
    resolve_current_native_panel_render_command_bundle,
};
use super::panel_types::{NativeExpandedSurface, NativePanelState, NativeStatusQueuePayload};
use crate::native_panel_scene::visible_panel_mascot_state_from_scene_pose;

pub(super) struct NativeMascotFrameInput {
    pub(super) base_state: NativeMascotState,
    pub(super) expanded: bool,
    pub(super) completion_count: usize,
    pub(super) mascot_hidden: bool,
    pub(super) debug_mode_enabled: bool,
    pub(super) collapsed_chrome_alpha: f64,
    pub(super) completion_glow_opacity: f64,
}

pub(super) fn resolve_native_mascot_frame_input(
    state: &NativePanelState,
) -> NativeMascotFrameInput {
    let cached_bundle = resolve_current_native_panel_render_command_bundle(state);
    let snapshot = state.last_snapshot.clone();
    let presentation = resolve_current_native_panel_presentation_model(state);
    let completion_count = cached_bundle
        .as_ref()
        .map(|bundle| bundle.compact_bar.completion_count)
        .or_else(|| {
            presentation
                .as_ref()
                .map(|model| model.compact_bar.completion_count)
        })
        .unwrap_or_else(|| state.completion_badge_items.len());
    let mascot_command = cached_bundle
        .as_ref()
        .map(|bundle| bundle.mascot.clone())
        .or_else(|| presentation.as_ref().map(|model| model.mascot.command()));
    let has_status_completion = state.expanded
        && state.surface_mode == NativeExpandedSurface::Status
        && state
            .status_queue
            .iter()
            .any(|item| matches!(item.payload, NativeStatusQueuePayload::Completion(_)));
    let has_completion_badge = !state.completion_badge_items.is_empty();
    let base_state = mascot_command
        .as_ref()
        .and_then(|command| visible_panel_mascot_state_from_scene_pose(command.pose))
        .map(native_mascot_state_from_core)
        .unwrap_or_else(|| {
            native_mascot_state_from_core(crate::native_panel_core::resolve_mascot_base_state(
                snapshot.as_ref(),
                has_status_completion,
                has_completion_badge,
            ))
        });
    let glow_command = cached_bundle
        .as_ref()
        .and_then(|bundle| bundle.glow.clone())
        .or_else(|| {
            presentation
                .as_ref()
                .and_then(|model| model.glow.as_ref().map(|glow| glow.command()))
        });
    let collapsed_chrome_alpha = presentation
        .as_ref()
        .map(|model| {
            let chrome = crate::native_panel_core::resolve_panel_chrome_visibility_spec(
                crate::native_panel_core::PanelChromeVisibilitySpecInput {
                    expanded_display_mode: model.shell.visible,
                    surface: model.shell.surface,
                    edge_actions_visible: model.action_buttons.visible,
                    transition_visibility_progress: model.shell.chrome_transition_progress,
                },
            );
            if chrome.collapsed_mascot_visible {
                1.0 - chrome.collapsed_exit_progress.clamp(0.0, 1.0)
            } else {
                0.0
            }
        })
        .unwrap_or(if state.expanded { 0.0 } else { 1.0 });

    NativeMascotFrameInput {
        base_state,
        expanded: state.expanded,
        completion_count,
        mascot_hidden: mascot_command.as_ref().is_some_and(|command| {
            command.pose == crate::native_panel_scene::SceneMascotPose::Hidden
        }),
        debug_mode_enabled: mascot_command
            .as_ref()
            .map(|command| command.debug_mode_enabled)
            .unwrap_or(false),
        collapsed_chrome_alpha,
        completion_glow_opacity: glow_command
            .map(|command| command.glow.opacity)
            .unwrap_or(0.0),
    }
}

fn native_mascot_state_from_core(
    state: crate::native_panel_core::PanelMascotBaseState,
) -> NativeMascotState {
    match state {
        crate::native_panel_core::PanelMascotBaseState::Idle => NativeMascotState::Idle,
        crate::native_panel_core::PanelMascotBaseState::Running => NativeMascotState::Bouncing,
        crate::native_panel_core::PanelMascotBaseState::Approval => NativeMascotState::Approval,
        crate::native_panel_core::PanelMascotBaseState::Question => NativeMascotState::Question,
        crate::native_panel_core::PanelMascotBaseState::MessageBubble => {
            NativeMascotState::MessageBubble
        }
        crate::native_panel_core::PanelMascotBaseState::Complete => NativeMascotState::Complete,
        crate::native_panel_core::PanelMascotBaseState::Sleepy => NativeMascotState::Sleepy,
        crate::native_panel_core::PanelMascotBaseState::WakeAngry => NativeMascotState::WakeAngry,
    }
}
