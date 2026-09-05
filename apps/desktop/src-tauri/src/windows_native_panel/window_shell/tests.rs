use super::{
    WINDOWS_WM_LBUTTONUP, WINDOWS_WM_MOUSELEAVE, WINDOWS_WM_MOUSEMOVE,
    WindowsNativePanelShellCommand, WindowsNativePanelWindowHandle, WindowsNativePanelWindowShell,
    windows_client_hover_fallback_frames,
};
use crate::native_panel_renderer::facade::shell::NativePanelPlatformWindowHandleAdapter;
use crate::{
    native_panel_core::{PanelPoint, PanelRect},
    native_panel_renderer::facade::{
        descriptor::{
            NativePanelHostWindowState, NativePanelPointerInput, NativePanelPointerRegion,
            NativePanelPointerRegionKind,
        },
        presentation::{
            NativePanelActionButtonsPresentation, NativePanelCardStackPresentation,
            NativePanelCompactBarPresentation, NativePanelGlowPresentation,
            NativePanelMascotPresentation, NativePanelPresentationMetrics,
            NativePanelPresentationModel, NativePanelShellPresentation,
            NativePanelVisualDisplayMode, NativePanelVisualPlanInput,
        },
        shell::NativePanelHostShellLifecycle,
    },
    windows_native_panel::{
        draw_presenter::WindowsNativePanelDrawPresenter, host_window::WindowsNativePanelDrawFrame,
    },
};

fn visible_window_state() -> NativePanelHostWindowState {
    NativePanelHostWindowState {
        screen_scale_factor: None,
        screen_physical_frame: None,
        frame: Some(PanelRect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 80.0,
        }),
        visible: true,
        preferred_display_index: 0,
    }
}

fn presentation_with_mascot(
    pose: crate::native_panel_scene::SceneMascotPose,
    shell_visible: bool,
) -> NativePanelPresentationModel {
    NativePanelPresentationModel {
        panel_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 80.0,
        },
        content_frame: PanelRect {
            x: 10.0,
            y: 40.0,
            width: 300.0,
            height: 30.0,
        },
        shell: NativePanelShellPresentation {
            surface: crate::native_panel_core::ExpandedSurface::Default,
            frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 80.0,
            },
            visible: shell_visible,
            separator_visibility: if shell_visible { 1.0 } else { 0.0 },
            shared_visible: true,
            chrome_transition_progress: if shell_visible { 1.0 } else { 0.0 },
        },
        compact_bar: NativePanelCompactBarPresentation {
            frame: PanelRect {
                x: 10.0,
                y: 0.0,
                width: 300.0,
                height: 37.0,
            },
            left_shoulder_frame: PanelRect {
                x: 4.0,
                y: 30.0,
                width: 6.0,
                height: 6.0,
            },
            right_shoulder_frame: PanelRect {
                x: 310.0,
                y: 30.0,
                width: 6.0,
                height: 6.0,
            },
            shoulder_progress: 0.0,
            headline: crate::native_panel_scene::SceneText {
                text: "Codex ready".to_string(),
                emphasized: false,
            },
            active_count: "1".to_string(),
            total_count: "1".to_string(),
            completion_count: 0,
            headline_emphasized: false,
            actions_visible: false,
        },
        card_stack: NativePanelCardStackPresentation {
            frame: PanelRect {
                x: 10.0,
                y: 40.0,
                width: 300.0,
                height: 30.0,
            },
            surface: crate::native_panel_core::ExpandedSurface::Default,
            cards: Vec::new(),
            content_height: 0.0,
            body_height: 0.0,
            visible: false,
        },
        mascot: NativePanelMascotPresentation {
            pose,
            debug_mode_enabled: false,
        },
        glow: None,
        action_buttons: NativePanelActionButtonsPresentation {
            visible: false,
            buttons: Vec::new(),
        },
        metrics: NativePanelPresentationMetrics {
            expanded_content_height: 0.0,
            expanded_body_height: 0.0,
        },
    }
}

