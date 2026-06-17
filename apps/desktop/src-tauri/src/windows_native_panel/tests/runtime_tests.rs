use super::*;

#[test]
fn windows_runtime_and_host_satisfy_shared_native_traits() {
    fn assert_runtime<T>()
    where
        T: NativePanelClickStateBridge
            + NativePanelCoreStateBridge
            + NativePanelHostInteractionStateBridge
            + NativePanelHostShellRuntimePump
            + NativePanelPlatformWindowMessagePump
            + NativePanelPointerInputRuntimeBridge
            + NativePanelPrimaryPointerStateBridge
            + NativePanelRuntimeSceneMutableStateBridge
            + NativePanelRuntimeSceneStateBridge
            + NativePanelSceneRuntimeBridge,
    {
    }

    fn assert_host<H>()
    where
        H: NativePanelHost
            + NativePanelRuntimeHostController
            + NativePanelSceneHost
            + NativePanelQueuedPlatformEventBridge,
    {
    }

    assert_runtime::<super::WindowsNativePanelRuntime>();
    assert_host::<super::WindowsNativePanelHost>();
}

#[test]
fn windows_native_default_enable_preflight_uses_shared_runtime_pipeline() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    runtime.create_panel().expect("create native panel");
    assert!(runtime.host.window.descriptor.visible);
    assert_eq!(
        runtime.host.shell.lifecycle(),
        NativePanelHostShellLifecycle::Visible
    );
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
        .expect("seed animation descriptor");
    runtime
        .sync_snapshot_bundle(&pending_permission_snapshot("session-1"), &input)
        .expect("sync snapshot through shared runtime");

    let scene = runtime
        .scene_cache
        .last_scene
        .as_ref()
        .expect("shared scene cached");
    assert!(!scene.cards.is_empty());
    assert!(runtime.host.renderer.last_layout.is_some());
    assert!(runtime.host.renderer.last_render_state.is_some());
    assert!(!runtime.host.renderer.last_pointer_regions.is_empty());

    let first_region = runtime.host.renderer.last_pointer_regions[0].clone();
    let point = PanelPoint {
        x: first_region.frame.x + first_region.frame.width / 2.0,
        y: first_region.frame.y + first_region.frame.height / 2.0,
    };
    let mut handler = RecordingEventHandler::default();
    let outcome = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Move(point),
            Instant::now(),
            &input,
            &mut handler,
        )
        .expect("route pointer input through shared helper");
    assert!(matches!(outcome, NativePanelPointerInputOutcome::Hover(_)));

    runtime
        .toggle_settings_surface_with_input(&input)
        .expect("toggle settings through shared runtime");
    assert!(runtime.host.renderer.last_layout.is_some());

    runtime
        .set_shared_expanded_body_height(180.0)
        .expect("route shared body height through host facade");
    assert_eq!(
        runtime.host.window.descriptor.shared_body_height,
        Some(180.0)
    );

    runtime
        .pump_platform_loop()
        .expect("pump shared shell commands");
    assert!(runtime.platform_loop.applied_command_count > 0);
}

#[test]
fn windows_runtime_first_snapshot_renders_without_seeded_animation_descriptor() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    runtime.create_panel().expect("create native panel");
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("sync first snapshot");

    assert!(runtime.host.renderer.last_animation_descriptor.is_some());
    assert!(runtime.host.renderer.last_layout.is_some());
    assert!(runtime.host.renderer.last_render_state.is_some());
    assert!(runtime.host.renderer.last_window_state.is_some());
    assert!(
        runtime
            .host
            .renderer
            .last_window_state
            .is_some_and(|state| state
                .frame
                .is_some_and(|frame| { frame.width > 1.0 && frame.height > 1.0 && state.visible }))
    );
}

#[test]
fn windows_runtime_rerender_hides_mascot_when_setting_is_disabled() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("sync first snapshot");
    runtime.pump_platform_loop().expect("pump first snapshot");

    let mut hidden_input = runtime_input_descriptor();
    hidden_input.scene_input.settings.mascot_enabled = false;
    let rerendered = runtime
        .rerender_from_last_snapshot_with_input(&hidden_input)
        .expect("rerender after mascot setting change");
    runtime.pump_platform_loop().expect("pump hidden rerender");

    assert!(rerendered);
    assert_eq!(
        runtime
            .host
            .renderer
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.mascot_pose),
        Some(SceneMascotPose::Hidden)
    );
    assert_eq!(
        runtime
            .host
            .shell
            .display_snapshot()
            .map(|display| display.visual_input.mascot_pose),
        Some(SceneMascotPose::Hidden)
    );
}

