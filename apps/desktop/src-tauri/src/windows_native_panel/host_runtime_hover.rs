use std::time::Instant;

use tracing::info;

use super::{
    host_runtime::WindowsNativePanelRuntime,
    platform_loop::current_windows_native_panel_pointer_polling_sample,
};
use crate::{
    native_panel_core::{HoverTransition, PanelPoint},
    native_panel_renderer::facade::{
        command::NativePanelPointerInput,
        descriptor::NativePanelRuntimeInputDescriptor,
        host::NativePanelHost,
        interaction::{
            NativePanelHostBehaviorCommand, NativePanelHostPollingInteractionResult,
            NativePanelPollingHostFacts, native_panel_interactive_inside_from_host_facts,
            native_panel_polling_interaction_input_from_host_facts,
            sync_native_panel_host_behavior_for_interactive_inside,
            sync_native_panel_host_polling_interaction_for_state,
            sync_native_panel_hover_interaction_at_point_for_state,
            sync_native_panel_hover_interaction_for_pointer_input_for_state,
        },
        presentation::NativePanelPresentationModel,
        renderer::{NativePanelCloseTrigger, NativePanelRuntimeSceneStateBridge},
        transition::NativePanelTransitionRequest,
    },
};

impl WindowsNativePanelRuntime {
    pub(super) fn sync_hover_at_point(
        &mut self,
        point: PanelPoint,
        now: Instant,
    ) -> Option<HoverTransition> {
        sync_native_panel_hover_interaction_at_point_for_state(
            &mut self.panel_state,
            &self.host,
            point,
            now,
            crate::native_panel_core::HOVER_DELAY_MS,
        )
    }

    pub(super) fn sync_hover_for_pointer_input(
        &mut self,
        input: NativePanelPointerInput,
        now: Instant,
    ) -> Option<HoverTransition> {
        sync_native_panel_hover_interaction_for_pointer_input_for_state(
            &mut self.panel_state,
            &self.host,
            input,
            now,
            crate::native_panel_core::HOVER_DELAY_MS,
        )
    }

