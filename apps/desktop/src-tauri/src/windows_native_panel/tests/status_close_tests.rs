use super::*;

#[test]
fn windows_animation_scheduler_starts_open_transition_and_queues_redraw() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime
        .sync_snapshot_bundle(
            &pending_permission_snapshot("session-1"),
            &runtime_input_descriptor(),
        )
        .expect("sync transition snapshot");
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );
    let _ = runtime.host.presenter.take_redraw_frame();

    let frame = runtime
        .advance_animation_frame_at(now)
        .expect("advance animation")
        .expect("opening frame");

    assert_eq!(frame.descriptor.animation.kind, PanelAnimationKind::Open);
    assert!(frame.continue_animating);
    assert!(runtime.panel_state.transitioning);
    assert!(runtime.animation_scheduler.is_active());
    assert!(runtime.host.presenter.redraw_requested());
}

#[test]
fn windows_animation_scheduler_finishes_close_transition_without_redraw_loop() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 180.0,
            visible_height: 180.0,
            width_progress: 1.0,
            height_progress: 1.0,
            shoulder_progress: 1.0,
            drop_progress: 1.0,
            cards_progress: 1.0,
        })
        .expect("seed expanded descriptor");
    runtime.last_transition_request = Some(NativePanelTransitionRequest::Close);
    let first = runtime
        .advance_animation_frame_at(now)
        .expect("advance close")
        .expect("closing frame");
    assert_eq!(first.descriptor.animation.kind, PanelAnimationKind::Close);
    assert!(first.continue_animating);
    assert!(runtime.animation_scheduler.is_active());
    let _ = runtime.host.presenter.take_redraw_frame();

    let mut final_frame = None;
    for step in 1..=first
        .total_ms
        .div_ceil(crate::native_panel_core::PANEL_ANIMATION_FRAME_MS)
        + 1
    {
        let frame = runtime
            .advance_animation_frame_at(
                now + Duration::from_millis(
                    first.total_ms + step * crate::native_panel_core::PANEL_ANIMATION_FRAME_MS,
                ),
            )
            .expect("advance close frame")
            .expect("closing frame");
        if !frame.continue_animating {
            final_frame = Some(frame);
            break;
        }
    }
    let final_frame = final_frame.expect("terminal frame");
    assert!(!final_frame.continue_animating);
    assert!(!runtime.animation_scheduler.is_active());
    assert!(!runtime.panel_state.transitioning);
    let _ = runtime.host.presenter.take_redraw_frame();

    let idle = runtime
        .advance_animation_frame_at(now + Duration::from_millis(first.total_ms + 32))
        .expect("idle advance");
    assert!(idle.is_none());
    assert!(!runtime.host.presenter.redraw_requested());
}

#[test]
fn windows_runtime_refreshes_active_count_marquee_paint_job_without_scene_sync() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let now = Instant::now();
    let mut frame = shell_draw_frame(Vec::new(), true);
    frame
        .presentation_model
        .as_mut()
        .expect("presentation model")
        .compact_bar
        .active_count = "23".to_string();
    runtime.host.presenter.present(frame);
    runtime.host.consume_presenter_into_shell_result();
    runtime.active_count_marquee_started_at =
        Some(now - Duration::from_millis(ACTIVE_COUNT_SCROLL_HOLD_MS as u64 + 10));

    assert!(runtime.refresh_active_count_marquee_frame_at(now));

    let paint_job = runtime
        .host
        .shell
        .pending_paint_job()
        .expect("marquee paint job");
    assert_eq!(paint_job.active_count, "23");
    assert!(paint_job.active_count_elapsed_ms >= ACTIVE_COUNT_SCROLL_HOLD_MS);
    assert!(runtime.host.shell.redraw_requests() > 0);
}