#[test]
fn windows_runtime_snapshot_sync_exposes_shared_message_sound_reminder() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    let sync = runtime
        .sync_snapshot_bundle(&pending_permission_snapshot("session-1"), &input)
        .expect("sync snapshot through shared runtime")
        .expect("snapshot sync result");

    assert!(sync.reminder.play_sound);
}

#[test]
fn windows_runtime_emits_generic_completion_reminder_for_active_to_idle_without_message() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    let mut running_snapshot = sessions_snapshot(1);
    running_snapshot.sessions[0].status = "Running".to_string();
    running_snapshot.sessions[0].last_assistant_message = Some("Working".to_string());
    let mut completed_snapshot = running_snapshot.clone();
    completed_snapshot.active_session_count = 0;
    completed_snapshot.status = "Idle".to_string();
    completed_snapshot.sessions[0].status = "Idle".to_string();
    completed_snapshot.sessions[0].last_activity = Utc::now();
    completed_snapshot.sessions[0].last_assistant_message = None;
    runtime
        .sync_snapshot_bundle(&running_snapshot, &input)
        .expect("sync running snapshot");

    let sync = runtime
        .sync_snapshot_bundle(&completed_snapshot, &input)
        .expect("sync completion without assistant message")
        .expect("snapshot sync result");

    assert!(sync.reminder.play_sound);
    assert_eq!(sync.panel_transition, Some(true));
    assert_eq!(runtime.panel_state.completion_badge_items.len(), 1);
    assert!(runtime.panel_state.status_queue.iter().any(|item| matches!(
        item.payload,
        crate::native_panel_core::StatusQueuePayload::Completion(_)
    )));
}

#[test]
fn windows_runtime_preserves_pending_open_request_across_duplicate_completion_refresh() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    let mut running_snapshot = sessions_snapshot(1);
    running_snapshot.sessions[0].status = "Running".to_string();
    running_snapshot.sessions[0].last_assistant_message = Some("Working".to_string());
    let mut completed_snapshot = running_snapshot.clone();
    completed_snapshot.active_session_count = 0;
    completed_snapshot.status = "Idle".to_string();
    completed_snapshot.sessions[0].status = "Idle".to_string();
    completed_snapshot.sessions[0].last_activity = Utc::now();
    completed_snapshot.sessions[0].last_assistant_message = Some("Done".to_string());
    runtime
        .sync_snapshot_bundle(&running_snapshot, &input)
        .expect("seed running snapshot");

    runtime
        .sync_snapshot_bundle(&completed_snapshot, &input)
        .expect("sync completion snapshot")
        .expect("completion sync result");
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );

    runtime
        .sync_snapshot_bundle(&completed_snapshot, &input)
        .expect("sync duplicate completion refresh")
        .expect("duplicate sync result");

    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );
    assert!(
        runtime
            .advance_animation_frame_at(Instant::now())
            .expect("advance pending open")
            .is_some()
    );
}

#[test]
fn windows_runtime_reopens_completion_when_message_arrives_after_expired_generic_card() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    let now = Utc::now();
    let mut previous_snapshot = sessions_snapshot(1);
    previous_snapshot.active_session_count = 0;
    previous_snapshot.status = "Idle".to_string();
    previous_snapshot.sessions[0].status = "Idle".to_string();
    previous_snapshot.sessions[0].last_activity = now - chrono::Duration::seconds(5);
    previous_snapshot.sessions[0].last_assistant_message = None;
    let previous_session = previous_snapshot.sessions[0].clone();
    runtime.panel_state.last_raw_snapshot = Some(previous_snapshot.clone());
    runtime.panel_state.status_queue = vec![crate::native_panel_core::StatusQueueItem {
        key: "completion:session-1".to_string(),
        session_id: "session-1".to_string(),
        sort_time: previous_session.last_activity,
        expires_at: Instant::now() - Duration::from_millis(1),
        is_live: true,
        is_removing: false,
        remove_after: None,
        payload: crate::native_panel_core::StatusQueuePayload::Completion(previous_session),
    }];

    let mut current_snapshot = previous_snapshot;
    current_snapshot.sessions[0].last_activity = now;
    current_snapshot.sessions[0].last_assistant_message = Some("Done".to_string());

    let sync = runtime
        .sync_snapshot_bundle(&current_snapshot, &input)
        .expect("sync completion message update")
        .expect("snapshot sync result");

    assert!(sync.reminder.play_sound);
    assert!(sync.reminder.show_status_card);
    assert_eq!(sync.panel_transition, Some(true));
    assert_eq!(runtime.panel_state.status_queue.len(), 1);
    assert!(runtime.panel_state.status_queue.iter().any(|item| matches!(
        item.payload,
        crate::native_panel_core::StatusQueuePayload::Completion(_)
    )));
}

