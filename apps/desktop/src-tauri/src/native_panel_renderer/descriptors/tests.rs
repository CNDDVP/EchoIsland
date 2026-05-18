use echoisland_runtime::RuntimeSnapshot;

use crate::native_panel_core::{
    ExpandedSurface, HoverTransition, PanelGeometryMetrics, PanelHitAction, PanelHitTarget,
    PanelInteractionCommand, PanelLayout, PanelLayoutInput, PanelPoint, PanelRect, PanelState,
    resolve_panel_layout,
};
use crate::native_panel_renderer::card_visual_spec::{
    CardVisualRowSpec, card_visual_settings_row_layout,
};
use crate::native_panel_scene::{PanelSceneBuildInput, build_panel_scene};

use super::{
    NativePanelEdgeAction, NativePanelEdgeActionFrames, NativePanelPointerRegion,
    NativePanelPointerRegionInput, NativePanelPointerRegionKind,
};
use super::{
    NativePanelHostWindowDescriptor, NativePanelHostWindowDescriptorPatch,
    NativePanelHostWindowState, NativePanelPlatformEvent, NativePanelPointerInput,
    NativePanelPointerInputOutcome, NativePanelRuntimeCommand, NativePanelTimelineDescriptor,
    absolute_expanded_rect, absolute_panel_rect, native_panel_hit_target_at_point,
    native_panel_host_window_descriptor, native_panel_host_window_frame,
    native_panel_platform_event_at_point, native_panel_platform_event_for_interaction_command,
    native_panel_platform_event_for_pointer_input, native_panel_platform_event_for_pointer_region,
    native_panel_pointer_input_outcome, native_panel_pointer_inside_for_input,
    native_panel_pointer_inside_regions, native_panel_pointer_region_at_point,
    native_panel_pointer_state_at_point, native_panel_runtime_command_for_platform_event,
    native_panel_timeline_descriptor, native_panel_timeline_descriptor_for_animation,
    patch_native_panel_host_window_descriptor, queue_native_panel_platform_event_at_point,
    queue_native_panel_platform_event_for_pointer_region, resolve_native_panel_interaction_plan,
    resolve_native_panel_pointer_regions, sync_native_panel_host_window_screen_frame,
    sync_native_panel_host_window_shared_body_height, sync_native_panel_host_window_timeline,
    sync_native_panel_host_window_visibility,
};

fn pointer_test_layout() -> PanelLayout {
    resolve_panel_layout(PanelLayoutInput {
        screen_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        },
        metrics: PanelGeometryMetrics {
            compact_height: 38.0,
            compact_width: 253.0,
            expanded_width: 283.0,
            panel_width: 420.0,
        },
        canvas_height: 180.0,
        visible_height: 180.0,
        bar_progress: 1.0,
        height_progress: 1.0,
        drop_progress: 1.0,
        content_visibility: 1.0,
        collapsed_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
        drop_distance: crate::native_panel_core::PANEL_DROP_DISTANCE,
        content_top_gap: crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP,
        content_bottom_inset: crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET,
        cards_side_inset: crate::native_panel_core::EXPANDED_CARDS_SIDE_INSET,
        shoulder_size: crate::native_panel_core::COMPACT_SHOULDER_SIZE,
        separator_side_inset: crate::native_panel_core::EXPANDED_SEPARATOR_SIDE_INSET,
    })
}

fn pointer_test_scene() -> crate::native_panel_scene::PanelScene {
    let mut state = PanelState {
        expanded: true,
        surface_mode: ExpandedSurface::Default,
        ..PanelState::default()
    };
    state.transitioning = false;
    build_panel_scene(
        &state,
        &RuntimeSnapshot {
            status: "Idle".to_string(),
            primary_source: "claude".to_string(),
            active_session_count: 0,
            total_session_count: 0,
            pending_permission_count: 0,
            pending_question_count: 0,
            pending_permission: None,
            pending_question: None,
            pending_permissions: Vec::new(),
            pending_questions: Vec::new(),
            sessions: Vec::new(),
        },
        &PanelSceneBuildInput::default(),
    )
}