#[test]
fn windows_runtime_refreshes_mascot_animation_paint_job_without_scene_sync() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let now = Instant::now();
    let mut frame = shell_draw_frame(Vec::new(), true);
    frame
        .presentation_model
        .as_mut()
        .expect("presentation model")
        .mascot
        .pose = SceneMascotPose::Idle;
    runtime.host.presenter.present(frame);
    runtime.host.consume_presenter_into_shell_result();
    runtime.mascot_animation_started_at =
        Some(now - Duration::from_millis(crate::native_panel_core::MASCOT_ANIMATION_REFRESH_MS));

    assert!(runtime.refresh_mascot_animation_frame_at(now));

    let paint_job = runtime
        .host
        .shell
        .pending_paint_job()
        .expect("mascot paint job");
    assert_eq!(paint_job.mascot_pose, SceneMascotPose::Idle);
    assert!(paint_job.mascot_elapsed_ms > 0);
    assert!(runtime.host.shell.redraw_requests() > 0);
}

#[test]
fn windows_runtime_starts_mascot_animation_with_immediate_paint_job() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let now = Instant::now();
    let mut frame = shell_draw_frame(Vec::new(), true);
    frame
        .presentation_model
        .as_mut()
        .expect("presentation model")
        .mascot
        .pose = SceneMascotPose::Idle;
    runtime.host.presenter.present(frame);
    runtime.host.consume_presenter_into_shell_result();
    let _ = runtime.host.shell.paint_next_frame();
    assert!(runtime.mascot_animation_started_at.is_none());

    assert!(runtime.refresh_mascot_animation_frame_at(now));

    let paint_job = runtime
        .host
        .shell
        .pending_paint_job()
        .expect("initial mascot paint job");
    assert_eq!(paint_job.mascot_pose, SceneMascotPose::Idle);
    assert_eq!(paint_job.mascot_elapsed_ms, 0);
    assert_eq!(runtime.mascot_animation_started_at, Some(now));
    assert!(runtime.host.shell.redraw_requests() > 0);
}

#[test]
fn windows_runtime_defers_mascot_animation_refresh_during_panel_transition() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let now = Instant::now();
    let mut frame = shell_draw_frame(Vec::new(), true);
    let presentation = frame
        .presentation_model
        .as_mut()
        .expect("presentation model");
    presentation.compact_bar.active_count = "23".to_string();
    presentation.mascot.pose = SceneMascotPose::Idle;
    runtime.host.presenter.present(frame);
    runtime.host.consume_presenter_into_shell_result();
    let _ = runtime.host.shell.paint_next_frame();
    runtime.panel_state.transitioning = true;
    runtime.active_count_marquee_started_at =
        Some(now - Duration::from_millis(ACTIVE_COUNT_SCROLL_HOLD_MS as u64 + 10));
    runtime.mascot_animation_started_at =
        Some(now - Duration::from_millis(crate::native_panel_core::MASCOT_ANIMATION_REFRESH_MS));

    assert!(!runtime.refresh_active_count_marquee_frame_at(now));
    assert!(!runtime.refresh_mascot_animation_frame_at(now));
    assert!(runtime.active_count_marquee_started_at.is_none());
    assert!(runtime.mascot_animation_started_at.is_none());

    runtime.panel_state.transitioning = false;
    runtime.mascot_animation_started_at =
        Some(now - Duration::from_millis(crate::native_panel_core::MASCOT_ANIMATION_REFRESH_MS));

    assert!(runtime.refresh_mascot_animation_frame_at(now));
    let paint_job = runtime
        .host
        .shell
        .pending_paint_job()
        .expect("mascot transition paint job");
    assert_eq!(paint_job.mascot_pose, SceneMascotPose::Idle);
    assert!(paint_job.mascot_elapsed_ms > 0);
    assert!(runtime.mascot_animation_started_at.is_some());
}