#[test]
fn windows_runtime_auto_pops_question_status_card() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    let sync = runtime
        .sync_snapshot_bundle(
            &pending_question_snapshot_with_request("question-1", "session-1"),
            &input,
        )
        .expect("sync question snapshot through shared runtime")
        .expect("snapshot sync result");

    assert!(sync.reminder.play_sound);
    assert!(sync.reminder.show_status_card);
    assert_eq!(sync.panel_transition, Some(true));
    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("question status presentation");
    assert_eq!(presentation.shell.surface, ExpandedSurface::Status);
    assert_eq!(presentation.compact_bar.headline.text, "Question waiting");
    assert!(
        presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusQuestion { .. }))
    );
}

#[test]
fn windows_runtime_keeps_mixed_approval_and_question_status_cards() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    let mut snapshot = pending_permission_snapshot_with_request("req-1", "session-1");
    let question = pending_question_snapshot_with_request("question-1", "session-2");
    snapshot.pending_question_count = question.pending_question_count;
    snapshot.pending_question = question.pending_question.clone();
    snapshot.pending_questions = question.pending_questions.clone();

    runtime
        .sync_snapshot_bundle(&snapshot, &input)
        .expect("sync mixed status snapshot")
        .expect("snapshot sync result");

    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("mixed status presentation");
    assert_eq!(presentation.shell.surface, ExpandedSurface::Status);
    assert_eq!(presentation.compact_bar.headline.text, "Requests waiting");
    assert_eq!(presentation.card_stack.cards.len(), 2);
    assert!(matches!(
        presentation.card_stack.cards[0],
        SceneCard::StatusApproval { .. }
    ));
    assert!(matches!(
        presentation.card_stack.cards[1],
        SceneCard::StatusQuestion { .. }
    ));
}

#[test]
fn windows_runtime_pump_refreshes_status_queue_from_last_raw_snapshot() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    runtime
        .sync_snapshot_bundle(&pending_permission_snapshot("session-1"), &input)
        .expect("seed status queue");
    assert!(!runtime.panel_state.status_queue.is_empty());
    runtime.panel_state.status_queue[0].expires_at = Instant::now() - Duration::from_millis(1);

    runtime
        .pump_platform_loop()
        .expect("pump status queue refresh");

    assert!(runtime.panel_state.status_queue.is_empty());
}

#[test]
fn windows_runtime_status_queue_refresh_does_not_cancel_pending_open_transition() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Close,
            canvas_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
            visible_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
            width_progress: 0.0,
            height_progress: 0.0,
            shoulder_progress: 0.0,
            drop_progress: 0.0,
            cards_progress: 0.0,
        })
        .expect("seed collapsed animation descriptor");

    runtime
        .sync_snapshot_bundle(&pending_permission_snapshot("session-1"), &input)
        .expect("seed status queue");
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );

    runtime.pump_platform_loop().expect("pump pending open");

    assert_eq!(
        runtime
            .host
            .renderer
            .last_animation_descriptor
            .map(|descriptor| descriptor.kind),
        Some(PanelAnimationKind::Open)
    );
}

#[test]
fn windows_scaffold_consumes_shared_scene_bundle() {
    let mut state = PanelState::default();
    let bundle =
        test_runtime_scene_bundle(&mut state, &snapshot(), &PanelSceneBuildInput::default());
    let scene = bundle.scene;
    let runtime_render_state = bundle.runtime_render_state;

    assert!(!scene.cards.is_empty());
    assert!(matches!(
        scene.mascot_pose,
        SceneMascotPose::Idle | SceneMascotPose::Running | SceneMascotPose::Hidden
    ));
    assert!(
        scene
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::Empty))
    );
    assert!(!runtime_render_state.transitioning);
}

#[test]
fn windows_host_lifecycle_tracks_create_show_hide() {
    let mut host = super::WindowsNativePanelHost::default();

    assert_eq!(
        host.window.lifecycle,
        super::host_window::WindowsNativePanelWindowLifecycle::NotCreated
    );
    assert_eq!(
        host.shell.lifecycle(),
        NativePanelHostShellLifecycle::Detached
    );
    assert!(!host.window.descriptor.visible);

    host.show().expect("show host");
    assert_eq!(
        host.window.lifecycle,
        super::host_window::WindowsNativePanelWindowLifecycle::Created
    );
    assert_eq!(
        host.shell.lifecycle(),
        NativePanelHostShellLifecycle::Visible
    );
    assert!(host.window.descriptor.visible);
    assert_eq!(
        host.renderer.last_window_state,
        Some(NativePanelHostWindowState {
            frame: None,
            visible: true,
            preferred_display_index: 0,
        })
    );

    host.reposition_to_display(2, None)
        .expect("reposition host");
    assert_eq!(host.window.descriptor.preferred_display_index, 2);
    assert_eq!(
        host.renderer.last_window_state,
        Some(NativePanelHostWindowState {
            frame: None,
            visible: true,
            preferred_display_index: 2,
        })
    );

    host.set_shared_body_height(320.0)
        .expect("sync shared body height");
    assert_eq!(host.window.descriptor.shared_body_height, Some(320.0));
    assert_eq!(
        host.renderer.last_host_window_descriptor,
        Some(host.window.descriptor)
    );

    host.hide().expect("hide host");
    assert_eq!(
        host.window.lifecycle,
        super::host_window::WindowsNativePanelWindowLifecycle::Created
    );
    assert_eq!(
        host.shell.lifecycle(),
        NativePanelHostShellLifecycle::Hidden
    );
    assert!(!host.window.descriptor.visible);
    assert_eq!(
        host.renderer.last_window_state,
        Some(NativePanelHostWindowState {
            frame: None,
            visible: false,
            preferred_display_index: 2,
        })
    );
}

