use echoisland_runtime::RuntimeSnapshot;

use super::host_runtime::WindowsNativePanelRuntime;
use crate::{
    native_panel_core::{PanelSnapshotSyncResult, panel_state_needs_status_queue_refresh},
    native_panel_renderer::facade::{
        descriptor::NativePanelRuntimeInputDescriptor,
        host::NativePanelHost,
        presentation::NativePanelPresentationModel,
        renderer::{
            NativePanelClosePresentationInput, NativePanelClosePresentationPlan,
            NativePanelCloseTrigger, NativePanelStatusClosePreservationInput,
            NativePanelStatusClosePreservationPlan, resolve_native_panel_close_presentation_plan,
            resolve_native_panel_status_close_preservation_plan,
        },
        runtime::sync_runtime_scene_bundle_for_runtime_with_input,
        transition::NativePanelTransitionRequest,
    },
};

impl WindowsNativePanelRuntime {
    pub(super) fn sync_snapshot_bundle(
        &mut self,
        snapshot: &RuntimeSnapshot,
        input: &NativePanelRuntimeInputDescriptor,
    ) -> Result<Option<PanelSnapshotSyncResult>, String> {
        if self.user_hidden {
            return Ok(None);
        }
        let preserved_close_presentation = self.capture_close_preservation_presentation();
        let active_close_before_sync = self.status_close_preservation_plan().active_close;
        let trigger = self.close_presentation_trigger();
        let plan = self.close_presentation_plan(trigger, preserved_close_presentation.is_some());
        let sync = sync_runtime_scene_bundle_for_runtime_with_input(self, snapshot, input)?;
        if active_close_before_sync && plan.should_apply_preserved_card_stack {
            self.apply_close_preservation_presentation(preserved_close_presentation.as_ref(), plan);
            self.host.present_renderer_state()?;
        }
        self.store_pending_close_presentation_if_needed(preserved_close_presentation, plan);
        Ok(Some(sync))
    }

    pub(super) fn refresh_status_queue_from_last_raw_snapshot_with_input(
        &mut self,
        input: &NativePanelRuntimeInputDescriptor,
    ) -> Result<bool, String> {
        let Some(snapshot) = self.status_queue_refresh_snapshot() else {
            return Ok(false);
        };
        let pending_transition = self.last_transition_request;
        let active_close_before_refresh = self.status_close_preservation_plan().active_close;
        let preserved_close_presentation = self.capture_close_preservation_presentation();
        let trigger = self.close_presentation_trigger();
        let plan_before_refresh =
            self.close_presentation_plan(trigger, preserved_close_presentation.is_some());
        sync_runtime_scene_bundle_for_runtime_with_input(self, &snapshot, input)?;
        if self.last_transition_request.is_none() {
            self.last_transition_request = pending_transition;
        }
        let close_preservation_after_refresh = self.status_close_preservation_plan();
        let plan_after_refresh =
            self.close_presentation_plan(trigger, preserved_close_presentation.is_some());
        if (active_close_before_refresh || close_preservation_after_refresh.pending_close)
            && plan_after_refresh.should_apply_preserved_card_stack
        {
            self.apply_close_preservation_presentation(
                preserved_close_presentation.as_ref(),
                plan_after_refresh,
            );
            self.host.present_renderer_state()?;
        }
        self.store_pending_close_presentation_if_needed(
            preserved_close_presentation,
            if close_preservation_after_refresh.pending_close {
                plan_after_refresh
            } else {
                plan_before_refresh
            },
        );
        Ok(true)
    }

    /// Capture the presentation state that should survive the next mid-close re-render.
    /// Hover-driven close preserves cards from any surface (and falls back to
    /// the stash we set when the close was kicked off, since the currently
    /// presented model may already have been mutated by an earlier preserve
    /// pass on this tick). Status-queue auto-collapse keeps the original
    /// Status-only filter. Keeping the full presentation avoids mascot flicker
    /// when a refresh rebuilds the scene with default mascot state mid-close.
    pub(super) fn capture_close_preservation_presentation(
        &self,
    ) -> Option<NativePanelPresentationModel> {
        match self.close_presentation_trigger() {
            NativePanelCloseTrigger::Hover => self
                .capture_presentation_for_hover_close_transition()
                .or_else(|| self.pending_close_presentation.clone()),
            NativePanelCloseTrigger::StatusAuto | NativePanelCloseTrigger::MessageAuto => {
                self.capture_status_presentation_for_close_transition()
            }
            NativePanelCloseTrigger::Explicit => None,
        }
    }

