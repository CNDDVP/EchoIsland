use super::*;

#[test]
fn windows_host_dispatches_queued_platform_events_through_handler() {
    let mut host = super::WindowsNativePanelHost::default();
    host.pending_events = vec![
        NativePanelPlatformEvent::FocusSession("session-1".to_string()),
        NativePanelPlatformEvent::ToggleCompletionSound,
        NativePanelPlatformEvent::ToggleMascot,
        NativePanelPlatformEvent::OpenSettingsLocation,
        NativePanelPlatformEvent::OpenReleasePage,
    ];
    let mut handler = RecordingEventHandler::default();

    dispatch_queued_native_panel_platform_events_with_handler(&mut host, &mut handler)
        .expect("dispatch queued events");

    assert_eq!(
        handler.handled,
        vec![
            NativePanelPlatformEvent::FocusSession("session-1".to_string()),
            NativePanelPlatformEvent::ToggleCompletionSound,
            NativePanelPlatformEvent::ToggleMascot,
            NativePanelPlatformEvent::OpenSettingsLocation,
            NativePanelPlatformEvent::OpenReleasePage,
        ]
    );
    assert!(host.pending_events.is_empty());
}

#[test]
fn windows_renderer_caches_scene_and_resolves_shared_render_inputs() {
    let mut panel_state = PanelState::default();
    let bundle = test_runtime_scene_bundle(
        &mut panel_state,
        &snapshot(),
        &PanelSceneBuildInput::default(),
    );
    let scene = bundle.scene;
    let runtime_render_state = bundle.runtime_render_state;
    let mut renderer = super::WindowsNativePanelRenderer::default();

    renderer.update_screen_frame(Some(PanelRect {
        x: 100.0,
        y: 50.0,
        width: 1000.0,
        height: 700.0,
    }));
    renderer
        .render_scene(&scene, runtime_render_state)
        .expect("render scene");
    let animation_descriptor = PanelAnimationDescriptor {
        kind: PanelAnimationKind::Open,
        canvas_height: 180.0,
        visible_height: 140.0,
        width_progress: 0.5,
        height_progress: 0.75,
        shoulder_progress: 1.0,
        drop_progress: 0.25,
        cards_progress: 0.8,
    };
    renderer
        .apply_animation_descriptor(animation_descriptor)
        .expect("apply descriptor");

    assert_eq!(
        renderer
            .scene_cache
            .last_scene
            .as_ref()
            .map(|cached| cached.surface),
        Some(scene.surface)
    );
    assert_eq!(
        renderer.scene_cache.last_runtime_render_state,
        Some(runtime_render_state)
    );
    let width_spec = crate::native_panel_core::island_width_spec(
        crate::app_settings::current_app_settings().island_width_preset,
    );
    let animation_plan = resolve_native_panel_animation_plan(
        NativePanelTimelineDescriptor {
            animation: animation_descriptor,
            cards_entering: true,
        },
        scene.cards.len(),
    );
    let expected_layout = crate::native_panel_core::resolve_panel_layout(
        crate::native_panel_core::PanelLayoutInput {
            screen_frame: PanelRect {
                x: 100.0,
                y: 50.0,
                width: 1000.0,
                height: 700.0,
            },
            metrics: crate::native_panel_core::PanelGeometryMetrics {
                compact_height: crate::native_panel_core::DEFAULT_COMPACT_PILL_HEIGHT,
                compact_width: width_spec.compact_width,
                expanded_width: width_spec.expanded_width,
                panel_width: width_spec.canvas_width,
            },
            canvas_height: animation_descriptor.canvas_height,
            visible_height: animation_descriptor.visible_height,
            bar_progress: animation_descriptor.width_progress,
            height_progress: animation_descriptor.height_progress,
            drop_progress: animation_descriptor.drop_progress,
            content_visibility: animation_plan.card_stack.visibility_progress,
            collapsed_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
            drop_distance: crate::native_panel_core::PANEL_DROP_DISTANCE,
            content_top_gap: crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP,
            content_bottom_inset: crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET,
            cards_side_inset: crate::native_panel_core::EXPANDED_CARDS_SIDE_INSET,
            shoulder_size: crate::native_panel_core::COMPACT_SHOULDER_SIZE,
            separator_side_inset: crate::native_panel_core::EXPANDED_SEPARATOR_SIDE_INSET,
        },
    );
    assert_eq!(renderer.last_layout, Some(expected_layout));

    let render_state = renderer.last_render_state.expect("cached render state");
    assert!(!render_state.shared.enabled);
    assert!(!render_state.shared.visible);
    assert_eq!(
        render_state.layer_style.headline_emphasized,
        runtime_render_state.shell_scene.headline_emphasized
    );
    assert_eq!(
        render_state.layer_style.edge_actions_visible,
        runtime_render_state.shell_scene.edge_actions_visible
    );
    assert!(
        renderer
            .last_pointer_regions
            .iter()
            .any(|region| matches!(region.kind, NativePanelPointerRegionKind::CardsContainer))
    );
    let command_bundle = renderer
        .scene_cache
        .last_render_command_bundle
        .as_ref()
        .expect("cached render command bundle");
    assert_eq!(
        command_bundle.compact_bar.frame,
        command_bundle.layout.pill_frame
    );
    assert_eq!(
        command_bundle.compact_bar.headline.text,
        scene.compact_bar.headline.text
    );
    assert_eq!(
        command_bundle.card_stack.frame,
        command_bundle.layout.cards_frame
    );
    assert_eq!(command_bundle.card_stack.cards.len(), scene.cards.len());
    assert_eq!(command_bundle.mascot.pose, scene.mascot_pose);
}

