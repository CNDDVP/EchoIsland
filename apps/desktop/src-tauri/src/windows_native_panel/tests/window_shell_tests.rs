use super::*;

#[test]
fn windows_runtime_window_message_pointer_leave_collapses_and_refreshes() {
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
    let mut handler = RecordingEventHandler::default();

    let outcome = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Leave,
            now,
            &runtime_input_descriptor(),
            &mut handler,
        )
        .expect("handle pointer leave");

    assert_eq!(
        outcome,
        NativePanelPointerInputOutcome::Hover(Some(HoverTransition::Collapse))
    );
    assert!(!runtime.panel_state.expanded);
    assert!(runtime.scene_cache.last_scene.is_some());
    assert!(runtime.host.renderer.scene_cache.last_scene.is_some());
    assert!(handler.handled.is_empty());
}

#[test]
fn windows_runtime_window_message_click_dispatches_hit_target_event() {
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

    let outcome = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Click(PanelPoint { x: 30.0, y: 30.0 }),
            std::time::Instant::now(),
            &NativePanelRuntimeInputDescriptor {
                screen_scale_factor: None,
                screen_physical_frame: None,
                scene_input: PanelSceneBuildInput::default(),
                screen_frame: None,
            },
            &mut handler,
        )
        .expect("handle pointer click");

    assert_eq!(
        outcome,
        NativePanelPointerInputOutcome::Click(Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )))
    );
    assert_eq!(
        handler.handled,
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
}

#[test]
fn windows_runtime_clicks_visual_settings_button_center() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();
    runtime.panel_state.expanded = true;

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
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
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("seed expanded scene");
    runtime
        .host
        .present_renderer_state()
        .expect("present expanded frame");
    runtime
        .pump_platform_loop()
        .expect("sync expanded frame into shell");

    let window_state = runtime
        .host
        .window
        .presented_window_state
        .expect("presented window state");
    let surface_height = window_state.frame.expect("window frame").height;
    let presentation = runtime
        .host
        .window
        .presented_presentation_model
        .as_ref()
        .expect("presented presentation");
    let visual_input = native_panel_visual_plan_input_from_presentation(
        window_state,
        NativePanelVisualDisplayMode::Expanded,
        Some(presentation),
    );
    let plan = resolve_native_panel_visual_plan(&visual_input);
    let settings_icon_center = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                origin,
                max_width,
                text,
                size,
                weight,
                ..
            } if text == "\u{E713}"
                && *size == 16
                && *weight == NativePanelVisualTextWeight::Normal =>
            {
                Some(PanelPoint {
                    x: origin.x + max_width / 2.0,
                    y: surface_height - origin.y - 12.0,
                })
            }
            _ => None,
        })
        .expect("settings icon text primitive");

    assert_eq!(
        runtime
            .host
            .shell
            .pointer_state_at_point(settings_icon_center)
            .platform_event,
        Some(NativePanelPlatformEvent::ToggleSettingsSurface)
    );
    assert_eq!(
        super::platform_loop::resolve_windows_native_panel_cached_hit_test(
            hwnd,
            settings_icon_center
        ),
        super::hit_region::WindowsNativePanelHitTest::Client
    );

    let mut handler = RecordingEventHandler::default();
    let outcome = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Click(settings_icon_center),
            Instant::now(),
            &input,
            &mut handler,
        )
        .expect("click settings icon center");

    assert_eq!(
        outcome,
        NativePanelPointerInputOutcome::Click(Some(
            NativePanelPlatformEvent::ToggleSettingsSurface
        ))
    );
    assert_eq!(
        handler.handled,
        vec![NativePanelPlatformEvent::ToggleSettingsSurface]
    );
}

#[test]
fn windows_runtime_window_message_helper_decodes_and_expands_hover() {
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
    let mut handler = RecordingEventHandler::default();

    let outcome = runtime
        .handle_window_message_with_handler(
            super::window_shell::WINDOWS_WM_MOUSEMOVE,
            ((30_i32 as u32 as u64) | ((30_i32 as u32 as u64) << 16)) as isize,
            now,
            &runtime_input_descriptor(),
            &mut handler,
        )
        .expect("handle decoded move message");

    assert_eq!(
        outcome,
        Some(NativePanelPointerInputOutcome::Hover(Some(
            HoverTransition::Expand
        )))
    );
    assert_eq!(
        runtime.host.shell.last_pointer_input(),
        Some(NativePanelPointerInput::Move(PanelPoint {
            x: 30.0,
            y: 30.0
        }))
    );
    assert!(runtime.panel_state.expanded);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );
    assert!(handler.handled.is_empty());
}