#[test]
fn interaction_command_maps_to_platform_event() {
    assert_eq!(
        native_panel_platform_event_for_interaction_command(&PanelInteractionCommand::HitTarget(
            PanelHitTarget {
                action: PanelHitAction::FocusSession,
                value: "session-1".to_string(),
            }
        )),
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        native_panel_platform_event_for_interaction_command(
            &PanelInteractionCommand::ToggleSettingsSurface
        ),
        Some(NativePanelPlatformEvent::ToggleSettingsSurface)
    );
    assert_eq!(
        native_panel_platform_event_for_interaction_command(
            &PanelInteractionCommand::QuitApplication
        ),
        Some(NativePanelPlatformEvent::QuitApplication)
    );
    assert_eq!(
        native_panel_platform_event_for_interaction_command(&PanelInteractionCommand::None),
        None
    );
}

#[test]
fn platform_event_maps_to_runtime_command() {
    assert_eq!(
        native_panel_runtime_command_for_platform_event(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )),
        NativePanelRuntimeCommand::FocusSession("session-1".to_string())
    );
    assert_eq!(
        native_panel_runtime_command_for_platform_event(
            NativePanelPlatformEvent::ToggleCompletionSound
        ),
        NativePanelRuntimeCommand::ToggleCompletionSound
    );
    assert_eq!(
        native_panel_runtime_command_for_platform_event(NativePanelPlatformEvent::OpenReleasePage),
        NativePanelRuntimeCommand::OpenReleasePage
    );
}

#[test]
fn pointer_input_outcome_projects_expected_variant_payload() {
    assert_eq!(
        NativePanelPointerInputOutcome::Hover(Some(HoverTransition::Expand))
            .into_hover_transition(),
        Some(HoverTransition::Expand)
    );
    assert_eq!(
        NativePanelPointerInputOutcome::Click(Some(NativePanelPlatformEvent::QuitApplication))
            .into_click_event(),
        Some(NativePanelPlatformEvent::QuitApplication)
    );
    assert_eq!(
        NativePanelPointerInputOutcome::Click(Some(NativePanelPlatformEvent::QuitApplication))
            .into_hover_transition(),
        None
    );
    assert_eq!(
        NativePanelPointerInputOutcome::Hover(Some(HoverTransition::Collapse)).into_click_event(),
        None
    );
}

#[test]
fn pointer_region_maps_to_platform_event() {
    let frame = PanelRect {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };
    assert_eq!(
        native_panel_platform_event_for_pointer_region(&NativePanelPointerRegion {
            frame,
            kind: NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Settings),
        }),
        Some(NativePanelPlatformEvent::ToggleSettingsSurface)
    );
    assert_eq!(
        native_panel_platform_event_for_pointer_region(&NativePanelPointerRegion {
            frame,
            kind: NativePanelPointerRegionKind::CompactBar,
        }),
        None
    );
}

#[test]
fn point_hit_testing_prefers_topmost_pointer_region() {
    let regions = vec![
        NativePanelPointerRegion {
            frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            kind: NativePanelPointerRegionKind::CardsContainer,
        },
        NativePanelPointerRegion {
            frame: PanelRect {
                x: 10.0,
                y: 10.0,
                width: 40.0,
                height: 40.0,
            },
            kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
                action: PanelHitAction::FocusSession,
                value: "session-1".to_string(),
            }),
        },
    ];
    let point = PanelPoint { x: 20.0, y: 20.0 };

    assert!(matches!(
        native_panel_pointer_region_at_point(&regions, point).map(|region| &region.kind),
        Some(NativePanelPointerRegionKind::HitTarget(target))
            if target.value == "session-1"
    ));
    assert_eq!(
        native_panel_platform_event_at_point(&regions, point),
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        native_panel_platform_event_at_point(&regions, PanelPoint { x: 80.0, y: 80.0 }),
        None
    );
    assert!(native_panel_pointer_inside_regions(
        &regions,
        PanelPoint { x: 80.0, y: 80.0 }
    ));
    assert!(!native_panel_pointer_inside_regions(
        &regions,
        PanelPoint { x: 180.0, y: 180.0 }
    ));
}