#[test]
fn windows_renderer_resolves_pointer_regions_from_shared_scene_and_layout() {
    let mut panel_state = PanelState::default();
    let bundle = test_runtime_scene_bundle(
        &mut panel_state,
        &pending_permission_snapshot("session-1"),
        &PanelSceneBuildInput::default(),
    );
    let scene = bundle.scene;
    let runtime_render_state = bundle.runtime_render_state;
    let mut renderer = super::WindowsNativePanelRenderer::default();

    renderer.update_screen_frame(Some(PanelRect {
        x: 100.0,
        y: 50.0,
        width: 1000.0,
        height: 700.0,
    }));
    renderer
        .render_scene(&scene, runtime_render_state)
        .expect("render scene");
    renderer
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
        .expect("apply descriptor");

    assert!(
        renderer
            .last_pointer_regions
            .iter()
            .any(|region| matches!(region.kind, NativePanelPointerRegionKind::CompactBar))
    );
    assert!(
        renderer
            .last_pointer_regions
            .iter()
            .any(|region| matches!(region.kind, NativePanelPointerRegionKind::CardsContainer))
    );
    assert!(renderer.last_pointer_regions.iter().any(|region| matches!(
        &region.kind,
        NativePanelPointerRegionKind::HitTarget(target)
            if target.action == PanelHitAction::FocusSession
                && target.value == "session-1"
    )));
}