#[test]
fn windows_runtime_keeps_pending_hover_open_after_badge_clearing_mousemove() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.panel_state.pointer_inside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime.panel_state.completion_badge_items = vec![CompletionBadgeItem {
        session_id: "session-1".to_string(),
        completed_at: Utc::now(),
        last_user_prompt: Some("prompt".to_string()),
        last_assistant_message: Some("done".to_string()),
    }];
    runtime.host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];
    let mut handler = RecordingEventHandler::default();

    let first = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Move(PanelPoint { x: 30.0, y: 30.0 }),
            now,
            &runtime_input_descriptor(),
            &mut handler,
        )
        .expect("first hover clears badge and requests open");
    let second = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Move(PanelPoint { x: 30.0, y: 30.0 }),
            now + Duration::from_millis(1),
            &runtime_input_descriptor(),
            &mut handler,
        )
        .expect("second hover before animation starts");

    assert_eq!(
        first,
        NativePanelPointerInputOutcome::Hover(Some(HoverTransition::Expand))
    );
    assert_eq!(second, NativePanelPointerInputOutcome::Hover(None));
    assert!(runtime.panel_state.completion_badge_items.is_empty());
    assert!(runtime.panel_state.expanded);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );
}

#[test]
fn windows_runtime_window_message_expands_hover_after_presenting_shared_absolute_regions() {
    let now = std::time::Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(pending_permission_snapshot("session-1"));
    runtime.panel_state.pointer_inside_since =
        Some(now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100));
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    runtime.host.window.present(
        NativePanelHostWindowState {
            screen_scale_factor: None,
            screen_physical_frame: None,
            frame: Some(PanelRect {
                x: 100.0,
                y: 50.0,
                width: 320.0,
                height: 120.0,
            }),
            visible: true,
            preferred_display_index: 0,
        },
        &[NativePanelPointerRegion {
            frame: PanelRect {
                x: 110.0,
                y: 110.0,
                width: 80.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        None,
    );
    let frame = runtime
        .host
        .window
        .take_pending_draw_frame()
        .expect("pending draw frame");
    runtime.host.presenter.present(frame);
    runtime
        .pump_platform_loop()
        .expect("present shared regions");
    assert_eq!(
        runtime.host.shell.pointer_regions(),
        &[NativePanelPointerRegion {
            frame: PanelRect {
                x: 10.0,
                y: 20.0,
                width: 80.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }]
    );
    let mut handler = RecordingEventHandler::default();

    let outcome = runtime
        .handle_window_message_with_handler(
            super::window_shell::WINDOWS_WM_MOUSEMOVE,
            ((30_i32 as u32 as u64) | ((30_i32 as u32 as u64) << 16)) as isize,
            now,
            &runtime_input_descriptor(),
            &mut handler,
        )
        .expect("handle decoded move message");

    assert_eq!(
        outcome,
        Some(NativePanelPointerInputOutcome::Hover(Some(
            HoverTransition::Expand
        )))
    );
    assert!(runtime.panel_state.expanded);
}

#[test]
fn windows_runtime_window_message_helper_decodes_and_dispatches_click() {
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

    let outcome = runtime
        .handle_window_message_with_handler(
            super::window_shell::WINDOWS_WM_LBUTTONUP,
            ((30_i32 as u32 as u64) | ((30_i32 as u32 as u64) << 16)) as isize,
            std::time::Instant::now(),
            &NativePanelRuntimeInputDescriptor {
                screen_scale_factor: None,
                screen_physical_frame: None,
                scene_input: PanelSceneBuildInput::default(),
                screen_frame: None,
            },
            &mut handler,
        )
        .expect("handle decoded click message");

    assert_eq!(
        outcome,
        Some(NativePanelPointerInputOutcome::Click(Some(
            NativePanelPlatformEvent::FocusSession("session-1".to_string())
        )))
    );
    assert_eq!(
        handler.handled,
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
}

#[test]
fn windows_runtime_pump_platform_loop_tracks_window_state_command() {
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump platform loop");

    assert_eq!(
        runtime.platform_loop.last_window_state,
        Some(runtime.host.window.window_state())
    );
    assert!(runtime.platform_loop.applied_command_count > 0);
    assert!(runtime.host.take_pending_shell_commands().is_empty());
}

#[test]
fn windows_runtime_pump_platform_loop_backfills_shell_raw_window_handle() {
    let mut runtime = super::WindowsNativePanelRuntime::default();

    assert_eq!(runtime.host.shell.raw_window_handle(), None);

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump platform loop");

    assert!(runtime.host.shell.raw_window_handle().is_some());
    assert_eq!(
        runtime.platform_loop.last_raw_window_handle,
        runtime.host.shell.raw_window_handle()
    );
}

#[test]
fn windows_runtime_pump_platform_loop_clears_shell_raw_window_handle_on_destroy() {
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    super::platform_loop::sync_windows_native_panel_hit_regions(
        Some(hwnd),
        &[NativePanelPointerRegion {
            frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
    );

    runtime.host.shell.destroy();
    runtime.pump_platform_loop().expect("pump destroy");

    assert_eq!(runtime.host.shell.raw_window_handle(), None);
    assert_eq!(runtime.platform_loop.last_raw_window_handle, None);
    assert_eq!(
        super::platform_loop::resolve_windows_native_panel_cached_hit_test(
            hwnd,
            PanelPoint { x: 10.0, y: 10.0 }
        ),
        super::hit_region::WindowsNativePanelHitTest::Transparent
    );
}

#[test]
fn windows_runtime_pump_window_messages_consumes_paint_job() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");

    runtime.host.presenter.present(WindowsNativePanelDrawFrame {
        window_state: NativePanelHostWindowState {
            screen_scale_factor: None,
            screen_physical_frame: None,
            frame: Some(PanelRect {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 100.0,
            }),
            visible: true,
            preferred_display_index: 0,
        },
        pointer_regions: Vec::new(),
        presentation_model: None,
    });
    runtime.pump_platform_loop().expect("pump presenter frame");

    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    super::clear_windows_native_panel_window_messages(Some(hwnd));
    super::queue_windows_native_panel_window_message(hwnd, super::WINDOWS_WM_PAINT, 0);

    runtime
        .pump_window_messages()
        .expect("pump window messages");

    assert_eq!(
        runtime.platform_loop.last_window_message_id,
        Some(super::WINDOWS_WM_PAINT)
    );
    assert!(runtime.platform_loop.paint_dispatch_count >= 1);
    assert!(
        runtime
            .platform_loop
            .last_paint_plan
            .as_ref()
            .is_some_and(|plan| !plan.hidden && !plan.primitives.is_empty())
    );
    assert_eq!(
        runtime
            .platform_loop
            .last_painted_job
            .as_ref()
            .map(|job| job.panel_frame.width),
        Some(300.0)
    );
    assert_eq!(
        runtime.platform_loop.last_paint_surface_resource_revision,
        Some(runtime.platform_loop.surface_resource_revision)
    );
    assert_eq!(
        runtime.platform_loop.paint_surface_resource_rebuild_count,
        1
    );
    assert!(runtime.host.shell.pending_paint_job().is_none());
}

#[test]
fn windows_runtime_pump_window_messages_queues_click_event() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    sync_test_pointer_regions(
        &mut runtime,
        vec![NativePanelPointerRegion {
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
        }],
    );

    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    super::clear_windows_native_panel_window_messages(Some(hwnd));
    super::queue_windows_native_panel_window_message(
        hwnd,
        super::window_shell::WINDOWS_WM_LBUTTONUP,
        ((30_i32 as u32 as u64) | ((30_i32 as u32 as u64) << 16)) as isize,
    );

    runtime
        .pump_window_messages()
        .expect("pump window messages");

    assert_eq!(
        runtime.platform_loop.last_window_message_id,
        Some(super::window_shell::WINDOWS_WM_LBUTTONUP)
    );
    assert_eq!(
        runtime.host.take_platform_events(),
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
}

#[test]
fn windows_runtime_pump_window_messages_routes_move_message_into_pointer_path() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(pending_permission_snapshot("session-1"));
    runtime.panel_state.pointer_inside_since = Some(
        Instant::now() - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100),
    );
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
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");

    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    super::clear_windows_native_panel_window_messages(Some(hwnd));
    super::queue_windows_native_panel_window_message(
        hwnd,
        super::window_shell::WINDOWS_WM_MOUSEMOVE,
        ((30_i32 as u32 as u64) | ((30_i32 as u32 as u64) << 16)) as isize,
    );

    runtime
        .pump_window_messages()
        .expect("pump window messages");

    assert_eq!(
        runtime.platform_loop.last_window_message_id,
        Some(super::window_shell::WINDOWS_WM_MOUSEMOVE)
    );
    assert!(runtime.platform_loop.processed_window_message_count >= 1);
    assert_eq!(
        runtime.host.shell.last_pointer_input(),
        Some(NativePanelPointerInput::Move(PanelPoint {
            x: 30.0,
            y: 30.0
        }))
    );
    assert!(runtime.host.take_platform_events().is_empty());
}

#[test]
fn windows_runtime_pump_window_messages_leave_collapses_and_refreshes() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.panel_state.expanded = true;
    runtime.panel_state.pointer_outside_since = Some(
        Instant::now() - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100),
    );
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
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");

    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    super::clear_windows_native_panel_window_messages(Some(hwnd));
    super::queue_windows_native_panel_window_message(
        hwnd,
        super::window_shell::WINDOWS_WM_MOUSELEAVE,
        0,
    );

    runtime
        .pump_window_messages()
        .expect("pump window messages");

    assert_eq!(
        runtime.platform_loop.last_window_message_id,
        Some(super::window_shell::WINDOWS_WM_MOUSELEAVE)
    );
    assert_eq!(
        runtime.host.shell.last_pointer_input(),
        Some(NativePanelPointerInput::Leave)
    );
    assert!(!runtime.panel_state.expanded);
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Close)
    );
    assert!(runtime.scene_cache.last_scene.is_some());
    assert!(runtime.host.take_platform_events().is_empty());
}