#[test]
fn windows_animation_scheduler_preserves_status_cards_when_auto_status_close_skips_default_rebuild()
{
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime
        .sync_snapshot_bundle(&pending_permission_snapshot("session-1"), &input)
        .expect("seed auto status surface");
    runtime
        .host
        .present_renderer_state()
        .expect("present status surface");
    let presented_status_cards = runtime
        .host
        .window
        .presented_presentation_model
        .as_ref()
        .expect("presented status model")
        .card_stack
        .cards
        .clone();
    assert!(
        presented_status_cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusApproval { .. }))
    );
    runtime.panel_state.status_queue[0].expires_at = Instant::now() - Duration::from_millis(1);
    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("refresh expired status queue");
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Close)
    );
    assert!(runtime.panel_state.skip_next_close_card_exit);
    let pre_close_presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("pre-close presentation");
    assert!(!pre_close_presentation.compact_bar.actions_visible);
    assert!(!pre_close_presentation.action_buttons.visible);
    assert!(
        pre_close_presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusApproval { .. }))
    );
    let pre_close_frame = runtime
        .host
        .take_pending_draw_frame()
        .expect("pre-close draw frame");
    let pre_close_frame_presentation = pre_close_frame
        .presentation_model
        .as_ref()
        .expect("pre-close frame presentation");
    assert!(!pre_close_frame_presentation.compact_bar.actions_visible);
    assert!(!pre_close_frame_presentation.action_buttons.visible);
    assert!(
        pre_close_frame_presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusApproval { .. }))
    );
    runtime.host.presenter.present(pre_close_frame);
    runtime
        .pump_platform_loop()
        .expect("pump pre-close frame into shell");
    let shell_job = runtime
        .host
        .shell
        .pending_paint_job()
        .expect("pre-close shell paint job");
    assert!(!shell_job.action_buttons_visible);
    assert_eq!(
        runtime
            .last_animation_descriptor
            .expect("pump started close animation")
            .kind,
        PanelAnimationKind::Close
    );
    assert!(!runtime.panel_state.skip_next_close_card_exit);
    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("suppressed close presentation");
    assert!(!presentation.compact_bar.actions_visible);
    assert!(!presentation.action_buttons.visible);
    assert!(presentation.card_stack.visible);
    assert_eq!(
        presentation.card_stack.cards.len(),
        presented_status_cards.len()
    );
    assert!(
        presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusApproval { .. })),
        "unexpected close cards: {:?}",
        presentation.card_stack.cards
    );
    assert!(
        !presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::Empty))
    );

    let next_frame = runtime
        .advance_animation_frame_at(
            Instant::now()
                + Duration::from_millis(crate::native_panel_core::PANEL_ANIMATION_FRAME_MS),
        )
        .expect("advance next close frame")
        .expect("next closing frame");
    assert_eq!(
        next_frame.descriptor.animation.kind,
        PanelAnimationKind::Close
    );
    let next_presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("next close presentation");
    assert!(!next_presentation.compact_bar.actions_visible);
    assert!(!next_presentation.action_buttons.visible);
}

#[test]
fn windows_default_hover_close_transition_keeps_edge_actions_for_retract_animation() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime.panel_state.expanded = true;
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("seed expanded default surface");
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 180.0,
            visible_height: 180.0,
            width_progress: 1.0,
            height_progress: 1.0,
            shoulder_progress: 1.0,
            drop_progress: 1.0,
            cards_progress: 1.0,
        })
        .expect("seed fully expanded descriptor");
    let expanded_presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("expanded default presentation");
    assert!(expanded_presentation.compact_bar.actions_visible);
    assert!(expanded_presentation.action_buttons.visible);

    runtime.last_transition_request = Some(NativePanelTransitionRequest::Close);
    runtime
        .advance_animation_frame_at(now)
        .expect("advance close")
        .expect("close frame");

    let closing_presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("closing presentation");
    assert_eq!(closing_presentation.shell.surface, ExpandedSurface::Default);
    assert_eq!(
        closing_presentation.compact_bar.headline.text,
        "No active tasks"
    );
    assert!(closing_presentation.compact_bar.actions_visible);
    assert!(closing_presentation.action_buttons.visible);
    assert!(
        runtime
            .host
            .renderer
            .last_pointer_regions
            .iter()
            .any(|region| matches!(region.kind, NativePanelPointerRegionKind::EdgeAction(_)))
    );
}