#[test]
fn queue_pointer_region_platform_event_pushes_focus_event() {
    let mut events = Vec::new();
    let region = NativePanelPointerRegion {
        frame: PanelRect {
            x: 10.0,
            y: 10.0,
            width: 40.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
    };

    let event = queue_native_panel_platform_event_for_pointer_region(&mut events, &region);

    assert_eq!(
        event,
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        events,
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
}

#[test]
fn queue_platform_event_at_point_pushes_hit_target_event() {
    let regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 10.0,
            y: 10.0,
            width: 40.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
    }];
    let mut events = Vec::new();

    let event = queue_native_panel_platform_event_at_point(
        &mut events,
        &regions,
        PanelPoint { x: 20.0, y: 20.0 },
    );

    assert_eq!(
        event,
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        events,
        vec![NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )]
    );
}

#[test]
fn pointer_input_resolves_hover_and_click_semantics() {
    let regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 10.0,
            y: 10.0,
            width: 40.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
    }];

    assert_eq!(
        native_panel_pointer_inside_for_input(
            &regions,
            NativePanelPointerInput::Move(PanelPoint { x: 20.0, y: 20.0 })
        ),
        Some(true)
    );
    assert_eq!(
        native_panel_pointer_inside_for_input(&regions, NativePanelPointerInput::Leave),
        Some(false)
    );
    assert_eq!(
        native_panel_pointer_inside_for_input(
            &regions,
            NativePanelPointerInput::Click(PanelPoint { x: 20.0, y: 20.0 })
        ),
        None
    );
    assert_eq!(
        native_panel_platform_event_for_pointer_input(
            &regions,
            NativePanelPointerInput::Click(PanelPoint { x: 20.0, y: 20.0 })
        ),
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        native_panel_platform_event_for_pointer_input(
            &regions,
            NativePanelPointerInput::Move(PanelPoint { x: 20.0, y: 20.0 })
        ),
        None
    );
    assert_eq!(
        native_panel_hit_target_at_point(&regions, PanelPoint { x: 20.0, y: 20.0 }),
        Some(PanelHitTarget {
            action: crate::native_panel_core::PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        })
    );
    assert_eq!(
        native_panel_pointer_input_outcome(
            &regions,
            NativePanelPointerInput::Move(PanelPoint { x: 20.0, y: 20.0 })
        ),
        NativePanelPointerInputOutcome::Hover(Some(HoverTransition::Expand))
    );
    assert_eq!(
        native_panel_pointer_input_outcome(&regions, NativePanelPointerInput::Leave),
        NativePanelPointerInputOutcome::Hover(Some(HoverTransition::Collapse))
    );
    assert_eq!(
        native_panel_pointer_input_outcome(
            &regions,
            NativePanelPointerInput::Click(PanelPoint { x: 20.0, y: 20.0 })
        ),
        NativePanelPointerInputOutcome::Click(Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        )))
    );
}

#[test]
fn pointer_state_at_point_collects_inside_event_and_hit_target() {
    let regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 10.0,
            y: 10.0,
            width: 40.0,
            height: 40.0,
        },
        kind: NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
    }];

    let state = native_panel_pointer_state_at_point(&regions, PanelPoint { x: 20.0, y: 20.0 });

    assert!(state.inside);
    assert_eq!(
        state.platform_event,
        Some(NativePanelPlatformEvent::FocusSession(
            "session-1".to_string()
        ))
    );
    assert_eq!(
        state.hit_target,
        Some(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        })
    );
}

#[test]
fn pointer_regions_use_default_edge_action_frames_without_platform_input() {
    let layout = pointer_test_layout();
    let scene = pointer_test_scene();

    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);
    let settings_frame = regions
        .iter()
        .find_map(|region| match region.kind {
            NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Settings) => {
                Some(region.frame)
            }
            _ => None,
        })
        .expect("settings pointer region");

    let pill = crate::native_panel_core::absolute_rect(layout.panel_frame, layout.pill_frame);
    let action_layout = crate::native_panel_core::resolve_compact_action_button_layout(pill);
    assert_eq!(
        settings_frame,
        PanelRect {
            x: action_layout.settings.x - 5.0,
            y: pill.y,
            width: action_layout.settings.width + 10.0,
            height: pill.height,
        }
    );
}