#[test]
fn windows_renderer_caches_complete_render_commands() {
    let mut expanded_state = PanelState {
        expanded: true,
        ..PanelState::default()
    };
    let expanded_bundle = test_runtime_scene_bundle(
        &mut expanded_state,
        &snapshot(),
        &PanelSceneBuildInput::default(),
    );
    let mut renderer = super::WindowsNativePanelRenderer::default();
    renderer
        .render_scene(&expanded_bundle.scene, expanded_bundle.runtime_render_state)
        .expect("render expanded scene");
    renderer
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
        .expect("apply expanded descriptor");

    let expanded_command = renderer
        .scene_cache
        .last_render_command_bundle
        .as_ref()
        .expect("expanded render command");
    let expanded_presentation = renderer
        .last_presentation_model
        .as_ref()
        .expect("expanded presentation model");
    assert!(expanded_command.compact_bar.actions_visible);
    assert_eq!(
        expanded_command.card_stack.cards.len(),
        expanded_bundle.scene.cards.len()
    );
    assert_eq!(
        expanded_command.mascot.pose,
        expanded_bundle.scene.mascot_pose
    );
    assert_eq!(expanded_command.action_buttons.len(), 2);
    assert!(expanded_command.glow.is_none());
    assert_eq!(
        expanded_presentation.panel_frame,
        expanded_command.layout.panel_frame
    );
    assert_eq!(
        expanded_presentation.compact_bar.frame,
        expanded_command.compact_bar.frame
    );

    let completion_state = PanelState {
        completion_badge_items: vec![CompletionBadgeItem {
            session_id: "session-1".to_string(),
            completed_at: Utc::now(),
            last_user_prompt: None,
            last_assistant_message: Some("Done".to_string()),
        }],
        ..PanelState::default()
    };
    let completion_scene = build_panel_scene(
        &completion_state,
        &snapshot(),
        &PanelSceneBuildInput::default(),
    );
    renderer
        .render_scene(&completion_scene, PanelRuntimeRenderState::default())
        .expect("render completion scene");
    renderer
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Close,
            canvas_height: 80.0,
            visible_height: 80.0,
            width_progress: 0.0,
            height_progress: 0.0,
            shoulder_progress: 0.0,
            drop_progress: 0.0,
            cards_progress: 0.0,
        })
        .expect("apply completion descriptor");

    let completion_command = renderer
        .scene_cache
        .last_render_command_bundle
        .as_ref()
        .expect("completion render command");
    assert!(completion_command.glow.is_some());
    assert!(
        renderer
            .last_presentation_model
            .as_ref()
            .and_then(|presentation| presentation.glow.as_ref())
            .is_some()
    );
    assert_eq!(
        completion_command.compact_bar.completion_count,
        completion_scene.compact_bar.completion_count
    );
}

#[test]
fn windows_runtime_scene_state_bridge_syncs_current_bundle_and_pointer_regions() {
    let input = runtime_input_descriptor();
    let mut runtime = super::WindowsNativePanelRuntime {
        panel_state: PanelState {
            expanded: true,
            ..PanelState::default()
        },
        ..Default::default()
    };
    let bundle =
        test_runtime_scene_bundle(&mut runtime.panel_state, &snapshot(), &input.scene_input);

    runtime
        .host
        .renderer
        .render_scene(&bundle.scene, bundle.runtime_render_state)
        .expect("render scene");
    runtime
        .host
        .renderer
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
        .expect("apply descriptor");

    let rendered_bundle = runtime
        .host
        .renderer
        .scene_cache
        .last_render_command_bundle
        .clone()
        .expect("rendered bundle");
    runtime.scene_cache.last_snapshot = Some(snapshot());
    runtime.host.renderer.last_pointer_regions.clear();

    cache_render_command_bundle_for_state_bridge_with_input(&mut runtime, &input, &rendered_bundle);

    let current_bundle =
        resolve_current_native_panel_render_command_bundle_for_state_bridge_with_input(
            &runtime, &input,
        )
        .expect("current bundle");

    assert_eq!(
        runtime.host.renderer.last_pointer_regions.len(),
        rendered_bundle.pointer_regions.len()
    );
    assert_eq!(
        current_bundle.compact_bar.headline.text,
        rendered_bundle.compact_bar.headline.text
    );
}

#[test]
fn windows_host_presents_renderer_state_into_window() {
    let mut host = super::WindowsNativePanelHost::default();
    let scene = build_panel_scene(
        &PanelState {
            expanded: true,
            ..PanelState::default()
        },
        &snapshot(),
        &PanelSceneBuildInput::default(),
    );
    host.renderer.last_window_state = Some(NativePanelHostWindowState {
        frame: Some(PanelRect {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 120.0,
        }),
        visible: true,
        preferred_display_index: 1,
    });
    host.renderer.last_pointer_regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 20.0,
            width: 80.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];
    host.renderer.scene_cache.last_scene = Some(scene.clone());

    host.present_renderer_state()
        .expect("present renderer state");

    assert!(host.presenter.redraw_requested());
    let draw_frame = host.take_pending_draw_frame().expect("pending draw frame");

    assert_eq!(
        host.window.presented_window_state,
        host.renderer.last_window_state
    );
    assert_eq!(
        host.window.pointer_regions(&[]),
        &[NativePanelPointerRegion {
            frame: PanelRect {
                x: 10.0,
                y: 80.0,
                width: 80.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }]
    );
    assert_eq!(
        host.window
            .presented_presentation_model
            .as_ref()
            .map(|presentation| presentation.compact_bar.headline.text.as_str()),
        Some(scene.compact_bar.headline.text.as_str())
    );
    assert_eq!(
        draw_frame.window_state,
        host.window.presented_window_state.unwrap()
    );
    assert_eq!(
        draw_frame
            .presentation_model
            .as_ref()
            .map(|presentation| presentation.compact_bar.headline.text.as_str()),
        Some(scene.compact_bar.headline.text.as_str())
    );
    assert!(!host.presenter.redraw_requested());
    assert!(host.take_pending_draw_frame().is_none());
}