#[test]
fn shell_window_handle_helpers_roundtrip_handle_presence() {
    let mut handle = WindowsNativePanelWindowHandle::default();

    assert_eq!(handle.raw_window_handle(), None);

    handle.set_raw_window_handle(Some(99));

    assert_eq!(handle.raw_window_handle(), Some(99));
}

#[test]
fn shell_proxies_raw_window_handle_adapter() {
    let mut shell = WindowsNativePanelWindowShell::default();

    assert!(!shell.has_raw_window_handle());

    shell.set_raw_window_handle(Some(123));

    assert_eq!(shell.raw_window_handle(), Some(123));
    assert!(shell.has_raw_window_handle());
}

#[test]
fn shell_decodes_pointer_move_message_from_lparam() {
    let shell = WindowsNativePanelWindowShell::default();
    let message = shell.decode_window_message(WINDOWS_WM_MOUSEMOVE, 0x001E_000Aisize);

    assert_eq!(
        message,
        Some(NativePanelPointerInput::Move(PanelPoint {
            x: 10.0,
            y: 30.0,
        }))
    );
}

#[test]
fn shell_decodes_pointer_click_message_from_signed_lparam() {
    let shell = WindowsNativePanelWindowShell::default();
    let message = shell.decode_window_message(WINDOWS_WM_LBUTTONUP, 0xFFEC_FFF6u32 as isize);

    assert_eq!(
        message,
        Some(NativePanelPointerInput::Click(PanelPoint {
            x: -10.0,
            y: -20.0,
        }))
    );
}

#[test]
fn shell_decodes_pointer_leave_message() {
    let shell = WindowsNativePanelWindowShell::default();
    let message = shell.decode_window_message(WINDOWS_WM_MOUSELEAVE, 0x0000_0000isize);

    assert_eq!(message, Some(NativePanelPointerInput::Leave));
}

#[test]
fn shell_records_last_pointer_input() {
    let mut shell = WindowsNativePanelWindowShell::default();

    shell.record_pointer_input(NativePanelPointerInput::Leave);

    assert_eq!(
        shell.last_pointer_input(),
        Some(NativePanelPointerInput::Leave)
    );
}

#[test]
fn shell_consumes_presenter_redraw_frame() {
    let mut presenter = WindowsNativePanelDrawPresenter::default();
    let mut shell = WindowsNativePanelWindowShell::default();
    presenter.present(WindowsNativePanelDrawFrame {
        window_state: NativePanelHostWindowState {
            screen_scale_factor: None,
            screen_physical_frame: None,
            frame: Some(PanelRect {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 80.0,
            }),
            visible: true,
            preferred_display_index: 0,
        },
        pointer_regions: Vec::new(),
        presentation_model: None,
    });

    let result = shell.consume_presenter(&mut presenter);

    assert!(result.display_updated);
    assert!(result.paint_queued);
    assert!(result.redraw_requested);
    assert_eq!(shell.lifecycle(), NativePanelHostShellLifecycle::Created);
    assert_eq!(shell.redraw_requests(), 1);
    assert_eq!(
        shell
            .last_frame()
            .and_then(|frame| frame.window_state.frame)
            .map(|frame| frame.width),
        Some(320.0)
    );
    let paint_job = shell.paint_next_frame().expect("paint job");
    assert_eq!(
        paint_job.display_mode,
        NativePanelVisualDisplayMode::Compact
    );
    assert_eq!(shell.consume_presenter(&mut presenter), Default::default());
}

#[test]
fn shell_lifecycle_tracks_create_show_hide_destroy() {
    let mut shell = WindowsNativePanelWindowShell::default();

    shell.create();
    assert_eq!(shell.lifecycle(), NativePanelHostShellLifecycle::Created);

    shell.show();
    assert_eq!(shell.lifecycle(), NativePanelHostShellLifecycle::Visible);

    shell.hide();
    assert_eq!(shell.lifecycle(), NativePanelHostShellLifecycle::Hidden);

    shell.destroy();
    assert_eq!(shell.lifecycle(), NativePanelHostShellLifecycle::Detached);
}