#[test]
fn interaction_plan_carries_shared_pointer_regions() {
    let layout = pointer_test_layout();
    let scene = pointer_test_scene();

    let plan = resolve_native_panel_interaction_plan(layout, &scene, None);
    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);

    assert_eq!(plan.pointer_regions, regions);
    assert!(
        plan.pointer_regions
            .iter()
            .any(|region| matches!(region.kind, NativePanelPointerRegionKind::CompactBar))
    );
}

#[test]
fn interaction_plan_resolves_pointer_semantics() {
    let layout = pointer_test_layout();
    let scene = pointer_test_scene();

    let plan = resolve_native_panel_interaction_plan(layout, &scene, None);
    let settings = plan
        .pointer_regions
        .iter()
        .find(|region| {
            matches!(
                region.kind,
                NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Settings)
            )
        })
        .expect("settings region")
        .frame;
    let point = PanelPoint {
        x: settings.x + settings.width / 2.0,
        y: settings.y + settings.height / 2.0,
    };

    assert_eq!(
        plan.platform_event_at_point(point),
        Some(NativePanelPlatformEvent::ToggleSettingsSurface)
    );
    assert_eq!(
        plan.input_outcome(NativePanelPointerInput::Click(point)),
        NativePanelPointerInputOutcome::Click(Some(
            NativePanelPlatformEvent::ToggleSettingsSurface
        ))
    );
    assert!(plan.pointer_state_at_point(point).inside);
    assert_eq!(
        plan.inside_for_input(NativePanelPointerInput::Move(point)),
        Some(true)
    );
    assert_eq!(
        plan.inside_for_input(NativePanelPointerInput::Leave),
        Some(false)
    );
    assert!(plan.hit_target_at_point(point).is_none());

    let mut events = Vec::new();
    assert_eq!(
        plan.queue_platform_event_at_point(&mut events, point),
        Some(NativePanelPlatformEvent::ToggleSettingsSurface)
    );
    assert_eq!(
        events,
        vec![NativePanelPlatformEvent::ToggleSettingsSurface]
    );
}

#[test]
fn pointer_regions_accept_platform_edge_action_frame_input() {
    let layout = pointer_test_layout();
    let scene = pointer_test_scene();
    let input = NativePanelPointerRegionInput {
        edge_action_frames: NativePanelEdgeActionFrames {
            settings_action: Some(PanelRect {
                x: 640.5,
                y: 824.0,
                width: 26.0,
                height: 26.0,
            }),
            quit_action: Some(PanelRect {
                x: 774.0,
                y: 824.0,
                width: 22.0,
                height: 22.0,
            }),
        },
    };

    let regions = resolve_native_panel_pointer_regions(layout, &scene, Some(input));

    assert_eq!(
        native_panel_platform_event_at_point(&regions, PanelPoint { x: 645.0, y: 830.0 }),
        Some(NativePanelPlatformEvent::ToggleSettingsSurface)
    );
    assert_eq!(
        native_panel_platform_event_at_point(&regions, PanelPoint { x: 780.0, y: 830.0 }),
        Some(NativePanelPlatformEvent::QuitApplication)
    );
}

#[test]
fn pointer_regions_use_settings_gap_for_debug_mode_trigger() {
    let layout = pointer_test_layout();
    let mut scene = pointer_test_scene();
    scene.mascot_pose = crate::native_panel_scene::SceneMascotPose::Hidden;

    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);
    let pill = crate::native_panel_core::absolute_rect(layout.panel_frame, layout.pill_frame);
    let action_layout = crate::native_panel_core::resolve_compact_action_button_layout(pill);
    let point = PanelPoint {
        x: action_layout.settings.x + action_layout.settings.width + 24.0,
        y: pill.y + pill.height / 2.0,
    };

    assert_eq!(
        native_panel_platform_event_at_point(&regions, point),
        Some(NativePanelPlatformEvent::DebugModeTrigger)
    );
}