#[test]
fn windows_runtime_user_hide_blocks_snapshot_refresh_until_show() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let input = runtime_input_descriptor();

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    runtime.hide_panel().expect("hide panel");
    runtime.pump_platform_loop().expect("pump hide");

    assert!(runtime.user_hidden);
    assert_eq!(runtime.platform_loop.last_visible, Some(false));

    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("hidden snapshot sync");
    runtime.pump_platform_loop().expect("pump hidden refresh");

    assert!(runtime.user_hidden);
    assert_eq!(runtime.platform_loop.last_visible, Some(false));

    runtime.create_panel().expect("show panel");
    runtime.pump_platform_loop().expect("pump show");

    assert!(!runtime.user_hidden);
    assert_eq!(runtime.platform_loop.last_visible, Some(true));
}

#[test]
fn windows_host_shell_can_consume_presenter_frame() {
    let mut host = super::WindowsNativePanelHost::default();
    host.presenter.present(WindowsNativePanelDrawFrame {
        window_state: NativePanelHostWindowState {
            frame: Some(PanelRect {
                x: 12.0,
                y: 24.0,
                width: 256.0,
                height: 96.0,
            }),
            visible: true,
            preferred_display_index: 0,
        },
        pointer_regions: Vec::new(),
        presentation_model: None,
    });

    assert!(host.consume_presenter_into_shell());
    assert_eq!(host.shell.redraw_requests(), 1);
    assert_eq!(
        host.shell
            .last_frame()
            .and_then(|frame| frame.window_state.frame)
            .map(|frame| frame.width),
        Some(256.0)
    );
    assert!(host.shell.pending_paint_job().is_some());
    assert!(!host.consume_presenter_into_shell());
}

#[test]
fn windows_host_shell_paints_pending_presenter_frame() {
    let mut host = super::WindowsNativePanelHost::default();
    host.presenter.present(WindowsNativePanelDrawFrame {
        window_state: NativePanelHostWindowState {
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

    let result = host.consume_presenter_into_shell_result();

    assert!(result.redraw_requested);
    assert!(result.paint_queued);
    assert!(host.shell.pending_paint_job().is_some());
    let paint_job = host.shell.paint_next_frame().expect("paint job");
    assert_eq!(
        paint_job.display_mode,
        NativePanelVisualDisplayMode::Compact
    );
    assert_eq!(host.shell.paint_pass_count(), 1);
    assert_eq!(
        host.shell
            .last_painted_job()
            .map(|job| job.panel_frame.width),
        Some(300.0)
    );
}

#[test]
fn windows_runtime_records_pointer_input_on_window_event_path() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    let mut handler = RecordingEventHandler::default();

    let _ = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Move(PanelPoint { x: 8.0, y: 16.0 }),
            std::time::Instant::now(),
            &NativePanelRuntimeInputDescriptor {
                scene_input: PanelSceneBuildInput::default(),
                screen_frame: None,
            },
            &mut handler,
        )
        .expect("handle move");

    assert_eq!(
        runtime.host.shell.last_pointer_input(),
        Some(NativePanelPointerInput::Move(PanelPoint {
            x: 8.0,
            y: 16.0
        }))
    );
}

#[test]
fn windows_runtime_pointer_move_syncs_mouse_passthrough_state() {
    let mut runtime = super::WindowsNativePanelRuntime {
        ignores_mouse_events: true,
        ..Default::default()
    };
    runtime.host.presenter.present(shell_draw_frame(
        vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        false,
    ));
    let _ = runtime.host.consume_presenter_into_shell_result();
    let mut handler = RecordingEventHandler::default();

    let _ = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Move(PanelPoint { x: 20.0, y: 20.0 }),
            Instant::now(),
            &NativePanelRuntimeInputDescriptor {
                scene_input: PanelSceneBuildInput::default(),
                screen_frame: None,
            },
            &mut handler,
        )
        .expect("handle move");

    assert!(!runtime.ignores_mouse_events);
    assert_eq!(runtime.host.shell.last_ignores_mouse_events(), Some(false));
    assert!(
        runtime
            .host
            .take_pending_shell_commands()
            .into_iter()
            .any(|command| matches!(
                command,
                super::window_shell::WindowsNativePanelShellCommand::SyncMouseEventPassthrough(
                    false
                )
            ))
    );
}

