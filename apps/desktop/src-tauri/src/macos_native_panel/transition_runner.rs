use std::sync::atomic::Ordering;
use std::time::Instant;

use tauri::AppHandle;
use tokio::sync::oneshot;
use tokio::time::Duration;

use crate::{
    native_panel_core::PANEL_ANIMATION_FRAME_MS,
    native_panel_renderer::facade::{
        renderer::{NativePanelAnimationFrame, NativePanelAnimationFrameScheduler},
        transition::NativePanelTransitionRequest,
    },
};

use super::panel_globals::NATIVE_TEST_PANEL_ANIMATION_ID;
use super::panel_types::NativePanelHandles;
use super::panel_view_updates::with_disabled_layer_actions;
use super::transition_logic::update_timeline_transition_state_from_descriptor;
use super::transition_metrics::{
    NativePanelAnimationFrameMetrics, NativePanelAnimationSummary,
    native_panel_animation_metrics_enabled,
};
use super::transition_ui::{
    apply_transition_timeline_descriptor, apply_transition_timeline_descriptor_without_metrics,
};

pub(super) async fn animate_transition_request<R: tauri::Runtime + 'static>(
    app: AppHandle<R>,
    handles: NativePanelHandles,
    animation_id: u64,
    request: NativePanelTransitionRequest,
    start_height: f64,
    target_height: f64,
    card_count: usize,
) {
    animate_panel_timeline(
        app,
        handles,
        animation_id,
        request,
        start_height,
        target_height,
        card_count,
    )
    .await;
}

async fn animate_panel_timeline<R>(
    app: AppHandle<R>,
    handles: NativePanelHandles,
    animation_id: u64,
    request: NativePanelTransitionRequest,
    start_height: f64,
    target_height: f64,
    card_count: usize,
) where
    R: tauri::Runtime + 'static,
{
    let metrics_enabled = native_panel_animation_metrics_enabled();
    let animation_started_at = metrics_enabled.then(Instant::now);
    let mut summary = metrics_enabled.then(|| {
        NativePanelAnimationSummary::new(
            animation_id,
            request,
            start_height,
            target_height,
            card_count,
        )
    });
    let mut scheduler = NativePanelAnimationFrameScheduler::default();
    let initial_frame = scheduler.start(
        native_panel_animation_target(request, start_height, target_height, card_count),
        Instant::now(),
    );
    if let Some(metrics) = apply_animation_frame_on_main_thread(
        &app,
        handles,
        animation_id,
        initial_frame,
        metrics_enabled,
    )
    .await
    {
        if let Some(summary) = summary.as_mut() {
            summary.record(metrics);
        }
    }

    while scheduler.is_active() {
        let delay_ms = scheduler
            .next_frame_delay_ms()
            .unwrap_or(PANEL_ANIMATION_FRAME_MS);
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        let Some(frame) = scheduler.sample(Instant::now()) else {
            continue;
        };
        if let Some(metrics) = apply_animation_frame_on_main_thread(
            &app,
            handles,
            animation_id,
            frame,
            metrics_enabled,
        )
        .await
        {
            if let Some(summary) = summary.as_mut() {
                summary.record(metrics);
            }
        }
        if !frame.continue_animating {
            break;
        }
    }
    if let (Some(summary), Some(animation_started_at)) = (summary, animation_started_at) {
        summary.log(animation_started_at.elapsed().as_millis());
    }
}

fn native_panel_animation_target(
    request: NativePanelTransitionRequest,
    start_height: f64,
    target_height: f64,
    card_count: usize,
) -> crate::native_panel_renderer::facade::renderer::NativePanelAnimationTarget {
    crate::native_panel_renderer::facade::renderer::NativePanelAnimationTarget {
        request,
        start_height,
        target_height,
        card_count,
    }
}

async fn apply_animation_frame_on_main_thread<R: tauri::Runtime + 'static>(
    app: &AppHandle<R>,
    handles: NativePanelHandles,
    animation_id: u64,
    frame: NativePanelAnimationFrame,
    metrics_enabled: bool,
) -> Option<NativePanelAnimationFrameMetrics> {
    let timeline_descriptor = frame.plan.timeline;
    let continue_animating = frame.continue_animating;
    let queued_at = metrics_enabled.then(Instant::now);
    let (done_tx, done_rx) = oneshot::channel();
    let scheduled = app.run_on_main_thread(move || unsafe {
        let main_started_at = metrics_enabled.then(Instant::now);
        let queued_ms = match (queued_at, main_started_at) {
            (Some(queued_at), Some(main_started_at)) => {
                main_started_at.duration_since(queued_at).as_millis()
            }
            _ => 0,
        };
        if NATIVE_TEST_PANEL_ANIMATION_ID.load(Ordering::SeqCst) != animation_id {
            let _ = done_tx.send(None);
            return;
        }
        let apply_started_at = metrics_enabled.then(Instant::now);
        let metrics = with_disabled_layer_actions(|| {
            update_timeline_transition_state_from_descriptor(timeline_descriptor);
            if metrics_enabled {
                Some(apply_transition_timeline_descriptor(
                    handles,
                    timeline_descriptor,
                ))
            } else {
                apply_transition_timeline_descriptor_without_metrics(handles, timeline_descriptor);
                None
            }
        });
        let apply_ms = apply_started_at.map_or(0, |started_at| started_at.elapsed().as_millis());
        let _ = done_tx.send(metrics.map(|timeline| NativePanelAnimationFrameMetrics {
            queued_ms,
            apply_ms,
            continue_animating,
            timeline,
        }));
    });
    if scheduled.is_ok() {
        done_rx.await.ok().flatten()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::native_panel_animation_target;
    use crate::native_panel_renderer::facade::{
        renderer::NativePanelAnimationFrameScheduler, transition::NativePanelTransitionRequest,
    };

    #[test]
    fn macos_transition_runner_uses_shared_animation_scheduler_target() {
        let target = native_panel_animation_target(
            NativePanelTransitionRequest::SurfaceSwitch,
            164.0,
            196.0,
            3,
        );
        let mut scheduler = NativePanelAnimationFrameScheduler::default();
        let frame = scheduler.start(target, std::time::Instant::now());

        assert_eq!(
            frame.descriptor.animation.kind,
            crate::native_panel_core::PanelAnimationKind::SurfaceSwitch
        );
        assert_eq!(frame.plan.card_stack.card_count, 3);
        assert_eq!(frame.descriptor.animation.visible_height, 164.0);
    }
}