#[test]
fn settings_pointer_regions_match_visible_rows_without_settings_folder_action() {
    let layout = resolve_panel_layout(PanelLayoutInput {
        screen_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        },
        metrics: PanelGeometryMetrics {
            compact_height: 38.0,
            compact_width: 253.0,
            expanded_width: 283.0,
            panel_width: 420.0,
        },
        canvas_height: 260.0,
        visible_height: 260.0,
        bar_progress: 1.0,
        height_progress: 1.0,
        drop_progress: 1.0,
        content_visibility: 1.0,
        collapsed_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
        drop_distance: crate::native_panel_core::PANEL_DROP_DISTANCE,
        content_top_gap: crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP,
        content_bottom_inset: crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET,
        cards_side_inset: crate::native_panel_core::EXPANDED_CARDS_SIDE_INSET,
        shoulder_size: crate::native_panel_core::COMPACT_SHOULDER_SIZE,
        separator_side_inset: crate::native_panel_core::EXPANDED_SEPARATOR_SIDE_INSET,
    });
    let state = PanelState {
        expanded: true,
        surface_mode: ExpandedSurface::Settings,
        ..PanelState::default()
    };
    let scene = build_panel_scene(
        &state,
        &RuntimeSnapshot {
            status: "Idle".to_string(),
            primary_source: "claude".to_string(),
            active_session_count: 0,
            total_session_count: 0,
            pending_permission_count: 0,
            pending_question_count: 0,
            pending_permission: None,
            pending_question: None,
            pending_permissions: Vec::new(),
            pending_questions: Vec::new(),
            sessions: Vec::new(),
        },
        &PanelSceneBuildInput::default(),
    );

    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);
    let hit_regions = regions
        .iter()
        .filter(|region| matches!(region.kind, NativePanelPointerRegionKind::HitTarget(_)))
        .collect::<Vec<_>>();

    assert_eq!(hit_regions.len(), 10);
    let display_region = hit_regions
        .iter()
        .find(|region| {
            matches!(
                region.kind,
                NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
                    action: PanelHitAction::CycleDisplay,
                    ..
                })
            )
        })
        .expect("display cycle region");
    assert_eq!(
        native_panel_platform_event_at_point(
            &regions,
            PanelPoint {
                x: display_region.frame.x + 8.0,
                y: display_region.frame.y + 2.0,
            },
        ),
        Some(NativePanelPlatformEvent::CycleDisplay)
    );
    assert!(!hit_regions.iter().any(|region| matches!(
        region.kind,
        NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
            action: PanelHitAction::OpenSettingsLocation,
            ..
        })
    )));

    let width_regions = hit_regions
        .iter()
        .filter(|region| {
            matches!(
                region.kind,
                NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
                    action: PanelHitAction::CycleIslandWidth,
                    ..
                })
            )
        })
        .collect::<Vec<_>>();
    assert!(width_regions.len() >= 2);
    let width_region = width_regions
        .iter()
        .max_by(|left, right| left.frame.width.total_cmp(&right.frame.width))
        .expect("island width region");
    let width_badge_region = width_regions
        .iter()
        .min_by(|left, right| left.frame.width.total_cmp(&right.frame.width))
        .expect("island width badge region");
    assert!(width_badge_region.frame.width < width_region.frame.width);
    assert_eq!(
        native_panel_platform_event_at_point(
            &regions,
            PanelPoint {
                x: width_region.frame.x + 8.0,
                y: width_region.frame.y + 2.0,
            },
        ),
        Some(NativePanelPlatformEvent::CycleIslandWidth)
    );
    let cards = absolute_expanded_rect(layout, layout.cards_frame);
    let settings_card_frame = PanelRect {
        x: cards.x,
        y: cards.y
            - (crate::native_panel_core::resolve_settings_surface_card_height(5) - cards.height)
                .max(0.0),
        width: cards.width,
        height: crate::native_panel_core::resolve_settings_surface_card_height(5),
    };
    let expected_badge = card_visual_settings_row_layout(
        settings_card_frame,
        1,
        &CardVisualRowSpec {
            title: "Island Width".to_string(),
            value: "M".to_string(),
            active: true,
        },
    )
    .expect("settings width row layout")
    .value_badge_frame;
    assert_eq!(width_badge_region.frame, expected_badge);
    assert_eq!(
        native_panel_platform_event_at_point(
            &regions,
            PanelPoint {
                x: width_badge_region.frame.x + width_badge_region.frame.width / 2.0,
                y: width_badge_region.frame.y + width_badge_region.frame.height / 2.0,
            },
        ),
        Some(NativePanelPlatformEvent::CycleIslandWidth)
    );

    let mute_region = hit_regions
        .iter()
        .find(|region| {
            matches!(
                region.kind,
                NativePanelPointerRegionKind::HitTarget(PanelHitTarget {
                    action: PanelHitAction::ToggleCompletionSound,
                    ..
                })
            )
        })
        .expect("mute sound region");
    assert_eq!(
        native_panel_platform_event_at_point(
            &regions,
            PanelPoint {
                x: mute_region.frame.x + 8.0,
                y: mute_region.frame.y + 2.0,
            },
        ),
        Some(NativePanelPlatformEvent::ToggleCompletionSound)
    );
}