#[test]
fn shell_tracks_window_state_and_platform_loop() {
    let mut shell = WindowsNativePanelWindowShell::default();
    let state = NativePanelHostWindowState {
        screen_scale_factor: None,
        screen_physical_frame: None,
        frame: Some(PanelRect {
            x: 8.0,
            y: 16.0,
            width: 320.0,
            height: 96.0,
        }),
        visible: true,
        preferred_display_index: 2,
    };

    shell.sync_window_state(state);
    shell.record_platform_loop_spawn();

    assert_eq!(shell.last_window_state(), Some(state));
    assert!(shell.platform_loop_started());
    assert_eq!(shell.platform_loop_spawn_count(), 1);
}

#[test]
fn shell_emits_lifecycle_and_redraw_commands() {
    let mut shell = WindowsNativePanelWindowShell::default();
    let state = NativePanelHostWindowState {
        screen_scale_factor: None,
        screen_physical_frame: None,
        frame: Some(PanelRect {
            x: 4.0,
            y: 6.0,
            width: 220.0,
            height: 72.0,
        }),
        visible: true,
        preferred_display_index: 1,
    };

    shell.create();
    shell.show();
    shell.sync_window_state(state);
    shell.request_redraw();
    shell.hide();

    assert_eq!(
        shell.take_pending_commands(),
        vec![
            WindowsNativePanelShellCommand::Create,
            WindowsNativePanelShellCommand::Show,
            WindowsNativePanelShellCommand::SyncWindowState(state),
            WindowsNativePanelShellCommand::RequestRedraw,
            WindowsNativePanelShellCommand::Hide,
        ]
    );
}

#[test]
fn shell_emits_mouse_passthrough_command_only_when_state_changes() {
    let mut shell = WindowsNativePanelWindowShell::default();

    shell.sync_mouse_event_passthrough(true);
    shell.sync_mouse_event_passthrough(true);
    shell.sync_mouse_event_passthrough(false);

    assert_eq!(shell.last_ignores_mouse_events(), Some(false));
    assert_eq!(
        shell.take_pending_commands(),
        vec![
            WindowsNativePanelShellCommand::SyncMouseEventPassthrough(true),
            WindowsNativePanelShellCommand::SyncMouseEventPassthrough(false),
        ]
    );
}