#[test]
fn windows_runtime_pointer_leave_syncs_mouse_passthrough_state() {
    let mut runtime = super::WindowsNativePanelRuntime {
        ignores_mouse_events: false,
        ..Default::default()
    };
    let mut handler = RecordingEventHandler::default();

    let _ = runtime
        .handle_pointer_input_with_handler(
            NativePanelPointerInput::Leave,
            Instant::now(),
            &NativePanelRuntimeInputDescriptor {
                scene_input: PanelSceneBuildInput::default(),
                screen_frame: None,
            },
            &mut handler,
        )
        .expect("handle leave");

    assert!(runtime.ignores_mouse_events);
    assert_eq!(runtime.host.shell.last_ignores_mouse_events(), Some(true));
    assert!(
        runtime
            .host
            .take_pending_shell_commands()
            .into_iter()
            .any(|command| matches!(
                command,
                super::window_shell::WindowsNativePanelShellCommand::SyncMouseEventPassthrough(
                    true
                )
            ))
    );
}

#[test]
fn windows_runtime_host_polling_interaction_updates_passthrough_state() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime {
        ignores_mouse_events: true,
        ..Default::default()
    };
    runtime.host.presenter.present(shell_draw_frame(
        vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 110.0,
                y: 60.0,
                width: 100.0,
                height: 30.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        false,
    ));
    let present = runtime.host.consume_presenter_into_shell_result();

    assert!(present.display_updated);

    let interaction = runtime
        .sync_host_polling_interaction(PanelPoint { x: 120.0, y: 70.0 }, false, now)
        .expect("polling interaction");

    assert!(interaction.interactive_inside);
    assert_eq!(interaction.click_command, PanelInteractionCommand::None);
    assert!(!interaction.next_ignores_mouse_events);
    assert!(interaction.sync_mouse_event_passthrough);
    assert_eq!(
        interaction.host_behavior.commands,
        vec![
            crate::native_panel_renderer::facade::interaction::NativePanelHostBehaviorCommand::SetMouseEventPassthrough {
                ignores_mouse_events: false,
            }
        ]
    );
    assert!(!runtime.ignores_mouse_events);
    assert_eq!(runtime.host.shell.last_ignores_mouse_events(), Some(false));
    assert!(
        runtime
            .host
            .take_pending_shell_commands()
            .into_iter()
            .any(|command| matches!(
                command,
                super::window_shell::WindowsNativePanelShellCommand::SyncMouseEventPassthrough(
                    false
                )
            ))
    );
}