#[test]
fn windows_host_shell_commands_track_lifecycle_and_reposition() {
    let mut host = super::WindowsNativePanelHost::default();

    host.show().expect("show host");
    host.reposition_to_display(1, None)
        .expect("reposition host");
    host.hide().expect("hide host");

    let commands = host.take_pending_shell_commands();

    assert!(commands.iter().any(|command| matches!(
        command,
        super::window_shell::WindowsNativePanelShellCommand::Create
    )));
    assert!(commands.iter().any(|command| matches!(
        command,
        super::window_shell::WindowsNativePanelShellCommand::Show
    )));
    assert!(commands.iter().any(|command| matches!(
        command,
        super::window_shell::WindowsNativePanelShellCommand::Hide
    )));
    assert!(commands.iter().any(|command| matches!(
        command,
        super::window_shell::WindowsNativePanelShellCommand::SyncWindowState(
            NativePanelHostWindowState {
                preferred_display_index: 1,
                ..
            }
        )
    )));
}

#[test]
fn windows_renderer_caches_shared_animation_descriptor() {
    let mut host = super::WindowsNativePanelHost::default();
    let descriptor = PanelAnimationDescriptor {
        kind: PanelAnimationKind::Open,
        canvas_height: 180.0,
        visible_height: 120.0,
        width_progress: 0.5,
        height_progress: 0.0,
        shoulder_progress: 1.0,
        drop_progress: 0.0,
        cards_progress: 0.25,
    };

    host.apply_animation_descriptor(descriptor)
        .expect("apply descriptor");

    assert_eq!(host.renderer.last_animation_descriptor, Some(descriptor));
    assert_eq!(
        host.renderer.last_timeline_descriptor,
        Some(NativePanelTimelineDescriptor {
            animation: descriptor,
            cards_entering: true,
        })
    );
    assert_eq!(
        host.renderer.last_host_window_descriptor,
        Some(host.window.descriptor)
    );
    assert_eq!(
        host.window.descriptor.timeline,
        Some(NativePanelTimelineDescriptor {
            animation: descriptor,
            cards_entering: true,
        })
    );
    assert!(host.window.last_frame.is_some());
    assert_eq!(
        host.renderer.last_window_state,
        Some(host.window.window_state())
    );
    assert_eq!(
        host.window.lifecycle,
        super::host_window::WindowsNativePanelWindowLifecycle::Created
    );
}

#[test]
fn windows_renderer_caches_pointer_regions_from_host_trait() {
    let mut host = super::WindowsNativePanelHost::default();
    let regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::Shell,
    }];

    host.sync_pointer_regions(&regions)
        .expect("sync pointer regions");

    assert_eq!(host.renderer.last_pointer_regions, regions);
}

#[test]
fn windows_host_queues_platform_events_from_pointer_regions() {
    let mut host = super::WindowsNativePanelHost::default();
    let frame = PanelRect {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 40.0,
    };

    host.queue_platform_event_for_pointer_region(&NativePanelPointerRegion {
        frame,
        kind: NativePanelPointerRegionKind::Shell,
    });
    host.queue_platform_event_for_pointer_region(&NativePanelPointerRegion {
        frame,
        kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
    });
    host.queue_platform_event_for_pointer_region(&NativePanelPointerRegion {
        frame,
        kind: NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Settings),
    });
    host.queue_platform_event_for_pointer_region(&NativePanelPointerRegion {
        frame,
        kind: NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Quit),
    });

    assert_eq!(
        host.take_platform_events(),
        vec![
            NativePanelPlatformEvent::FocusSession("session-1".to_string()),
            NativePanelPlatformEvent::ToggleSettingsSurface,
            NativePanelPlatformEvent::QuitApplication,
        ]
    );
    assert!(host.take_platform_events().is_empty());
}

