use super::mascot::NativeMascotState;
use super::panel_chrome_alpha::resolve_collapsed_chrome_alpha;
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
        .unwrap_or(state.completion_badge_items.len());
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
    let base_state =
        normalize_status_surface_mascot_state(base_state, state.expanded, state.surface_mode);
    let glow_command = cached_bundle
        .as_ref()
        .and_then(|bundle| bundle.glow.clone())
        .or_else(|| {
            presentation
                .as_ref()
                .and_then(|model| model.glow.as_ref().map(|glow| glow.command()))
        });
    let collapsed_chrome_alpha = state
        .transition_collapsed_chrome_alpha
        .or_else(|| {
            presentation
                .as_ref()
                .map(collapsed_chrome_alpha_from_presentation)
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

fn collapsed_chrome_alpha_from_presentation(
    model: &crate::native_panel_renderer::facade::presentation::NativePanelPresentationModel,
) -> f64 {
    resolve_collapsed_chrome_alpha(crate::native_panel_core::PanelChromeVisibilitySpecInput {
        expanded_display_mode: model.shell.visible,
        surface: model.shell.surface,
        edge_actions_visible: model.action_buttons.visible,
        transition_visibility_progress: model.shell.chrome_transition_progress,
    })
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

fn normalize_status_surface_mascot_state(
    state: NativeMascotState,
    expanded: bool,
    surface_mode: NativeExpandedSurface,
) -> NativeMascotState {
    if expanded
        && surface_mode == NativeExpandedSurface::Status
        && state == NativeMascotState::MessageBubble
    {
        NativeMascotState::Complete
    } else {
        state
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_status_surface_mascot_state;
    use crate::macos_native_panel::{
        mascot::NativeMascotState, panel_types::NativeExpandedSurface,
    };

    #[test]
    fn status_surface_uses_complete_mascot_instead_of_transient_message_bubble() {
        assert!(matches!(
            normalize_status_surface_mascot_state(
                NativeMascotState::MessageBubble,
                true,
                NativeExpandedSurface::Status,
            ),
            NativeMascotState::Complete
        ));
    }

    #[test]
    fn collapsed_mascot_can_still_show_message_bubble() {
        assert!(matches!(
            normalize_status_surface_mascot_state(
                NativeMascotState::MessageBubble,
                false,
                NativeExpandedSurface::Default,
            ),
            NativeMascotState::MessageBubble
        ));
    }
}