#[test]
fn windows_runtime_host_polling_interaction_marks_completion_viewed_on_hover_expand() {
    let now = Instant::now();
    let input = runtime_input_descriptor();
    let mut runtime = super::WindowsNativePanelRuntime {
        panel_state: PanelState {
            pointer_inside_since: Some(
                now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100),
            ),
            completion_badge_items: vec![CompletionBadgeItem {
                session_id: "session-1".to_string(),
                completed_at: Utc::now(),
                last_user_prompt: None,
                last_assistant_message: Some("Done".to_string()),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    runtime
        .sync_snapshot_bundle(&snapshot(), &input)
        .expect("sync snapshot");
    runtime.panel_state.completion_badge_items = vec![CompletionBadgeItem {
        session_id: "session-1".to_string(),
        completed_at: Utc::now(),
        last_user_prompt: None,
        last_assistant_message: Some("Done".to_string()),
    }];
    runtime.host.presenter.present(shell_draw_frame(
        vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 110.0,
                y: 60.0,
                width: 100.0,
                height: 30.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        false,
    ));
    let present = runtime.host.consume_presenter_into_shell_result();

    assert!(present.display_updated);

    let interaction = runtime
        .sync_host_polling_interaction_and_refresh(
            PanelPoint { x: 120.0, y: 70.0 },
            false,
            now,
            &input,
        )
        .expect("polling interaction")
        .expect("polling facts");

    assert!(interaction.interactive_inside);
    assert_eq!(
        interaction.transition_request,
        Some(NativePanelTransitionRequest::Open)
    );
    assert!(runtime.panel_state.expanded);
    assert!(runtime.panel_state.completion_badge_items.is_empty());
    assert_eq!(
        runtime.last_transition_request,
        Some(NativePanelTransitionRequest::Open)
    );
}

#[test]
fn windows_runtime_hover_expanded_panel_switches_to_new_completion_status_message() {
    let now = Instant::now();
    let input = runtime_input_descriptor();
    let mut running_snapshot = sessions_snapshot(1);
    running_snapshot.sessions[0].status = "Running".to_string();
    running_snapshot.sessions[0].last_assistant_message = Some("Working".to_string());
    let mut completed_snapshot = running_snapshot.clone();
    completed_snapshot.sessions[0].status = "Idle".to_string();
    completed_snapshot.sessions[0].last_activity = Utc::now();
    completed_snapshot.sessions[0].last_assistant_message = Some("Done".to_string());
    let mut runtime = super::WindowsNativePanelRuntime {
        panel_state: PanelState {
            expanded: true,
            pointer_inside_since: Some(
                now - Duration::from_millis(crate::native_panel_core::HOVER_DELAY_MS + 100),
            ),
            surface_mode: ExpandedSurface::Default,
            ..Default::default()
        },
        ..Default::default()
    };

    runtime
        .sync_snapshot_bundle(&running_snapshot, &input)
        .expect("seed running snapshot");
    runtime
        .sync_snapshot_bundle(&completed_snapshot, &input)
        .expect("sync completion snapshot");

    assert!(runtime.panel_state.expanded);
    assert!(runtime.panel_state.status_auto_expanded);
    assert_eq!(runtime.panel_state.surface_mode, ExpandedSurface::Status);
    let presentation = runtime
        .host
        .renderer
        .latest_scene_presentation_model()
        .expect("completion status presentation");
    assert_eq!(presentation.card_stack.surface, ExpandedSurface::Status);
    assert!(
        presentation
            .card_stack
            .cards
            .iter()
            .any(|card| matches!(card, SceneCard::StatusCompletion { .. }))
    );
    assert_eq!(presentation.mascot.pose, SceneMascotPose::Complete);
}

#[test]
fn windows_runtime_host_polling_interaction_resolves_hit_target_click() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime {
        panel_state: PanelState {
            expanded: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let target = PanelHitTarget {
        action: PanelHitAction::FocusSession,
        value: "session-1".to_string(),
    };
    runtime.host.presenter.present(shell_draw_frame(
        vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 110.0,
                y: 90.0,
                width: 200.0,
                height: 50.0,
            },
            kind: NativePanelPointerRegionKind::HitTarget(target.clone()),
        }],
        true,
    ));
    let present = runtime.host.consume_presenter_into_shell_result();

    assert!(present.display_updated);

    let interaction = runtime
        .sync_host_polling_interaction(PanelPoint { x: 140.0, y: 110.0 }, true, now)
        .expect("polling interaction");

    assert!(interaction.interactive_inside);
    assert_eq!(
        interaction.click_command,
        PanelInteractionCommand::HitTarget(target)
    );
    assert!(!interaction.next_ignores_mouse_events);
    assert!(!interaction.sync_mouse_event_passthrough);
    assert!(runtime.primary_pointer_down);
    assert!(
        runtime
            .last_focus_click
            .as_ref()
            .is_some_and(|(session_id, _)| session_id == "session-1")
    );
    assert!(
        !runtime
            .host
            .take_pending_shell_commands()
            .into_iter()
            .any(|command| matches!(
                command,
                super::window_shell::WindowsNativePanelShellCommand::SyncMouseEventPassthrough(_)
            ))
    );
}