#[test]
fn windows_host_queues_platform_event_by_point_from_cached_regions() {
    let mut host = super::WindowsNativePanelHost::default();
    host.renderer.last_pointer_regions = vec![
        NativePanelPointerRegion {
            frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            },
            kind: NativePanelPointerRegionKind::CardsContainer,
        },
        NativePanelPointerRegion {
            frame: PanelRect {
                x: 20.0,
                y: 20.0,
                width: 80.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
                action: PanelHitAction::FocusSession,
                value: "session-1".to_string(),
            }),
        },
        NativePanelPointerRegion {
            frame: PanelRect {
                x: 140.0,
                y: 140.0,
                width: 40.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Quit),
        },
    ];

    assert_eq!(
        host.queue_platform_event_at_point(PanelPoint { x: 30.0, y: 30.0 }),
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        host.queue_platform_event_at_point(PanelPoint { x: 150.0, y: 150.0 }),
        Some(NativePanelPlatformEvent::QuitApplication)
    );
    assert_eq!(
        host.queue_platform_event_at_point(PanelPoint { x: 190.0, y: 190.0 }),
        None
    );
    assert_eq!(
        host.take_platform_events(),
        vec![
            NativePanelPlatformEvent::FocusSession("session-1".to_string()),
            NativePanelPlatformEvent::QuitApplication,
        ]
    );
}

#[test]
fn windows_runtime_syncs_hover_expand_from_cached_regions() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.pointer_inside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];

    let transition = runtime.sync_hover_at_point(PanelPoint { x: 30.0, y: 30.0 }, now);

    assert_eq!(transition, Some(HoverTransition::Expand));
    assert!(runtime.panel_state.expanded);
    assert!(runtime.panel_state.pointer_outside_since.is_none());
}

#[test]
fn windows_runtime_syncs_hover_collapse_outside_cached_regions() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
    runtime.panel_state.pointer_outside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];

    let transition = runtime.sync_hover_at_point(PanelPoint { x: 180.0, y: 180.0 }, now);

    assert_eq!(transition, Some(HoverTransition::Collapse));
    assert!(!runtime.panel_state.expanded);
    assert!(runtime.panel_state.pointer_inside_since.is_none());
}

#[test]
fn windows_runtime_reposition_to_selected_display_uses_input_descriptor() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    runtime
        .reposition_to_selected_display_with_input(&input)
        .expect("reposition runtime to selected display");

    assert_eq!(
        runtime.host.window.descriptor.preferred_display_index,
        input.selected_display_index()
    );
    assert_eq!(
        runtime.host.window.descriptor.screen_frame,
        input.screen_frame
    );
}

#[test]
fn windows_runtime_set_shared_body_height_updates_host_descriptor() {
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime
        .set_shared_expanded_body_height(240.0)
        .expect("set shared body height");

    assert_eq!(
        runtime.host.window.descriptor.shared_body_height,
        Some(240.0)
    );
    assert_eq!(
        runtime.host.renderer.last_host_window_descriptor,
        Some(runtime.host.window.descriptor)
    );
}

#[test]
fn windows_runtime_expanded_target_height_prefers_current_native_content_over_stale_shared_height()
{
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime
        .set_shared_expanded_body_height(240.0)
        .expect("set stale shared body height");
    runtime.host.renderer.last_presentation_model =
        shell_draw_frame(Vec::new(), true).presentation_model;

    let expected_height = crate::native_panel_core::DEFAULT_COMPACT_PILL_HEIGHT
        + crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP
        + 70.0
        + crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET;

    assert_eq!(runtime.resolved_expanded_target_height(), expected_height);
}

#[test]
fn windows_runtime_expanded_target_height_prefers_latest_scene_over_stale_presentation_slot() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime
        .sync_snapshot_bundle(&sessions_snapshot(3), &input)
        .expect("sync three session snapshot");
    runtime.host.renderer.last_presentation_model =
        shell_draw_frame(Vec::new(), true).presentation_model;

    let target_height = runtime.resolved_expanded_target_height();
    let stale_height = crate::native_panel_core::DEFAULT_COMPACT_PILL_HEIGHT
        + crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP
        + 70.0
        + crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET;

    assert!(target_height > stale_height);
}