#[test]
fn pointer_regions_do_not_claim_transparent_canvas_margins() {
    let layout = pointer_test_layout();
    let scene = pointer_test_scene();
    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);

    let content_frame = absolute_panel_rect(layout, layout.content_frame);
    assert!(!regions.iter().any(|region| {
        matches!(region.kind, NativePanelPointerRegionKind::Shell) && region.frame == content_frame
    }));
    assert!(!native_panel_pointer_inside_regions(
        &regions,
        PanelPoint {
            x: layout.panel_frame.x + 10.0,
            y: layout.panel_frame.y + layout.panel_frame.height - 2.0,
        }
    ));
    assert!(native_panel_pointer_inside_regions(
        &regions,
        PanelPoint {
            x: layout.panel_frame.x + layout.pill_frame.x + 20.0,
            y: layout.panel_frame.y + layout.pill_frame.y + 20.0,
        }
    ));
}

#[test]
fn cards_container_pointer_region_matches_expanded_card_stack_frame() {
    let layout = pointer_test_layout();
    let scene = pointer_test_scene();
    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);
    let cards_region = regions
        .iter()
        .find(|region| matches!(region.kind, NativePanelPointerRegionKind::CardsContainer))
        .expect("cards container pointer region");

    assert_eq!(
        cards_region.frame,
        absolute_expanded_rect(layout, layout.cards_frame)
    );
    assert_ne!(
        cards_region.frame,
        absolute_panel_rect(layout, layout.cards_frame)
    );
}

#[test]
fn pointer_regions_claim_expanded_top_gap_without_claiming_side_margins() {
    let layout = pointer_test_layout();
    let scene = pointer_test_scene();
    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);
    let shell = absolute_panel_rect(layout, layout.expanded_frame);

    assert!(native_panel_pointer_inside_regions(
        &regions,
        PanelPoint {
            x: shell.x + shell.width / 2.0,
            y: shell.y + shell.height + 1.0,
        }
    ));
    assert!(!native_panel_pointer_inside_regions(
        &regions,
        PanelPoint {
            x: layout.panel_frame.x + 10.0,
            y: shell.y + shell.height + 1.0,
        }
    ));
}

#[test]
fn pointer_regions_include_mascot_bubble_hover_overhang() {
    let layout = pointer_test_layout();
    let mut scene = pointer_test_scene();
    scene.compact_bar.completion_count = 2;
    scene.mascot_pose = crate::native_panel_scene::SceneMascotPose::Complete;

    let regions = resolve_native_panel_pointer_regions(layout, &scene, None);
    let pill = absolute_panel_rect(layout, layout.pill_frame);

    assert!(native_panel_pointer_inside_regions(
        &regions,
        PanelPoint {
            x: pill.x + 42.0,
            y: pill.y + pill.height + 4.0,
        }
    ));
}

#[test]
fn host_window_descriptor_projects_animation_and_window_state() {
    let descriptor = NativePanelHostWindowDescriptor {
        visible: true,
        preferred_display_index: 2,
        screen_frame: Some(PanelRect {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        }),
        shared_body_height: Some(180.0),
        timeline: Some(NativePanelTimelineDescriptor {
            animation: crate::native_panel_core::PanelAnimationDescriptor {
                kind: crate::native_panel_core::PanelAnimationKind::Open,
                canvas_height: 140.0,
                visible_height: 120.0,
                width_progress: 0.5,
                height_progress: 0.75,
                shoulder_progress: 1.0,
                drop_progress: 0.25,
                cards_progress: 0.8,
            },
            cards_entering: true,
        }),
    };

    assert_eq!(
        descriptor.animation_descriptor(),
        descriptor.timeline.map(|timeline| timeline.animation)
    );
    assert_eq!(
        descriptor.window_state(Some(PanelRect {
            x: 30.0,
            y: 40.0,
            width: 160.0,
            height: 100.0,
        })),
        NativePanelHostWindowState {
            frame: Some(PanelRect {
                x: 30.0,
                y: 40.0,
                width: 160.0,
                height: 100.0,
            }),
            visible: true,
            preferred_display_index: 2,
        }
    );
    assert_eq!(
        native_panel_host_window_frame(
            descriptor,
            PanelRect {
                x: 0.0,
                y: 0.0,
                width: 1440.0,
                height: 900.0,
            },
            400.0,
            700.0,
        ),
        Some(PanelRect {
            x: 10.0,
            y: 80.0,
            width: 550.0,
            height: 140.0,
        })
    );
}