#[test]
fn shell_builds_display_snapshot_from_presenter_frame() {
    let mut presenter = WindowsNativePanelDrawPresenter::default();
    let mut shell = WindowsNativePanelWindowShell::default();
    presenter.present(WindowsNativePanelDrawFrame {
        window_state: NativePanelHostWindowState {
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
        pointer_regions: Vec::new(),
        presentation_model: Some(NativePanelPresentationModel {
            panel_frame: PanelRect {
                x: 100.0,
                y: 50.0,
                width: 320.0,
                height: 120.0,
            },
            content_frame: PanelRect {
                x: 110.0,
                y: 90.0,
                width: 300.0,
                height: 70.0,
            },
            shell: NativePanelShellPresentation {
                surface: crate::native_panel_core::ExpandedSurface::Status,
                frame: PanelRect {
                    x: 100.0,
                    y: 70.0,
                    width: 320.0,
                    height: 100.0,
                },
                visible: true,
                separator_visibility: 0.8,
                shared_visible: true,
                chrome_transition_progress: 1.0,
            },
            compact_bar: NativePanelCompactBarPresentation {
                frame: PanelRect {
                    x: 110.0,
                    y: 60.0,
                    width: 300.0,
                    height: 24.0,
                },
                left_shoulder_frame: PanelRect {
                    x: 104.0,
                    y: 78.0,
                    width: 6.0,
                    height: 6.0,
                },
                right_shoulder_frame: PanelRect {
                    x: 410.0,
                    y: 78.0,
                    width: 6.0,
                    height: 6.0,
                },
                shoulder_progress: 0.0,
                headline: crate::native_panel_scene::SceneText {
                    text: "等待审批".to_string(),
                    emphasized: true,
                },
                active_count: "23".to_string(),
                total_count: "2".to_string(),
                completion_count: 3,
                headline_emphasized: true,
                actions_visible: true,
            },
            card_stack: NativePanelCardStackPresentation {
                frame: PanelRect {
                    x: 110.0,
                    y: 90.0,
                    width: 300.0,
                    height: 70.0,
                },
                surface: crate::native_panel_core::ExpandedSurface::Status,
                cards: vec![crate::native_panel_scene::SceneCard::Empty],
                content_height: 70.0,
                body_height: 70.0,
                visible: true,
            },
            mascot: NativePanelMascotPresentation {
                pose: crate::native_panel_scene::SceneMascotPose::Complete,
                debug_mode_enabled: false,
            },
            glow: Some(NativePanelGlowPresentation {
                glow: crate::native_panel_scene::SceneGlow {
                    style: crate::native_panel_scene::SceneGlowStyle::Completion,
                    opacity: 0.8,
                },
            }),
            action_buttons: NativePanelActionButtonsPresentation {
                visible: true,
                buttons: Vec::new(),
            },
            metrics: NativePanelPresentationMetrics {
                expanded_content_height: 70.0,
                expanded_body_height: 70.0,
            },
        }),
    });

    let result = shell.consume_presenter(&mut presenter);

    assert!(result.redraw_requested);
    let snapshot = shell.display_snapshot().expect("display snapshot");
    assert_eq!(
        snapshot.display_mode,
        NativePanelVisualDisplayMode::Expanded
    );
    assert_eq!(
        snapshot.visual_input.surface,
        crate::native_panel_core::ExpandedSurface::Status
    );
    assert_eq!(snapshot.visual_input.headline_text, "等待审批");
    assert!(snapshot.visual_input.headline_emphasized);
    assert!(snapshot.visual_input.cards_visible);
    assert_eq!(snapshot.visual_input.card_count, 1);
    assert_eq!(snapshot.visual_input.cards[0].title, "暂无活动会话");
    assert!(snapshot.visual_input.glow_visible);
    assert!(snapshot.visual_input.action_buttons_visible);
    assert_eq!(snapshot.visual_input.completion_count, 3);
    assert_eq!(
        snapshot.visual_input.mascot_pose,
        crate::native_panel_scene::SceneMascotPose::Complete
    );

    assert!(!shell.active_count_marquee_needs_refresh());
    assert!(shell.mascot_animation_needs_refresh());

    shell.hide();

    assert!(!shell.active_count_marquee_needs_refresh());
    assert!(!shell.mascot_animation_needs_refresh());
}

#[test]
fn shell_refreshes_idle_mascot_into_sleepy_after_shared_delay() {
    let mut presenter = WindowsNativePanelDrawPresenter::default();
    let mut shell = WindowsNativePanelWindowShell::default();
    presenter.present(WindowsNativePanelDrawFrame {
        window_state: visible_window_state(),
        pointer_regions: Vec::new(),
        presentation_model: None,
    });
    shell.consume_presenter(&mut presenter);

    assert!(shell.refresh_mascot_animation(0));
    assert!(shell.refresh_mascot_animation(
        crate::native_panel_core::MASCOT_IDLE_LONG_SECONDS as u128 * 1000 + 1
    ));

    let paint_job = shell.paint_next_frame().expect("paint job");
    assert_eq!(
        paint_job.mascot_pose,
        crate::native_panel_scene::SceneMascotPose::Sleepy
    );
}

#[test]
fn shell_plays_wake_angry_after_sleepy_when_mascot_becomes_active() {
    let mut presenter = WindowsNativePanelDrawPresenter::default();
    let mut shell = WindowsNativePanelWindowShell::default();
    presenter.present(WindowsNativePanelDrawFrame {
        window_state: visible_window_state(),
        pointer_regions: Vec::new(),
        presentation_model: None,
    });
    shell.consume_presenter(&mut presenter);
    shell.refresh_mascot_animation(0);
    shell.refresh_mascot_animation(
        crate::native_panel_core::MASCOT_IDLE_LONG_SECONDS as u128 * 1000 + 1,
    );

    presenter.present(WindowsNativePanelDrawFrame {
        window_state: visible_window_state(),
        pointer_regions: Vec::new(),
        presentation_model: Some(presentation_with_mascot(
            crate::native_panel_scene::SceneMascotPose::Running,
            false,
        )),
    });
    shell.consume_presenter(&mut presenter);

    assert_eq!(
        shell
            .display_snapshot()
            .expect("display snapshot")
            .visual_input
            .mascot_pose,
        crate::native_panel_scene::SceneMascotPose::WakeAngry
    );
    let wake_started_at = crate::native_panel_core::MASCOT_IDLE_LONG_SECONDS as u128 * 1000 + 1;
    assert!(shell.refresh_mascot_animation(wake_started_at + 400));
    assert_eq!(
        shell.paint_next_frame().expect("wake paint").mascot_pose,
        crate::native_panel_scene::SceneMascotPose::WakeAngry
    );
    assert!(shell.refresh_mascot_animation(wake_started_at + 1_000));
    assert_eq!(
        shell
            .paint_next_frame()
            .expect("returned paint")
            .mascot_pose,
        crate::native_panel_scene::SceneMascotPose::Running
    );
}

#[test]
fn shell_keeps_sleepy_when_idle_compact_presenter_refreshes() {
    let mut presenter = WindowsNativePanelDrawPresenter::default();
    let mut shell = WindowsNativePanelWindowShell::default();
    presenter.present(WindowsNativePanelDrawFrame {
        window_state: visible_window_state(),
        pointer_regions: Vec::new(),
        presentation_model: None,
    });
    shell.consume_presenter(&mut presenter);
    shell.refresh_mascot_animation(0);
    shell.refresh_mascot_animation(
        crate::native_panel_core::MASCOT_IDLE_LONG_SECONDS as u128 * 1000 + 1,
    );

    presenter.present(WindowsNativePanelDrawFrame {
        window_state: visible_window_state(),
        pointer_regions: Vec::new(),
        presentation_model: None,
    });
    shell.consume_presenter(&mut presenter);

    assert!(shell.refresh_mascot_animation(
        crate::native_panel_core::MASCOT_IDLE_LONG_SECONDS as u128 * 1000 + 16
    ));
    assert_eq!(
        shell.paint_next_frame().expect("sleepy paint").mascot_pose,
        crate::native_panel_scene::SceneMascotPose::Sleepy
    );
}

#[test]
fn shell_preserves_mascot_elapsed_time_when_expanding_same_pose() {
    let mut presenter = WindowsNativePanelDrawPresenter::default();
    let mut shell = WindowsNativePanelWindowShell::default();
    presenter.present(WindowsNativePanelDrawFrame {
        window_state: visible_window_state(),
        pointer_regions: Vec::new(),
        presentation_model: Some(presentation_with_mascot(
            crate::native_panel_scene::SceneMascotPose::MessageBubble,
            false,
        )),
    });
    shell.consume_presenter(&mut presenter);
    assert!(shell.refresh_mascot_animation(500));

    presenter.present(WindowsNativePanelDrawFrame {
        window_state: visible_window_state(),
        pointer_regions: Vec::new(),
        presentation_model: Some(presentation_with_mascot(
            crate::native_panel_scene::SceneMascotPose::MessageBubble,
            true,
        )),
    });
    shell.consume_presenter(&mut presenter);

    assert_eq!(
        shell
            .display_snapshot()
            .expect("display snapshot")
            .visual_input
            .mascot_elapsed_ms,
        500
    );
}

#[test]
fn shell_pointer_and_hover_facts_follow_cached_frame() {
    let mut presenter = WindowsNativePanelDrawPresenter::default();
    let mut shell = WindowsNativePanelWindowShell::default();
    presenter.present(WindowsNativePanelDrawFrame {
        window_state: NativePanelHostWindowState {
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
        pointer_regions: vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 110.0,
                y: 60.0,
                width: 100.0,
                height: 30.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        presentation_model: None,
    });

    shell.consume_presenter(&mut presenter);

    let pointer_state = shell.pointer_state_at_point(PanelPoint { x: 120.0, y: 70.0 });
    assert!(pointer_state.inside);
    assert_eq!(
        shell.hover_inside_for_input(NativePanelPointerInput::Leave),
        Some(false)
    );
    assert_eq!(
        shell.hover_inside_for_input(NativePanelPointerInput::Move(PanelPoint {
            x: 120.0,
            y: 70.0
        })),
        Some(true)
    );

    let hover_frames = shell.hover_frames().expect("hover frames");
    assert!(hover_frames.interactive_pill_frame.width > 0.0);
    assert!(hover_frames.hover_pill_frame.width > 0.0);

    let facts = shell
        .polling_host_facts(PanelPoint { x: 120.0, y: 70.0 }, false, None)
        .expect("polling facts");
    assert_eq!(facts.pointer_regions.len(), 1);
}

#[test]
fn shell_hover_frames_use_windows_client_coordinates_and_stable_bubble_hover() {
    let zero = PanelRect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };
    let input = NativePanelVisualPlanInput {
        window_state: NativePanelHostWindowState {
            screen_scale_factor: None,
            screen_physical_frame: None,
            frame: Some(PanelRect {
                x: 510.0,
                y: 0.0,
                width: 420.0,
                height: 80.0,
            }),
            visible: true,
            preferred_display_index: 0,
        },
        display_mode: NativePanelVisualDisplayMode::Compact,
        surface: crate::native_panel_core::ExpandedSurface::Default,
        panel_frame: PanelRect {
            x: 510.0,
            y: 0.0,
            width: 420.0,
            height: 80.0,
        },
        compact_bar_frame: PanelRect {
            x: 83.5,
            y: 43.0,
            width: 253.0,
            height: 37.0,
        },
        left_shoulder_frame: zero,
        right_shoulder_frame: zero,
        shoulder_progress: 0.0,
        content_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 420.0,
            height: 80.0,
        },
        card_stack_frame: zero,
        card_stack_content_height: 0.0,
        shell_frame: zero,
        headline_text: String::new(),
        headline_emphasized: false,
        active_count: String::new(),
        active_count_elapsed_ms: 0,
        total_count: String::new(),
        separator_visibility: 0.0,
        chrome_transition_progress: 0.0,
        cards_visible: false,
        card_count: 0,
        cards: Vec::new(),
        glow_visible: false,
        glow_opacity: 0.0,
        action_buttons_visible: false,
        action_buttons: Vec::new(),
        completion_count: 0,
        mascot_elapsed_ms: 0,
        mascot_motion_frame: None,
        mascot_pose: crate::native_panel_scene::SceneMascotPose::Idle,
        mascot_debug_mode_enabled: false,
    };

    let frames = windows_client_hover_fallback_frames(
            &input,
            crate::native_panel_renderer::facade::interaction::resolve_native_panel_hover_fallback_frames(
                &input,
            ),
        );

    assert_eq!(
        frames.interactive_pill_frame,
        PanelRect {
            x: 83.5,
            y: 0.0,
            width: 253.0,
            height: 37.0,
        }
    );
    assert_eq!(frames.hover_pill_frame.x, 83.5);
    assert!(frames.hover_pill_frame.y < frames.interactive_pill_frame.y);
    assert!(frames.hover_pill_frame.height > frames.interactive_pill_frame.height);
}

#[test]
fn shell_does_not_schedule_marquee_redraws_for_numeric_counts() {
    for count in ["10", "22", "23", "64", "100"] {
        let mut presenter = WindowsNativePanelDrawPresenter::default();
        let mut shell = WindowsNativePanelWindowShell::default();
        let mut presentation =
            presentation_with_mascot(crate::native_panel_scene::SceneMascotPose::Idle, false);
        presentation.compact_bar.active_count = count.to_string();
        presenter.present(WindowsNativePanelDrawFrame {
            window_state: visible_window_state(),
            pointer_regions: Vec::new(),
            presentation_model: Some(presentation),
        });
        shell.consume_presenter(&mut presenter);
        assert!(!shell.active_count_marquee_needs_refresh());
        assert!(!shell.refresh_active_count_marquee(500));
    }
}