#[test]
fn windows_runtime_expanded_target_height_uses_compact_width_when_scene_has_no_layout_bundle() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let mut input = runtime_input_descriptor();
    input.scene_input.settings.island_width_preset =
        crate::native_panel_core::PanelIslandWidthPreset::Compact;
    let mut snapshot = sessions_snapshot(1);
    snapshot.sessions[0].last_user_prompt = Some("ok".to_string());
    snapshot.sessions[0].last_assistant_message =
        Some("Checking current implementation details and spacing now".to_string());
    runtime
        .sync_snapshot_bundle(&snapshot, &input)
        .expect("sync compact width snapshot");
    runtime.host.renderer.scene_cache.last_render_command_bundle = None;
    runtime.host.renderer.last_presentation_model = None;

    let scene = runtime
        .host
        .renderer
        .scene_cache
        .last_scene
        .as_ref()
        .expect("scene");
    let width_spec = crate::native_panel_core::island_width_spec(
        crate::native_panel_core::PanelIslandWidthPreset::Compact,
    );
    let card_width = crate::native_panel_core::resolve_expanded_cards_width(
        width_spec.expanded_width,
        crate::native_panel_core::EXPANDED_CARDS_SIDE_INSET,
    );
    let expected_body_height =
        crate::native_panel_renderer::facade::presentation::estimated_scene_content_height_for_card_width(
            scene,
            card_width,
        )
        .min(crate::native_panel_core::EXPANDED_MAX_BODY_HEIGHT);
    let expected_height = crate::native_panel_core::DEFAULT_COMPACT_PILL_HEIGHT
        + crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP
        + expected_body_height
        + crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET;

    assert_eq!(runtime.resolved_expanded_target_height(), expected_height);
}

#[test]
fn windows_host_presenter_prefers_latest_scene_over_stale_presentation_slot() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("sync empty state snapshot");
    runtime.host.renderer.last_presentation_model =
        shell_draw_frame(Vec::new(), true).presentation_model;

    runtime
        .host
        .present_renderer_state()
        .expect("present renderer state");
    let presented = runtime
        .host
        .window
        .presented_presentation_model
        .as_ref()
        .expect("presented model");

    assert_eq!(presented.card_stack.cards.len(), 1);
    assert!(matches!(presented.card_stack.cards[0], SceneCard::Empty));
}

#[test]
fn windows_runtime_hover_expand_refreshes_cached_scene_from_last_snapshot() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(pending_permission_snapshot("session-1"));
    runtime.panel_state.pointer_inside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 180.0,
            visible_height: 140.0,
            width_progress: 1.0,
            height_progress: 1.0,
            shoulder_progress: 1.0,
            drop_progress: 1.0,
            cards_progress: 1.0,
        })
        .expect("seed animation descriptor");
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];

    let transition = runtime
        .sync_hover_and_refresh_at_point_with_input(
            PanelPoint { x: 30.0, y: 30.0 },
            now,
            &runtime_input_descriptor(),
        )
        .expect("expand and refresh");

    assert_eq!(transition, Some(HoverTransition::Expand));
    assert!(runtime.panel_state.expanded);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );
    assert!(runtime.scene_cache.last_scene.is_some());
    assert!(runtime.scene_cache.last_runtime_render_state.is_some());
    assert!(runtime.host.renderer.scene_cache.last_scene.is_some());
    assert!(
        runtime
            .host
            .renderer
            .scene_cache
            .last_runtime_render_state
            .is_some()
    );
    assert!(
        runtime
            .scene_cache
            .last_scene
            .as_ref()
            .is_some_and(|scene| {
                scene.hit_targets.iter().any(|target| {
                    target.action == PanelHitAction::FocusSession && target.value == "session-1"
                })
            })
    );
}

#[test]
fn windows_runtime_hover_collapse_refreshes_cached_scene_from_last_snapshot() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.panel_state.expanded = true;
    runtime.panel_state.pointer_outside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Close,
            canvas_height: 120.0,
            visible_height: 120.0,
            width_progress: 0.0,
            height_progress: 0.0,
            shoulder_progress: 0.0,
            drop_progress: 0.0,
            cards_progress: 0.0,
        })
        .expect("seed animation descriptor");
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];

    let transition = runtime
        .sync_hover_and_refresh_at_point_with_input(
            PanelPoint { x: 180.0, y: 180.0 },
            now,
            &runtime_input_descriptor(),
        )
        .expect("collapse and refresh");

    assert_eq!(transition, Some(HoverTransition::Collapse));
    assert!(!runtime.panel_state.expanded);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Close)
    );
    assert!(runtime.scene_cache.last_scene.is_some());
    assert!(runtime.host.renderer.scene_cache.last_scene.is_some());
    assert!(
        runtime
            .scene_cache
            .last_scene
            .as_ref()
            .is_some_and(|scene| scene.compact_bar.actions_visible)
    );
    assert!(
        runtime
            .scene_cache
            .last_scene
            .as_ref()
            .is_some_and(|scene| scene.hit_targets.is_empty())
    );
}