    /// Re-apply the captured presentation onto the just-rebuilt scene. Hover-
    /// driven close uses the slim variant that does not suppress edge action
    /// buttons, so settings / quit fade out via the natural width morph.
    pub(super) fn apply_close_preservation_presentation(
        &mut self,
        preserved: Option<&NativePanelPresentationModel>,
        plan: NativePanelClosePresentationPlan,
    ) {
        self.host
            .renderer
            .apply_close_presentation_plan(preserved, plan);
    }

    pub(super) fn capture_status_presentation_for_close_transition(
        &self,
    ) -> Option<NativePanelPresentationModel> {
        self.host
            .window
            .presented_presentation_model
            .as_ref()
            .or(self.host.renderer.last_presentation_model.as_ref())
            .cloned()
            .filter(|presentation| {
                presentation.card_stack.surface == crate::native_panel_core::ExpandedSurface::Status
                    && !presentation.card_stack.cards.is_empty()
            })
    }

    pub(super) fn store_pending_close_presentation_if_needed(
        &mut self,
        presentation: Option<NativePanelPresentationModel>,
        plan: NativePanelClosePresentationPlan,
    ) {
        if plan.should_capture_card_stack {
            self.pending_close_presentation = presentation;
        }
        if plan.should_clear_pending_stack {
            self.pending_close_presentation = None;
        }
    }

    pub(super) fn close_presentation_trigger(&self) -> NativePanelCloseTrigger {
        if self.hover_close_in_progress {
            NativePanelCloseTrigger::Hover
        } else {
            NativePanelCloseTrigger::StatusAuto
        }
    }

    pub(super) fn close_presentation_plan(
        &self,
        trigger: NativePanelCloseTrigger,
        has_preserved_cards: bool,
    ) -> NativePanelClosePresentationPlan {
        self.close_presentation_plan_for_request(
            trigger,
            self.last_transition_request,
            has_preserved_cards,
        )
    }

    pub(super) fn close_presentation_plan_for_request(
        &self,
        trigger: NativePanelCloseTrigger,
        last_transition_request: Option<NativePanelTransitionRequest>,
        has_preserved_cards: bool,
    ) -> NativePanelClosePresentationPlan {
        resolve_native_panel_close_presentation_plan(NativePanelClosePresentationInput {
            trigger,
            status_close: self.status_close_preservation_plan_for_request(last_transition_request),
            has_preserved_cards,
        })
    }

    pub(super) fn status_queue_refresh_snapshot(&self) -> Option<RuntimeSnapshot> {
        if self.user_hidden || !panel_state_needs_status_queue_refresh(&self.panel_state) {
            return None;
        }
        self.panel_state.last_raw_snapshot.clone()
    }

    pub(super) fn status_close_transition_active(&self) -> bool {
        self.status_close_preservation_plan().active_close
    }

    pub(super) fn should_preserve_pending_status_close_frame(&self) -> bool {
        self.status_close_preservation_plan().pending_close
    }

    pub(super) fn status_close_preservation_plan(&self) -> NativePanelStatusClosePreservationPlan {
        self.status_close_preservation_plan_for_request(self.last_transition_request)
    }

    pub(super) fn status_close_preservation_plan_for_request(
        &self,
        last_transition_request: Option<NativePanelTransitionRequest>,
    ) -> NativePanelStatusClosePreservationPlan {
        resolve_native_panel_status_close_preservation_plan(
            NativePanelStatusClosePreservationInput {
                last_transition_request,
                skip_next_close_card_exit: self.panel_state.skip_next_close_card_exit,
                transitioning: self.panel_state.transitioning,
                last_animation: self.last_animation_descriptor,
            },
        )
    }
}