#[test]
fn windows_default_close_delayed_wake_keeps_cards_and_actions_during_catch_up() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime.panel_state.expanded = true;
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("seed expanded default surface");
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 180.0,
            visible_height: 180.0,
            width_progress: 1.0,
            height_progress: 1.0,
            shoulder_progress: 1.0,
            drop_progress: 1.0,
            cards_progress: 1.0,
        })
        .expect("seed fully expanded descriptor");
    runtime.last_transition_request = Some(NativePanelTransitionRequest::Close);

    let first_frame = runtime
        .advance_animation_frame_at(now)
        .expect("start close")
        .expect("first close frame");
    let delayed_frame = runtime
        .advance_animation_frame_at(now + Duration::from_millis(first_frame.total_ms + 250))
        .expect("delayed close")
        .expect("delayed close catch-up frame");

    assert!(delayed_frame.continue_animating);
    assert!(delayed_frame.descriptor.animation.cards_progress < 1.0);
    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("catch-up presentation");
    assert_eq!(presentation.shell.surface, ExpandedSurface::Default);
    assert!(presentation.card_stack.visible);
    assert!(!presentation.card_stack.cards.is_empty());
    assert!(presentation.compact_bar.actions_visible);
    assert!(presentation.action_buttons.visible);
}

#[test]
fn windows_repeated_hover_cycles_keep_transition_stages_stable() {
    fn finish_animation(
        runtime: &mut super::WindowsNativePanelRuntime,
        start: Instant,
        total_ms: u64,
    ) {
        for step in 1..=total_ms.div_ceil(crate::native_panel_core::PANEL_ANIMATION_FRAME_MS) + 2 {
            let Some(frame) = runtime
                .advance_animation_frame_at(
                    start
                        + Duration::from_millis(
                            step * crate::native_panel_core::PANEL_ANIMATION_FRAME_MS,
                        ),
                )
                .expect("advance animation")
            else {
                break;
            };
            if !frame.continue_animating {
                break;
            }
        }
    }

    let mut now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("seed default snapshot");

    for cycle in 0..5 {
        runtime.panel_state.pointer_inside_since =
            Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 1));
        assert_eq!(
            runtime
                .sync_hover_and_refresh_inside_with_input(true, now, &input)
                .expect("hover open"),
            Some(HoverTransition::Expand),
            "open transition missing at cycle {cycle}"
        );
        let open = runtime
            .advance_animation_frame_at(now)
            .expect("start open")
            .expect("open frame");
        let delayed_open = runtime
            .advance_animation_frame_at(now + Duration::from_millis(open.total_ms + 250))
            .expect("delayed open")
            .expect("open catch-up");
        assert!(
            delayed_open.continue_animating,
            "open ended early at cycle {cycle}"
        );
        assert!(
            delayed_open.descriptor.animation.width_progress < 0.7,
            "open skipped width stage at cycle {cycle}"
        );
        finish_animation(&mut runtime, now, open.total_ms);

        now += Duration::from_millis(open.total_ms + 240);
        runtime.panel_state.pointer_outside_since =
            Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 1));
        assert_eq!(
            runtime
                .sync_hover_and_refresh_inside_with_input(false, now, &input)
                .expect("hover close"),
            Some(HoverTransition::Collapse),
            "close transition missing at cycle {cycle}"
        );
        let close = runtime
            .advance_animation_frame_at(now)
            .expect("start close")
            .expect("close frame");
        let delayed_close = runtime
            .advance_animation_frame_at(now + Duration::from_millis(close.total_ms + 250))
            .expect("delayed close")
            .expect("close catch-up");
        assert!(
            delayed_close.continue_animating,
            "close ended early at cycle {cycle}"
        );
        assert!(
            delayed_close.descriptor.animation.cards_progress < 0.5,
            "close skipped card stage at cycle {cycle}"
        );
        let presentation = runtime
            .host
            .renderer
            .latest_scene_presentation_model()
            .expect("close presentation");
        assert!(
            presentation.card_stack.visible,
            "cards hidden during close at cycle {cycle}"
        );
        assert!(
            presentation.action_buttons.visible,
            "actions hidden during close at cycle {cycle}"
        );
        finish_animation(&mut runtime, now, close.total_ms);
        now += Duration::from_millis(close.total_ms + 240);
    }
}