#[test]
fn windows_runtime_hover_transition_without_snapshot_keeps_collapsed_state() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.pointer_inside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];

    let transition = runtime
        .sync_hover_and_refresh_at_point_with_input(
            PanelPoint { x: 30.0, y: 30.0 },
            now,
            &runtime_input_descriptor(),
        )
        .expect("hover without snapshot");

    assert_eq!(transition, None);
    assert!(!runtime.panel_state.expanded);
    assert!(runtime.scene_cache.last_scene.is_none());
    assert!(runtime.host.renderer.scene_cache.last_scene.is_none());
}

#[test]
fn windows_runtime_polling_hover_without_snapshot_keeps_collapsed_state() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.pointer_inside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime.host.presenter.present(shell_draw_frame(
        vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 20.0,
                y: 20.0,
                width: 80.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        false,
    ));
    assert!(runtime.host.consume_presenter_into_shell());

    let interaction = runtime
        .sync_host_polling_interaction_and_refresh(
            PanelPoint { x: 30.0, y: 30.0 },
            false,
            now,
            &runtime_input_descriptor(),
        )
        .expect("poll hover without snapshot")
        .expect("polling facts exist");

    assert!(interaction.interactive_inside);
    assert_eq!(interaction.transition_request, None);
    assert_eq!(runtime.last_transition_request, None);
    assert!(!runtime.panel_state.expanded);
    assert!(runtime.scene_cache.last_scene.is_none());
    assert!(runtime.host.renderer.scene_cache.last_scene.is_none());
}

#[test]
fn windows_runtime_toggle_settings_surface_updates_cached_scene() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.panel_state.expanded = true;
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
        .expect("seed animation descriptor");

    let changed = runtime
        .toggle_settings_surface_with_input(&runtime_input_descriptor())
        .expect("toggle settings surface");

    assert!(changed);
    assert_eq!(runtime.panel_state.surface_mode, ExpandedSurface::Settings);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::SurfaceSwitch)
    );
    assert_eq!(
        runtime
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Settings)
    );
    assert_eq!(
        runtime
            .host
            .renderer
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Settings)
    );
}

#[test]
fn windows_runtime_toggle_settings_surface_marks_completion_badge_as_viewed() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.panel_state.expanded = true;
    runtime.panel_state.completion_badge_items = vec![CompletionBadgeItem {
        session_id: "session-1".to_string(),
        completed_at: Utc::now(),
        last_user_prompt: Some("ship it".to_string()),
        last_assistant_message: Some("Done".to_string()),
    }];
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
        .expect("seed animation descriptor");

    let changed = runtime
        .toggle_settings_surface_with_input(&runtime_input_descriptor())
        .expect("toggle settings surface");

    assert!(changed);
    assert!(runtime.panel_state.completion_badge_items.is_empty());
}

#[test]
fn windows_runtime_toggle_settings_surface_cycles_back_to_default() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.panel_state.expanded = true;
    runtime.panel_state.surface_mode = ExpandedSurface::Settings;
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
        .expect("seed animation descriptor");

    let changed = runtime
        .toggle_settings_surface_with_input(&runtime_input_descriptor())
        .expect("toggle settings surface");

    assert!(changed);
    assert_eq!(runtime.panel_state.surface_mode, ExpandedSurface::Default);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::SurfaceSwitch)
    );
    assert_eq!(
        runtime
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Default)
    );
}

#[test]
fn windows_runtime_toggle_settings_surface_from_status_updates_cached_scene() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
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
        .expect("seed animation descriptor");
    runtime
        .sync_snapshot_bundle(
            &pending_permission_snapshot_with_request("req-1", "session-1"),
            &runtime_input_descriptor(),
        )
        .expect("seed status surface snapshot");
    assert_eq!(runtime.panel_state.surface_mode, ExpandedSurface::Status);

    let changed = runtime
        .toggle_settings_surface_with_input(&runtime_input_descriptor())
        .expect("toggle settings surface from status");

    assert!(changed);
    assert_eq!(runtime.panel_state.surface_mode, ExpandedSurface::Settings);
    assert!(!runtime.panel_state.status_queue.is_empty());
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::SurfaceSwitch)
    );
    assert_eq!(
        runtime
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Settings)
    );
    assert_eq!(
        runtime
            .host
            .renderer
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Settings)
    );
}

#[test]
fn windows_runtime_sync_snapshot_can_return_from_settings_to_status_on_new_item() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
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
        .expect("seed animation descriptor");
    runtime
        .sync_snapshot_bundle(
            &pending_permission_snapshot_with_request("req-1", "session-1"),
            &runtime_input_descriptor(),
        )
        .expect("seed status surface snapshot");
    runtime
        .toggle_settings_surface_with_input(&runtime_input_descriptor())
        .expect("switch to settings");
    assert_eq!(runtime.panel_state.surface_mode, ExpandedSurface::Settings);

    runtime
        .sync_snapshot_bundle(
            &pending_permission_snapshot_with_request("req-2", "session-2"),
            &runtime_input_descriptor(),
        )
        .expect("sync new status item");

    assert_eq!(runtime.panel_state.surface_mode, ExpandedSurface::Status);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::SurfaceSwitch)
    );
    assert_eq!(
        runtime
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Status)
    );
    assert_eq!(
        runtime
            .host
            .renderer
            .scene_cache
            .last_scene
            .as_ref()
            .map(|scene| scene.surface),
        Some(ExpandedSurface::Status)
    );
}