#[test]
fn host_window_descriptor_helpers_update_shared_fields() {
    let animation = crate::native_panel_core::PanelAnimationDescriptor {
        kind: crate::native_panel_core::PanelAnimationKind::Open,
        canvas_height: 140.0,
        visible_height: 120.0,
        width_progress: 0.5,
        height_progress: 0.75,
        shoulder_progress: 1.0,
        drop_progress: 0.25,
        cards_progress: 0.8,
    };
    let timeline = native_panel_timeline_descriptor(animation, true);
    let mut descriptor = native_panel_host_window_descriptor(false, 0, None, None, None);

    sync_native_panel_host_window_visibility(&mut descriptor, true);
    sync_native_panel_host_window_screen_frame(
        &mut descriptor,
        2,
        Some(PanelRect {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        }),
    );
    sync_native_panel_host_window_shared_body_height(&mut descriptor, Some(180.0));
    sync_native_panel_host_window_timeline(&mut descriptor, Some(timeline));

    assert!(descriptor.visible);
    assert_eq!(descriptor.preferred_display_index, 2);
    assert_eq!(descriptor.shared_body_height, Some(180.0));
    assert_eq!(descriptor.timeline, Some(timeline));
}

#[test]
fn host_window_descriptor_patch_updates_multiple_fields_together() {
    let animation = crate::native_panel_core::PanelAnimationDescriptor {
        kind: crate::native_panel_core::PanelAnimationKind::Open,
        canvas_height: 140.0,
        visible_height: 120.0,
        width_progress: 0.5,
        height_progress: 0.75,
        shoulder_progress: 1.0,
        drop_progress: 0.25,
        cards_progress: 0.8,
    };
    let timeline = native_panel_timeline_descriptor(animation, true);
    let mut descriptor = native_panel_host_window_descriptor(false, 0, None, None, None);

    patch_native_panel_host_window_descriptor(
        &mut descriptor,
        NativePanelHostWindowDescriptorPatch {
            visible: Some(true),
            preferred_display_index: Some(3),
            screen_frame: Some(Some(PanelRect {
                x: 10.0,
                y: 20.0,
                width: 300.0,
                height: 200.0,
            })),
            shared_body_height: Some(Some(180.0)),
            timeline: Some(Some(timeline)),
        },
    );

    assert!(descriptor.visible);
    assert_eq!(descriptor.preferred_display_index, 3);
    assert_eq!(descriptor.shared_body_height, Some(180.0));
    assert_eq!(descriptor.timeline, Some(timeline));
}

#[test]
fn timeline_descriptor_for_animation_derives_card_direction() {
    let mut animation = crate::native_panel_core::PanelAnimationDescriptor {
        kind: crate::native_panel_core::PanelAnimationKind::Open,
        canvas_height: 140.0,
        visible_height: 120.0,
        width_progress: 0.5,
        height_progress: 0.75,
        shoulder_progress: 1.0,
        drop_progress: 0.25,
        cards_progress: 0.8,
    };

    assert!(native_panel_timeline_descriptor_for_animation(animation).cards_entering);

    animation.kind = crate::native_panel_core::PanelAnimationKind::SurfaceSwitch;
    assert!(native_panel_timeline_descriptor_for_animation(animation).cards_entering);

    animation.kind = crate::native_panel_core::PanelAnimationKind::Close;
    assert!(!native_panel_timeline_descriptor_for_animation(animation).cards_entering);
}