#[test]
fn windows_runtime_replaces_stale_action_button_paint_before_status_close() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");

    runtime
        .sync_snapshot_bundle(&pending_permission_snapshot("session-1"), &input)
        .expect("seed auto status surface");
    runtime
        .host
        .present_renderer_state()
        .expect("present status surface");
    let mut stale_frame = runtime
        .host
        .take_pending_draw_frame()
        .expect("status draw frame");
    let stale_presentation = stale_frame
        .presentation_model
        .as_mut()
        .expect("stale frame presentation");
    stale_presentation.compact_bar.actions_visible = true;
    stale_presentation.action_buttons.visible = true;
    runtime.host.presenter.present(stale_frame);
    runtime.host.consume_presenter_into_shell_result();
    assert!(
        runtime
            .host
            .shell
            .pending_paint_job()
            .is_some_and(|job| job.action_buttons_visible)
    );

    super::clear_windows_native_panel_window_messages(Some(hwnd));
    super::queue_windows_native_panel_window_message(hwnd, super::WINDOWS_WM_PAINT, 0);
    runtime.panel_state.status_queue[0].expires_at = Instant::now() - Duration::from_millis(1);

    runtime
        .pump_platform_loop()
        .expect("pump expired status close");

    assert_eq!(
        runtime.platform_loop.last_window_message_id,
        Some(super::WINDOWS_WM_PAINT)
    );
    assert!(
        runtime
            .platform_loop
            .last_painted_job
            .as_ref()
            .is_some_and(|job| !job.action_buttons_visible)
    );
    assert_eq!(
        runtime
            .last_animation_descriptor
            .expect("status close animation")
            .kind,
        PanelAnimationKind::Close
    );
}

#[test]
fn windows_status_queue_refresh_during_close_keeps_preserved_status_cards() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime
        .sync_snapshot_bundle(&pending_permission_snapshot("session-1"), &input)
        .expect("seed auto status surface");
    runtime
        .host
        .present_renderer_state()
        .expect("present status surface");
    let presented_status_cards = runtime
        .host
        .window
        .presented_presentation_model
        .as_ref()
        .expect("presented status model")
        .card_stack
        .cards
        .clone();

    runtime.panel_state.status_queue[0].expires_at = Instant::now() - Duration::from_millis(1);
    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("refresh expired status queue");
    runtime
        .advance_animation_frame_at(now)
        .expect("advance auto status close")
        .expect("closing frame");
    runtime
        .host
        .present_renderer_state()
        .expect("present closing frame");

    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("refresh while close is active");

    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("closing presentation after refresh");
    assert!(!presentation.compact_bar.actions_visible);
    assert!(!presentation.action_buttons.visible);
    assert_eq!(
        presentation.card_stack.cards.len(),
        presented_status_cards.len()
    );
    assert!(
        presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusApproval { .. }))
    );
    assert!(
        !presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::Empty))
    );
}

#[test]
fn windows_status_queue_new_request_during_close_reopens_status_after_close() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime
        .sync_snapshot_bundle(
            &pending_permission_snapshot_with_request("req-1", "session-1"),
            &input,
        )
        .expect("seed first status surface");
    runtime
        .host
        .present_renderer_state()
        .expect("present first status surface");

    runtime.panel_state.status_queue[0].expires_at = Instant::now() - Duration::from_millis(1);
    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("expire first status item");
    let close_frame = runtime
        .advance_animation_frame_at(now)
        .expect("advance status close")
        .expect("closing frame");
    runtime
        .host
        .present_renderer_state()
        .expect("present closing status frame");

    runtime
        .sync_snapshot_bundle(
            &pending_question_snapshot_with_request("question-2", "session-2"),
            &input,
        )
        .expect("sync new status while close is active");

    let closing_presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("closing presentation after new status");
    assert!(
        closing_presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusApproval { .. })),
        "active close should keep the old status card stack"
    );
    assert!(
        !closing_presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusQuestion { .. })),
        "new status should wait until the close finishes"
    );

    let mut finished_close = false;
    for step in 1..=close_frame
        .total_ms
        .div_ceil(crate::native_panel_core::PANEL_ANIMATION_FRAME_MS)
        + 1
    {
        let frame = runtime
            .advance_animation_frame_at(
                now + Duration::from_millis(
                    close_frame.total_ms
                        + step * crate::native_panel_core::PANEL_ANIMATION_FRAME_MS,
                ),
            )
            .expect("finish close frame")
            .expect("closing frame");
        if !frame.continue_animating {
            finished_close = true;
            break;
        }
    }
    assert!(finished_close);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open),
        "new status should schedule a reopen after close finishes"
    );

    let open_frame = runtime
        .advance_animation_frame_at(now + Duration::from_millis(close_frame.total_ms + 17))
        .expect("start reopen")
        .expect("opening new status frame");
    assert_eq!(
        open_frame.descriptor.animation.kind,
        PanelAnimationKind::Open
    );
    let reopened_presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("reopened status presentation");
    assert!(
        reopened_presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusQuestion { .. }))
    );
}