#[test]
fn windows_runtime_dispatches_click_command_at_point_through_handler() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
    runtime.host.renderer.last_pointer_regions = vec![
        NativePanelPointerRegion {
            frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            },
            kind: NativePanelPointerRegionKind::CardsContainer,
        },
        NativePanelPointerRegion {
            frame: PanelRect {
                x: 20.0,
                y: 20.0,
                width: 80.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
                action: PanelHitAction::FocusSession,
                value: "session-1".to_string(),
            }),
        },
    ];
    let mut handler = RecordingEventHandler::default();

    let event = runtime
        .dispatch_click_command_at_point_with_handler(
            PanelPoint { x: 30.0, y: 30.0 },
            Instant::now(),
            &mut handler,
        )
        .expect("dispatch point event");

    assert_eq!(
        event,
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        handler.handled,
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
    assert!(runtime.host.pending_events.is_empty());
}

#[test]
fn windows_runtime_dispatches_edge_action_click_during_open_transition() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
    runtime.panel_state.transitioning = true;
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 40.0,
            height: 32.0,
        },
        kind: NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Settings),
    }];
    let mut handler = RecordingEventHandler::default();

    let event = runtime
        .dispatch_click_command_at_point_with_handler(
            PanelPoint { x: 30.0, y: 30.0 },
            Instant::now(),
            &mut handler,
        )
        .expect("dispatch edge action during transition");

    assert_eq!(event, Some(NativePanelPlatformEvent::ToggleSettingsSurface));
    assert_eq!(
        handler.handled,
        vec![NativePanelPlatformEvent::ToggleSettingsSurface]
    );
    assert!(runtime.host.pending_events.is_empty());
}

#[test]
fn windows_runtime_dispatches_queued_platform_events_through_handler() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.host.pending_events = vec![
        NativePanelPlatformEvent::FocusSession("session-1".to_string()),
        NativePanelPlatformEvent::ToggleCompletionSound,
    ];
    let mut handler = RecordingEventHandler::default();

    dispatch_queued_native_panel_platform_events_with_handler(&mut runtime.host, &mut handler)
        .expect("dispatch queued runtime events");

    assert_eq!(
        handler.handled,
        vec![
            NativePanelPlatformEvent::FocusSession("session-1".to_string()),
            NativePanelPlatformEvent::ToggleCompletionSound,
        ]
    );
    assert!(runtime.host.pending_events.is_empty());
}

#[test]
fn windows_runtime_can_drain_queued_platform_events_without_dispatching_under_lock() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.host.pending_events = vec![
        NativePanelPlatformEvent::ToggleSettingsSurface,
        NativePanelPlatformEvent::CycleDisplay,
    ];

    let events = runtime.take_queued_platform_events();

    assert_eq!(
        events,
        vec![
            NativePanelPlatformEvent::ToggleSettingsSurface,
            NativePanelPlatformEvent::CycleDisplay,
        ]
    );
    assert!(runtime.host.pending_events.is_empty());
}

#[test]
fn windows_runtime_pointer_event_dispatch_is_noop_when_point_has_no_target() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 60.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];
    let mut handler = RecordingEventHandler::default();

    let event = runtime
        .dispatch_click_command_at_point_with_handler(
            PanelPoint { x: 10.0, y: 10.0 },
            Instant::now(),
            &mut handler,
        )
        .expect("dispatch empty point event");

    assert_eq!(event, None);
    assert!(handler.handled.is_empty());
    assert!(runtime.host.pending_events.is_empty());
}

#[test]
fn windows_runtime_focus_click_dispatch_is_debounced() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
    }];
    let mut handler = RecordingEventHandler::default();

    let first = runtime
        .dispatch_click_command_at_point_with_handler(
            PanelPoint { x: 30.0, y: 30.0 },
            now,
            &mut handler,
        )
        .expect("dispatch first focus click");
    let duplicate = runtime
        .dispatch_click_command_at_point_with_handler(
            PanelPoint { x: 30.0, y: 30.0 },
            now + Duration::from_millis(100),
            &mut handler,
        )
        .expect("dispatch duplicate focus click");

    assert_eq!(
        first,
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(duplicate, None);
    assert_eq!(
        handler.handled,
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
}
