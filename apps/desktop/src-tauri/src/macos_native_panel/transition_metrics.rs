use tracing::warn;

use crate::native_panel_renderer::facade::transition::NativePanelTransitionRequest;

use super::transition_ui::NativeTransitionTimelineFrameMetrics;

#[derive(Clone, Copy, Debug)]
pub(super) struct NativePanelAnimationFrameMetrics {
    pub(super) queued_ms: u128,
    pub(super) apply_ms: u128,
    pub(super) continue_animating: bool,
    pub(super) timeline: NativeTransitionTimelineFrameMetrics,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NativePanelAnimationSummary {
    animation_id: u64,
    request: NativePanelTransitionRequest,
    start_height: f64,
    target_height: f64,
    card_count: usize,
    frame_count: usize,
    slow_frame_count: usize,
    total_queued_ms: u128,
    total_apply_ms: u128,
    max_queued_ms: u128,
    max_apply_ms: u128,
    max_timeline_ms: u128,
    max_geometry_ms: u128,
    max_geometry_total_ms: u128,
    max_visuals_ms: u128,
    max_sync_ms: u128,
    max_alpha_ms: u128,
    max_context_ms: u128,
    max_cards_ms: u128,
    max_mark_ms: u128,
    max_view_ms: u128,
    max_layer_ms: u128,
    max_invalidate_ms: u128,
    cached_collapsed_frames: usize,
    full_sync_frames: usize,
    transition_sync_frames: usize,
    entering_frames: usize,
    exiting_frames: usize,
    final_continue_animating: bool,
    final_width_progress: f64,
    final_height_progress: f64,
    final_cards_progress: f64,
    final_shell_visible: bool,
}

impl NativePanelAnimationSummary {
    pub(super) fn new(
        animation_id: u64,
        request: NativePanelTransitionRequest,
        start_height: f64,
        target_height: f64,
        card_count: usize,
    ) -> Self {
        Self {
            animation_id,
            request,
            start_height,
            target_height,
            card_count,
            frame_count: 0,
            slow_frame_count: 0,
            total_queued_ms: 0,
            total_apply_ms: 0,
            max_queued_ms: 0,
            max_apply_ms: 0,
            max_timeline_ms: 0,
            max_geometry_ms: 0,
            max_geometry_total_ms: 0,
            max_visuals_ms: 0,
            max_sync_ms: 0,
            max_alpha_ms: 0,
            max_context_ms: 0,
            max_cards_ms: 0,
            max_mark_ms: 0,
            max_view_ms: 0,
            max_layer_ms: 0,
            max_invalidate_ms: 0,
            cached_collapsed_frames: 0,
            full_sync_frames: 0,
            transition_sync_frames: 0,
            entering_frames: 0,
            exiting_frames: 0,
            final_continue_animating: false,
            final_width_progress: 0.0,
            final_height_progress: 0.0,
            final_cards_progress: 0.0,
            final_shell_visible: false,
        }
    }

    pub(super) fn record(&mut self, metrics: NativePanelAnimationFrameMetrics) {
        let timeline = metrics.timeline;
        let geometry = timeline.geometry;
        self.frame_count += 1;
        self.total_queued_ms += metrics.queued_ms;
        self.total_apply_ms += metrics.apply_ms;
        self.max_queued_ms = self.max_queued_ms.max(metrics.queued_ms);
        self.max_apply_ms = self.max_apply_ms.max(metrics.apply_ms);
        self.max_timeline_ms = self.max_timeline_ms.max(timeline.total_ms);
        self.max_geometry_ms = self.max_geometry_ms.max(timeline.geometry_ms);
        self.max_geometry_total_ms = self.max_geometry_total_ms.max(geometry.total_ms);
        self.max_visuals_ms = self.max_visuals_ms.max(geometry.visuals_ms);
        self.max_sync_ms = self.max_sync_ms.max(geometry.sync_ms);
        self.max_alpha_ms = self.max_alpha_ms.max(geometry.alpha_ms);
        self.max_context_ms = self.max_context_ms.max(timeline.context_ms);
        self.max_cards_ms = self.max_cards_ms.max(timeline.cards_ms);
        self.max_mark_ms = self.max_mark_ms.max(timeline.mark_ms);
        self.max_view_ms = self.max_view_ms.max(geometry.view_ms);
        self.max_layer_ms = self.max_layer_ms.max(geometry.layer_ms);
        self.max_invalidate_ms = self.max_invalidate_ms.max(geometry.invalidate_ms);
        if metrics.queued_ms >= 24 || metrics.apply_ms >= 20 || timeline.total_ms >= 20 {
            self.slow_frame_count += 1;
        }
        match geometry.sync_path {
            "cached_collapsed" => self.cached_collapsed_frames += 1,
            "full" => self.full_sync_frames += 1,
            "transition" => self.transition_sync_frames += 1,
            _ => {}
        }
        if timeline.cards_entering {
            self.entering_frames += 1;
        } else {
            self.exiting_frames += 1;
        }
        self.final_continue_animating = metrics.continue_animating;
        self.final_width_progress = timeline.width_progress;
        self.final_height_progress = timeline.height_progress;
        self.final_cards_progress = timeline.cards_progress;
        self.final_shell_visible = geometry.shell_visible;
    }

    pub(super) fn log(&self, elapsed_ms: u128) {
        if self.frame_count == 0 {
            return;
        }
        let avg_queued_ms = self.total_queued_ms / self.frame_count as u128;
        let avg_apply_ms = self.total_apply_ms / self.frame_count as u128;
        warn!(
            animation_id = self.animation_id,
            request = ?self.request,
            elapsed_ms,
            frame_count = self.frame_count,
            slow_frame_count = self.slow_frame_count,
            start_height = self.start_height,
            target_height = self.target_height,
            card_count = self.card_count,
            avg_queued_ms,
            avg_apply_ms,
            max_queued_ms = self.max_queued_ms,
            max_apply_ms = self.max_apply_ms,
            max_timeline_ms = self.max_timeline_ms,
            max_geometry_ms = self.max_geometry_ms,
            max_geometry_total_ms = self.max_geometry_total_ms,
            max_visuals_ms = self.max_visuals_ms,
            max_sync_ms = self.max_sync_ms,
            max_alpha_ms = self.max_alpha_ms,
            max_context_ms = self.max_context_ms,
            max_cards_ms = self.max_cards_ms,
            max_mark_ms = self.max_mark_ms,
            max_view_ms = self.max_view_ms,
            max_layer_ms = self.max_layer_ms,
            max_invalidate_ms = self.max_invalidate_ms,
            cached_collapsed_frames = self.cached_collapsed_frames,
            full_sync_frames = self.full_sync_frames,
            transition_sync_frames = self.transition_sync_frames,
            entering_frames = self.entering_frames,
            exiting_frames = self.exiting_frames,
            final_continue_animating = self.final_continue_animating,
            final_width_progress = self.final_width_progress,
            final_height_progress = self.final_height_progress,
            final_cards_progress = self.final_cards_progress,
            final_shell_visible = self.final_shell_visible,
            "native panel animation summary"
        );
    }
}

pub(super) fn native_panel_animation_metrics_enabled() -> bool {
    crate::app_settings::current_app_settings().debug_mode_enabled
}