#[test]
fn windows_completion_status_close_keeps_compact_mascot_until_cards_exit() {
    let now = Instant::now();
    let input = runtime_input_descriptor();
    let mut running_snapshot = sessions_snapshot(1);
    running_snapshot.sessions[0].status = "Running".to_string();
    running_snapshot.sessions[0].last_assistant_message = Some("Working".to_string());
    let mut completed_snapshot = running_snapshot.clone();
    completed_snapshot.sessions[0].status = "Idle".to_string();
    completed_snapshot.sessions[0].last_activity = Utc::now();
    completed_snapshot.sessions[0].last_assistant_message = Some("Done".to_string());
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime
        .sync_snapshot_bundle(&running_snapshot, &input)
        .expect("seed running snapshot");
    runtime
        .sync_snapshot_bundle(&completed_snapshot, &input)
        .expect("show completion status");
    runtime
        .host
        .present_renderer_state()
        .expect("present completion status");
    let completion_presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("completion status presentation");
    assert_eq!(
        completion_presentation.mascot.pose,
        SceneMascotPose::Complete
    );
    assert!(!completion_presentation.compact_bar.actions_visible);
    assert!(!completion_presentation.action_buttons.visible);

    runtime.panel_state.status_queue[0].expires_at = Instant::now() - Duration::from_millis(1);
    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("refresh expired status queue");
    runtime
        .advance_animation_frame_at(now)
        .expect("advance auto status close")
        .expect("closing frame");
    runtime
        .host
        .present_renderer_state()
        .expect("present closing frame");
    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("refresh while completion close is active");

    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("closing completion presentation");
    assert_eq!(presentation.mascot.pose, SceneMascotPose::Complete);
    assert!(!presentation.compact_bar.actions_visible);
    assert!(!presentation.action_buttons.visible);
    assert!(
        presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusCompletion { .. }))
    );
}

#[test]
fn windows_status_close_preservation_keeps_shell_surface_in_sync() {
    let now = Instant::now();
    let input = runtime_input_descriptor();
    let mut running_snapshot = sessions_snapshot(1);
    running_snapshot.sessions[0].status = "Running".to_string();
    running_snapshot.sessions[0].last_assistant_message = Some("Working".to_string());
    let mut completed_snapshot = running_snapshot.clone();
    completed_snapshot.sessions[0].status = "Idle".to_string();
    completed_snapshot.sessions[0].last_activity = Utc::now();
    completed_snapshot.sessions[0].last_assistant_message = Some("Done".to_string());
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime
        .sync_snapshot_bundle(&running_snapshot, &input)
        .expect("seed running snapshot");
    runtime
        .sync_snapshot_bundle(&completed_snapshot, &input)
        .expect("show completion status");
    runtime
        .host
        .present_renderer_state()
        .expect("present completion status");

    runtime.panel_state.status_queue[0].expires_at = Instant::now() - Duration::from_millis(1);
    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("refresh expired status queue");
    runtime
        .advance_animation_frame_at(now)
        .expect("advance auto status close")
        .expect("closing frame");
    runtime
        .host
        .present_renderer_state()
        .expect("present closing frame");
    runtime
        .refresh_status_queue_from_last_raw_snapshot_with_input(&input)
        .expect("refresh while status close is active");

    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("closing status presentation");
    assert_eq!(presentation.shell.surface, ExpandedSurface::Status);
    assert_eq!(presentation.card_stack.surface, ExpandedSurface::Status);
}