#[test]
fn windows_runtime_pump_platform_loop_processes_leave_before_unstarted_hover_open() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
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
    runtime.panel_state.expanded = true;
    runtime.last_transition_request = Some(NativePanelTransitionRequest::Open);

    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    super::clear_windows_native_panel_window_messages(Some(hwnd));
    super::queue_windows_native_panel_window_message(
        hwnd,
        super::window_shell::WINDOWS_WM_MOUSELEAVE,
        0,
    );

    runtime
        .pump_platform_loop()
        .expect("pump leave before unstarted open");

    assert_eq!(
        runtime.host.shell.last_pointer_input(),
        Some(NativePanelPointerInput::Leave)
    );
    assert_eq!(runtime.last_transition_request, None);
    assert!(!runtime.panel_state.expanded);
    assert!(!runtime.panel_state.transitioning);
    assert_eq!(
        runtime
            .host
            .renderer
            .last_animation_descriptor
            .map(|descriptor| descriptor.kind),
        Some(PanelAnimationKind::Close)
    );
}

#[test]
fn windows_runtime_pump_window_messages_debounces_focus_clicks() {
    let _guard = window_message_queue_test_guard();
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime.panel_state.expanded = true;
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    sync_test_pointer_regions(
        &mut runtime,
        vec![NativePanelPointerRegion {
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
        }],
    );

    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    super::clear_windows_native_panel_window_messages(Some(hwnd));
    let click_lparam = ((30_i32 as u32 as u64) | ((30_i32 as u32 as u64) << 16)) as isize;
    super::queue_windows_native_panel_window_message(
        hwnd,
        super::window_shell::WINDOWS_WM_LBUTTONUP,
        click_lparam,
    );
    super::queue_windows_native_panel_window_message(
        hwnd,
        super::window_shell::WINDOWS_WM_LBUTTONUP,
        click_lparam,
    );

    runtime
        .pump_window_messages()
        .expect("pump window messages");

    assert_eq!(
        runtime.platform_loop.last_window_message_id,
        Some(super::window_shell::WINDOWS_WM_LBUTTONUP)
    );
    assert!(runtime.platform_loop.processed_window_message_count >= 2);
    assert_eq!(
        runtime.host.take_platform_events(),
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
    assert!(
        runtime
            .last_focus_click
            .as_ref()
            .is_some_and(|(session_id, _)| session_id == "session-1")
    );
}

#[test]
fn windows_runtime_pump_platform_loop_tracks_lifecycle_and_redraw_commands() {
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime.host.show().expect("show host");
    runtime.host.shell.request_redraw();
    runtime.host.hide().expect("hide host");
    runtime.host.shell.destroy();
    runtime.pump_platform_loop().expect("pump platform loop");

    assert_eq!(runtime.platform_loop.create_count, 1);
    assert_eq!(runtime.platform_loop.show_count, 1);
    assert_eq!(runtime.platform_loop.hide_count, 1);
    assert_eq!(runtime.platform_loop.destroy_count, 1);
    assert_eq!(runtime.platform_loop.redraw_request_count, 1);
    assert_eq!(runtime.platform_loop.topmost_reassert_count, 1);
    assert_eq!(runtime.platform_loop.last_visible, Some(false));
    assert!(runtime.host.take_pending_shell_commands().is_empty());
}

#[test]
fn windows_spawn_platform_loops_marks_shell_state() {
    let before = super::runtime_entry::with_windows_native_panel_runtime(|runtime| {
        Ok(runtime.host.shell.platform_loop_spawn_count())
    })
    .expect("inspect pre-spawn runtime");

    super::runtime_entry::spawn_platform_loops_internal();

    super::runtime_entry::with_windows_native_panel_runtime(|runtime| {
        assert!(runtime.host.shell.platform_loop_started());
        assert!(runtime.host.shell.platform_loop_spawn_count() > before);
        Ok(())
    })
    .expect("inspect runtime");
}

#[test]
fn windows_spawn_platform_loops_background_thread_drains_public_api_work() {
    super::runtime_entry::spawn_platform_loops_internal();
    let before = super::windows_native_platform_loop_generations()
        .expect("platform loop generations")
        .0;

    super::create_native_panel().expect("create native panel");

    let after_create = super::windows_native_platform_loop_generations()
        .expect("platform loop generations")
        .0;
    assert!(after_create > before);
    assert!(super::wait_windows_native_platform_loop_processed_at_least(
        after_create,
        1000
    ));

    super::runtime_entry::with_windows_native_panel_runtime(|runtime| {
        assert!(runtime.host.shell.raw_window_handle().is_some());
        Ok(())
    })
    .expect("inspect runtime");
}

#[test]
fn windows_host_recomputes_cached_frame_when_display_changes() {
    let mut host = super::WindowsNativePanelHost::default();
    let descriptor = PanelAnimationDescriptor {
        kind: PanelAnimationKind::Open,
        canvas_height: 120.0,
        visible_height: 120.0,
        width_progress: 1.0,
        height_progress: 0.0,
        shoulder_progress: 0.0,
        drop_progress: 0.0,
        cards_progress: 0.0,
    };

    host.apply_animation_descriptor(descriptor)
        .expect("apply descriptor");
    host.reposition_to_display(
        1,
        Some(PanelRect {
            x: 500.0,
            y: 100.0,
            width: 800.0,
            height: 600.0,
        }),
    )
    .expect("reposition host");

    let width_spec = crate::native_panel_core::island_width_spec(
        crate::app_settings::current_app_settings().island_width_preset,
    );
    let expected_frame = super::host_window::resolve_windows_panel_window_frame(
        descriptor,
        PanelRect {
            x: 500.0,
            y: 100.0,
            width: 800.0,
            height: 600.0,
        },
        width_spec.canvas_width,
        width_spec.canvas_width,
    );

    assert_eq!(host.window.descriptor.preferred_display_index, 1);
    assert_eq!(host.window.last_frame, Some(expected_frame));
    assert_eq!(
        host.renderer.last_window_state,
        Some(host.window.window_state())
    );
}

#[test]
fn windows_window_frame_uses_canvas_width_to_contain_local_layout() {
    let descriptor = PanelAnimationDescriptor {
        kind: PanelAnimationKind::Open,
        canvas_height: 120.0,
        visible_height: 160.0,
        width_progress: 0.25,
        height_progress: 0.0,
        shoulder_progress: 0.0,
        drop_progress: 0.0,
        cards_progress: 0.0,
    };

    let frame = super::host_window::resolve_windows_panel_window_frame(
        descriptor,
        PanelRect {
            x: 100.0,
            y: 50.0,
            width: 1000.0,
            height: 700.0,
        },
        200.0,
        400.0,
    );

    assert_eq!(
        frame,
        PanelRect {
            x: 390.0,
            y: 50.0,
            width: 420.0,
            height: 160.0,
        }
    );
}

#[cfg(not(windows))]
#[test]
fn windows_native_ui_remains_disabled_on_non_windows() {
    assert!(!super::native_ui_enabled());
}

#[cfg(windows)]
#[test]
fn windows_native_ui_is_enabled_by_default_on_windows() {
    assert!(super::native_ui_enabled());
}