    pub(super) fn polling_host_facts(
        &self,
        pointer: PanelPoint,
        primary_mouse_down: bool,
    ) -> Option<NativePanelPollingHostFacts<'_>> {
        self.host.shell.polling_host_facts(
            pointer,
            primary_mouse_down,
            self.runtime_scene_current_snapshot().cloned(),
        )
    }

    pub(super) fn interactive_inside_for_pointer_input(
        &self,
        input: NativePanelPointerInput,
    ) -> Option<bool> {
        match input {
            NativePanelPointerInput::Move(point) => self
                .polling_host_facts(point, false)
                .map(native_panel_interactive_inside_from_host_facts),
            NativePanelPointerInput::Leave => Some(false),
            NativePanelPointerInput::Click(_) => None,
        }
    }

    pub(super) fn sync_mouse_passthrough_for_pointer_input(
        &mut self,
        input: NativePanelPointerInput,
    ) {
        let Some(interactive_inside) = self.interactive_inside_for_pointer_input(input) else {
            return;
        };
        let plan = sync_native_panel_host_behavior_for_interactive_inside(self, interactive_inside);
        self.apply_host_behavior_commands(plan.commands);
    }

    pub(super) fn apply_host_behavior_commands(
        &mut self,
        commands: impl IntoIterator<Item = NativePanelHostBehaviorCommand>,
    ) {
        for command in commands {
            match command {
                NativePanelHostBehaviorCommand::SetMouseEventPassthrough {
                    ignores_mouse_events,
                } => self
                    .host
                    .sync_shell_mouse_event_passthrough(ignores_mouse_events),
            }
        }
    }

    pub(super) fn sync_host_polling_interaction(
        &mut self,
        pointer: PanelPoint,
        primary_mouse_down: bool,
        now: Instant,
    ) -> Option<NativePanelHostPollingInteractionResult> {
        let facts = self.polling_host_facts(pointer, primary_mouse_down)?;
        let input = native_panel_polling_interaction_input_from_host_facts(facts);
        let interaction = sync_native_panel_host_polling_interaction_for_state(
            self,
            input,
            now,
            crate::native_panel_core::HOVER_DELAY_MS,
            crate::native_panel_core::CARD_FOCUS_CLICK_DEBOUNCE_MS,
        );
        self.apply_host_behavior_commands(interaction.host_behavior.commands.clone());
        Some(interaction)
    }

    pub(super) fn sync_host_polling_interaction_and_refresh(
        &mut self,
        pointer: PanelPoint,
        primary_mouse_down: bool,
        now: Instant,
        input: &NativePanelRuntimeInputDescriptor,
    ) -> Result<Option<NativePanelHostPollingInteractionResult>, String> {
        let Some(interaction) =
            self.sync_host_polling_interaction(pointer, primary_mouse_down, now)
        else {
            return Ok(None);
        };
        if interaction.transition_request.is_some() {
            self.last_transition_request = interaction.transition_request;
            let previous_transitioning = self.panel_state.transitioning;
            let is_close =
                interaction.transition_request == Some(NativePanelTransitionRequest::Close);
            if is_close {
                self.panel_state.transitioning = true;
                self.hover_close_in_progress = true;
            }
            // Hover sync already flipped panel_state.expanded = false. The upcoming
            // rerender rebuilds the scene as collapsed and clears the card stack from
            // scene_cache. Capture the soon-to-be-lost cards now so we can restore them
            // onto the rebuilt scene; without this the close animation starts with
            // card_count = 0 and visually skips the card-exit phase.
            let preserved_close_presentation = is_close
                .then(|| self.capture_presentation_for_hover_close_transition())
                .flatten();
            if let Err(error) = self.rerender_from_last_snapshot_with_input(input) {
                self.panel_state.transitioning = previous_transitioning;
                if is_close {
                    self.hover_close_in_progress = false;
                }
                return Err(error);
            }
            if is_close {
                let plan = self.close_presentation_plan(
                    NativePanelCloseTrigger::Hover,
                    preserved_close_presentation.is_some(),
                );
                if plan.should_capture_card_stack {
                    // Stash for refresh_status_queue_from_last_raw_snapshot_with_input,
                    // which fires on subsequent ticks while a status_queue / pending card
                    // is still present and would otherwise wipe our preserved cards.
                    self.pending_close_presentation = preserved_close_presentation.clone();
                }
                self.host
                    .renderer
                    .apply_close_presentation_plan(preserved_close_presentation.as_ref(), plan);
                self.host.present_renderer_state()?;
            }
        }
        Ok(Some(interaction))
    }

    pub(super) fn capture_presentation_for_hover_close_transition(
        &self,
    ) -> Option<NativePanelPresentationModel> {
        self.host
            .window
            .presented_presentation_model
            .as_ref()
            .or(self.host.renderer.last_presentation_model.as_ref())
            .cloned()
            .filter(|presentation| !presentation.card_stack.cards.is_empty())
    }

    pub(super) fn sync_current_pointer_polling_interaction(
        &mut self,
        now: Instant,
        input: &NativePanelRuntimeInputDescriptor,
    ) -> Result<Option<NativePanelHostPollingInteractionResult>, String> {
        if self.user_hidden {
            return Ok(None);
        }
        let Some(sample) = current_windows_native_panel_pointer_polling_sample(
            self.host.shell.raw_window_handle(),
        ) else {
            log_windows_native_hover_probe(
                self.host.shell.raw_window_handle(),
                None,
                self.host.shell.pointer_regions().len(),
                None,
                None,
                self.panel_state.expanded,
            );
            return Ok(None);
        };
        let interaction =
            self.sync_host_polling_interaction_and_refresh(sample.point, false, now, input)?;
        log_windows_native_hover_probe(
            self.host.shell.raw_window_handle(),
            Some(sample.point),
            self.host.shell.pointer_regions().len(),
            interaction.as_ref().map(|result| result.interactive_inside),
            interaction
                .as_ref()
                .and_then(|result| result.transition_request),
            self.panel_state.expanded,
        );
        Ok(interaction)
    }
}

fn windows_native_hover_probe_enabled() -> bool {
    std::env::var("ECHOISLAND_WINDOWS_HOVER_PROBE")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn log_windows_native_hover_probe(
    raw_window_handle: Option<isize>,
    pointer: Option<PanelPoint>,
    pointer_region_count: usize,
    interactive_inside: Option<bool>,
    transition_request: Option<NativePanelTransitionRequest>,
    expanded: bool,
) {
    if !windows_native_hover_probe_enabled() {
        return;
    }
    info!(
        raw_window_handle = ?raw_window_handle,
        pointer_x = pointer.map(|point| point.x),
        pointer_y = pointer.map(|point| point.y),
        pointer_region_count,
        interactive_inside,
        transition_request = ?transition_request,
        expanded,
        "windows native hover probe"
    );
}