#[test]
fn windows_preserved_status_stack_repairs_stale_default_shell_surface() {
    let input = runtime_input_descriptor();
    let mut running_snapshot = sessions_snapshot(1);
    running_snapshot.sessions[0].status = "Running".to_string();
    running_snapshot.sessions[0].last_assistant_message = Some("Working".to_string());
    let mut completed_snapshot = running_snapshot.clone();
    completed_snapshot.sessions[0].status = "Idle".to_string();
    completed_snapshot.sessions[0].last_activity = Utc::now();
    completed_snapshot.sessions[0].last_assistant_message = Some("Done".to_string());
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime
        .sync_snapshot_bundle(&running_snapshot, &input)
        .expect("seed running snapshot");
    runtime
        .sync_snapshot_bundle(&completed_snapshot, &input)
        .expect("show completion status");
    runtime
        .host
        .present_renderer_state()
        .expect("present completion status");

    let preserved = runtime
        .host
        .renderer
        .last_presentation_model
        .as_ref()
        .expect("status presentation")
        .clone();
    runtime
        .host
        .renderer
        .last_presentation_model
        .as_mut()
        .expect("presentation slot")
        .shell
        .surface = ExpandedSurface::Default;
    runtime
        .host
        .renderer
        .scene_cache
        .last_render_command_bundle
        .as_mut()
        .expect("render bundle")
        .shell
        .surface = ExpandedSurface::Default;
    runtime
        .host
        .renderer
        .last_presentation_model
        .as_mut()
        .expect("presentation slot")
        .mascot
        .pose = SceneMascotPose::Idle;
    runtime
        .host
        .renderer
        .scene_cache
        .last_render_command_bundle
        .as_mut()
        .expect("render bundle")
        .mascot
        .pose = SceneMascotPose::Idle;

    runtime.host.renderer.apply_close_presentation_plan(
        Some(&preserved),
        resolve_native_panel_close_presentation_plan(NativePanelClosePresentationInput {
            trigger: NativePanelCloseTrigger::StatusAuto,
            status_close: resolve_native_panel_status_close_preservation_plan(
                NativePanelStatusClosePreservationInput {
                    last_transition_request: Some(NativePanelTransitionRequest::Close),
                    skip_next_close_card_exit: true,
                    transitioning: false,
                    last_animation: None,
                },
            ),
            has_preserved_cards: true,
        }),
    );

    let presentation = runtime
        .host
        .renderer
        .current_presentation_model()
        .expect("preserved presentation");
    assert_eq!(presentation.shell.surface, ExpandedSurface::Status);
    assert_eq!(presentation.card_stack.surface, ExpandedSurface::Status);
    assert_eq!(presentation.mascot.pose, SceneMascotPose::Complete);

    runtime
        .host
        .present_renderer_state()
        .expect("present preserved state");
    runtime.host.consume_presenter_into_shell_result();
    let paint_job = runtime
        .host
        .shell
        .pending_paint_job()
        .expect("preserved paint job");
    assert_eq!(paint_job.surface, ExpandedSurface::Status);
    assert_eq!(paint_job.mascot_pose, SceneMascotPose::Complete);
}

#[test]
fn windows_animation_scheduler_runs_surface_switch_only_while_active() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 180.0,
            visible_height: 180.0,
            width_progress: 1.0,
            height_progress: 1.0,
            shoulder_progress: 1.0,
            drop_progress: 1.0,
            cards_progress: 1.0,
        })
        .expect("seed expanded descriptor");
    runtime.last_transition_request = Some(NativePanelTransitionRequest::SurfaceSwitch);

    let frame = runtime
        .advance_animation_frame_at(now)
        .expect("advance surface switch")
        .expect("surface switch frame");

    assert_eq!(
        frame.descriptor.animation.kind,
        PanelAnimationKind::SurfaceSwitch
    );
    assert!(frame.continue_animating);
    assert!(runtime.animation_scheduler.is_active());
}

#[test]
fn windows_animation_scheduler_defers_surface_switch_until_active_animation_finishes() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime
        .sync_snapshot_bundle(
            &pending_permission_snapshot("session-1"),
            &runtime_input_descriptor(),
        )
        .expect("sync transition snapshot");

    let opening = runtime
        .advance_animation_frame_at(now)
        .expect("start open")
        .expect("opening frame");
    assert_eq!(opening.descriptor.animation.kind, PanelAnimationKind::Open);
    assert!(runtime.animation_scheduler.is_active());

    runtime.last_transition_request = Some(NativePanelTransitionRequest::SurfaceSwitch);
    let deferred = runtime
        .advance_animation_frame_at(now + Duration::from_millis(16))
        .expect("sample active open")
        .expect("deferred open frame");

    assert_eq!(deferred.descriptor.animation.kind, PanelAnimationKind::Open);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::SurfaceSwitch)
    );

    let mut completed_open = None;
    let mut completed_at = now;
    for step in 2..=opening
        .total_ms
        .div_ceil(crate::native_panel_core::PANEL_ANIMATION_FRAME_MS)
        + 8
    {
        let frame_at =
            now + Duration::from_millis(step * crate::native_panel_core::PANEL_ANIMATION_FRAME_MS);
        let frame = runtime
            .advance_animation_frame_at(frame_at)
            .expect("advance deferred open")
            .expect("open frame while surface switch is deferred");
        assert_eq!(frame.descriptor.animation.kind, PanelAnimationKind::Open);
        assert_eq!(
            runtime.last_transition_request,
            Some(NativePanelTransitionRequest::SurfaceSwitch)
        );
        if !frame.continue_animating {
            completed_at = frame_at;
            completed_open = Some(frame);
            break;
        }
    }
    let completed_open = completed_open.expect("completed open frame");
    assert_eq!(
        completed_open.descriptor.animation.kind,
        PanelAnimationKind::Open
    );
    assert!(!completed_open.continue_animating);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::SurfaceSwitch)
    );

    let surface_switch = runtime
        .advance_animation_frame_at(
            completed_at
                + Duration::from_millis(crate::native_panel_core::PANEL_ANIMATION_FRAME_MS),
        )
        .expect("start deferred surface switch")
        .expect("surface switch frame");

    assert_eq!(
        surface_switch.descriptor.animation.kind,
        PanelAnimationKind::SurfaceSwitch
    );
    assert!(surface_switch.continue_animating);
}

#[test]
fn windows_animation_scheduler_idle_state_does_not_redraw_continuously() {
    let mut runtime = super::WindowsNativePanelRuntime::default();

    let frame = runtime
        .advance_animation_frame_at(Instant::now())
        .expect("advance idle");

    assert!(frame.is_none());
    assert!(!runtime.animation_scheduler.is_active());
    assert!(!runtime.host.presenter.redraw_requested());
}

#[test]
fn windows_renderer_treats_close_card_progress_as_exit_progress() {
    let mut state = PanelState::default();
    let bundle = test_runtime_scene_bundle(
        &mut state,
        &pending_permission_snapshot("session-1"),
        &PanelSceneBuildInput::default(),
    );
    let mut renderer = super::WindowsNativePanelRenderer::default();

    renderer
        .render_scene(&bundle.scene, bundle.runtime_render_state)
        .expect("render scene");
    renderer
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Close,
            canvas_height: 180.0,
            visible_height: 180.0,
            width_progress: 1.0,
            height_progress: 1.0,
            shoulder_progress: 1.0,
            drop_progress: 1.0,
            cards_progress: 0.0,
        })
        .expect("apply close descriptor");

    assert_eq!(
        renderer.last_layout.expect("layout").separator_visibility,
        0.88
    );
}
