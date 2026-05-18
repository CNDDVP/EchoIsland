use super::{
    Direct2DWindowsNativePanelPainter, PlanOnlyWindowsNativePanelPainter, WindowsCompactPillPath,
    WindowsCompactShoulderPath, WindowsDirect2DCoordinateSpace, WindowsNativePanelPainter,
};
use crate::windows_native_panel::d2d_resource_cache::{
    WindowsDirect2DResourceCacheState, WindowsDirect2DResourceKey,
};
use crate::{
    native_panel_core::{ExpandedSurface, PanelPoint, PanelRect},
    native_panel_renderer::facade::{
        descriptor::NativePanelEdgeAction,
        descriptor::NativePanelHostWindowState,
        presentation::{
            NativePanelVisualActionButtonInput, NativePanelVisualCardBadgeInput,
            NativePanelVisualCardInput, NativePanelVisualCardRowInput,
            NativePanelVisualDisplayMode,
        },
        visual::native_panel_visual_text_box_height,
    },
    native_panel_scene::SceneMascotPose,
    windows_native_panel::window_shell::WindowsNativePanelShellPaintJob,
};

fn compact_paint_job() -> WindowsNativePanelShellPaintJob {
    WindowsNativePanelShellPaintJob {
        window_state: NativePanelHostWindowState {
            frame: Some(PanelRect {
                x: 100.0,
                y: 20.0,
                width: 320.0,
                height: 80.0,
            }),
            visible: true,
            preferred_display_index: 0,
        },
        display_mode: NativePanelVisualDisplayMode::Compact,
        surface: ExpandedSurface::Default,
        panel_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 80.0,
        },
        compact_bar_frame: PanelRect {
            x: 32.0,
            y: 12.0,
            width: 253.0,
            height: 40.0,
        },
        left_shoulder_frame: PanelRect {
            x: 26.0,
            y: 46.0,
            width: 6.0,
            height: 6.0,
        },
        right_shoulder_frame: PanelRect {
            x: 285.0,
            y: 46.0,
            width: 6.0,
            height: 6.0,
        },
        shoulder_progress: 0.0,
        content_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 80.0,
        },
        card_stack_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 320.0,
            height: 80.0,
        },
        card_stack_content_height: 80.0,
        shell_frame: PanelRect {
            x: 32.0,
            y: 0.0,
            width: 253.0,
            height: 80.0,
        },
        headline_text: "Codex ready".to_string(),
        headline_emphasized: false,
        active_count: "1".to_string(),
        active_count_elapsed_ms: 0,
        total_count: "3".to_string(),
        separator_visibility: 0.0,
        chrome_transition_progress: 0.0,
        cards_visible: false,
        card_count: 0,
        cards: Vec::new(),
        glow_visible: false,
        glow_opacity: 0.0,
        action_buttons_visible: false,
        action_buttons: Vec::new(),
        completion_count: 1,
        mascot_elapsed_ms: 0,
        mascot_motion_frame: None,
        mascot_pose: SceneMascotPose::Idle,
        mascot_debug_mode_enabled: false,
    }
}

fn expanded_paint_job() -> WindowsNativePanelShellPaintJob {
    WindowsNativePanelShellPaintJob {
            display_mode: NativePanelVisualDisplayMode::Expanded,
            window_state: NativePanelHostWindowState {
                frame: Some(PanelRect {
                    x: 100.0,
                    y: 20.0,
                    width: 420.0,
                    height: 180.0,
                }),
                visible: true,
                preferred_display_index: 0,
            },
            panel_frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 420.0,
                height: 180.0,
            },
            content_frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 420.0,
                height: 180.0,
            },
            card_stack_frame: PanelRect {
                x: 68.5,
                y: 34.0,
                width: 283.0,
                height: 180.0,
            },
            card_stack_content_height: 180.0,
            compact_bar_frame: PanelRect {
                x: 83.5,
                y: 143.0,
                width: 253.0,
                height: 37.0,
            },
            left_shoulder_frame: PanelRect {
                x: 77.5,
                y: 174.0,
                width: 6.0,
                height: 6.0,
            },
            right_shoulder_frame: PanelRect {
                x: 336.5,
                y: 174.0,
                width: 6.0,
                height: 6.0,
            },
            shoulder_progress: 1.0,
            shell_frame: PanelRect {
                x: 68.5,
                y: 34.0,
                width: 283.0,
                height: 146.0,
            },
            surface: ExpandedSurface::Settings,
            headline_text: "Settings".to_string(),
            headline_emphasized: true,
            active_count: "2".to_string(),
            active_count_elapsed_ms: 0,
            total_count: "4".to_string(),
            separator_visibility: 0.8,
            chrome_transition_progress: 1.0,
            cards_visible: true,
            card_count: 2,
            cards: vec![
                NativePanelVisualCardInput {
                    style: crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Settings,
                    title: "Settings".to_string(),
                    subtitle: Some("EchoIsland v0.6.1".to_string()),
                    body: None,
                    badge: None,
                    source_badge: None,
                    body_prefix: None,
                    body_lines: Vec::new(),
                    action_hint: None,
                    rows: vec![NativePanelVisualCardRowInput {
                        title: "Mute Sound".to_string(),
                        value: "Off".to_string(),
                        active: true,
                    }],
                    height: 92.0,
                    collapsed_height: 64.0,
                    compact: false,
                    removing: false,
                },
                NativePanelVisualCardInput {
                    style: crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Completion,
                    title: "Done".to_string(),
                    subtitle: Some("#abcdef now".to_string()),
                    body: Some("Task complete".to_string()),
                    badge: Some(NativePanelVisualCardBadgeInput {
                        text: "Done".to_string(),
                        emphasized: true,
                    }),
                    source_badge: Some(NativePanelVisualCardBadgeInput {
                        text: "Codex".to_string(),
                        emphasized: false,
                    }),
                    body_prefix: Some("$".to_string()),
                    body_lines: Vec::new(),
                    action_hint: None,
                    rows: Vec::new(),
                    height: 76.0,
                    collapsed_height: 52.0,
                    compact: false,
                    removing: false,
                },
            ],
            glow_visible: true,
            glow_opacity: 0.78,
            action_buttons_visible: true,
            action_buttons: vec![NativePanelVisualActionButtonInput {
                action: NativePanelEdgeAction::Settings,
                frame: PanelRect {
                    x: 300.0,
                    y: 152.0,
                    width: 18.0,
                    height: 18.0,
                },
                debug_mode_enabled: false,
            }],
            completion_count: 0,
            mascot_elapsed_ms: 0,
            mascot_motion_frame: None,
            mascot_pose: SceneMascotPose::Complete,
            mascot_debug_mode_enabled: false,
        }
}

#[test]
fn plan_only_painter_preserves_text_primitives_for_tests() {
    let mut painter = PlanOnlyWindowsNativePanelPainter;
    let plan = painter.paint(&compact_paint_job()).expect("paint plan");

    assert!(!plan.hidden);
    assert!(
            plan.primitives
                .iter()
                .any(|primitive| matches!(
                    primitive,
                    crate::windows_native_panel::paint_backend::WindowsNativePanelPaintPrimitive::Text { text, .. }
                    if text == "Codex ready"
                ))
        );
}

#[test]
fn direct2d_painter_skeleton_consumes_shared_visual_plan() {
    let mut painter = Direct2DWindowsNativePanelPainter::default();
    let plan = painter.paint(&compact_paint_job()).expect("paint plan");

    assert!(!plan.hidden);
    assert!(!plan.primitives.is_empty());
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            crate::windows_native_panel::paint_backend::WindowsNativePanelPaintPrimitive::CompactShoulder {
                side: crate::native_panel_renderer::facade::visual::NativePanelVisualShoulderSide::Left,
                ..
            }
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            crate::windows_native_panel::paint_backend::WindowsNativePanelPaintPrimitive::CompactShoulder {
                side: crate::native_panel_renderer::facade::visual::NativePanelVisualShoulderSide::Right,
                ..
            }
        )));
}

#[test]
fn direct2d_painter_skeleton_routes_compact_text_to_directwrite_requests() {
    let mut job = compact_paint_job();
    job.completion_count = 0;
    let mut painter = Direct2DWindowsNativePanelPainter::default();
    let plan = painter.paint(&job).expect("paint plan");

    let requests = super::directwrite_text_requests_from_paint_plan(&plan);

    assert!(requests.iter().any(|request| request.text == "Codex ready"));
    assert!(requests.iter().any(|request| request.text == "1"));
    assert!(requests.iter().any(|request| request.text == "/"));
    assert!(requests.iter().any(|request| request.text == "3"));
    assert!(requests.iter().any(|request| {
            request.text == "Codex ready"
                && request.weight
                    == crate::native_panel_renderer::facade::visual::NativePanelVisualTextWeight::Semibold
                && request.alignment
                    == crate::native_panel_renderer::facade::visual::NativePanelVisualTextAlignment::Center
        }));
    assert!(requests.iter().any(|request| {
            request.text == "1"
                && request.alignment
                    == crate::native_panel_renderer::facade::visual::NativePanelVisualTextAlignment::Right
        }));
    assert!(requests.iter().any(|request| {
            request.text == "/"
                && request.alignment
                    == crate::native_panel_renderer::facade::visual::NativePanelVisualTextAlignment::Center
        }));
    assert!(requests.iter().any(|request| {
            request.text == "3"
                && request.alignment
                    == crate::native_panel_renderer::facade::visual::NativePanelVisualTextAlignment::Left
        }));
    assert!(requests.iter().all(|request| {
        request.fonts.primary == "Noto Sans SC" && request.fonts.fallback == "Segoe UI Variable"
    }));
}

#[test]
fn direct2d_painter_skeleton_routes_expanded_cards_to_directwrite_requests() {
    let mut painter = Direct2DWindowsNativePanelPainter::default();
    let plan = painter.paint(&expanded_paint_job()).expect("paint plan");
    let requests = super::directwrite_text_requests_from_paint_plan(&plan);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        crate::windows_native_panel::paint_backend::WindowsNativePanelPaintPrimitive::RoundRect {
            frame,
            ..
        } if (frame.width - 283.0).abs() < 0.001
            && (frame.height - 146.0).abs() < 0.001
    )));
    assert!(requests.iter().any(|request| request.text == "Settings"));
    assert!(requests.iter().any(|request| request.text == "Done"));
    assert!(
        requests
            .iter()
            .any(|request| request.text == "Task complete")
    );
    assert!(requests.iter().any(|request| request.text == "Codex"));
}

#[cfg(windows)]
#[test]
fn direct2d_painter_initializes_native_factories_for_alpha_rendering() {
    let painter = Direct2DWindowsNativePanelPainter::new(None).expect("create painter");

    assert!(painter.is_per_pixel_alpha_ready());
}

#[test]
fn direct2d_resource_cache_reuses_same_physical_rect_and_dpi_key() {
    let key = WindowsDirect2DResourceKey::new(
        crate::windows_native_panel::dpi::WindowsPhysicalRect {
            x: 10,
            y: 20,
            width: 253,
            height: 80,
        },
        crate::windows_native_panel::dpi::WindowsDpiScale::from_scale(1.0),
    );
    let mut cache = WindowsDirect2DResourceCacheState::default();

    assert!(cache.sync(key));
    assert!(!cache.sync(key));
    assert_eq!(cache.rebuild_count(), 1);
}

#[test]
fn direct2d_resource_cache_rebuilds_when_dpi_changes_without_size_change() {
    let rect = crate::windows_native_panel::dpi::WindowsPhysicalRect {
        x: 10,
        y: 20,
        width: 253,
        height: 80,
    };
    let mut cache = WindowsDirect2DResourceCacheState::default();

    assert!(cache.sync(WindowsDirect2DResourceKey::new(
        rect,
        crate::windows_native_panel::dpi::WindowsDpiScale::from_scale(1.0)
    )));
    assert!(cache.sync(WindowsDirect2DResourceKey::new(
        rect,
        crate::windows_native_panel::dpi::WindowsDpiScale::from_scale(1.25)
    )));
    assert_eq!(cache.rebuild_count(), 2);
}

#[test]
fn direct2d_coordinate_space_flips_shared_mac_style_rects_to_windows_top_left() {
    let coordinates = WindowsDirect2DCoordinateSpace::new(80.0);

    assert_eq!(
        coordinates.rect(PanelRect {
            x: 83.5,
            y: 43.0,
            width: 253.0,
            height: 37.0,
        }),
        PanelRect {
            x: 83.5,
            y: 0.0,
            width: 253.0,
            height: 37.0,
        }
    );
}

#[test]
fn direct2d_coordinate_space_flips_text_origin_with_text_height() {
    let coordinates = WindowsDirect2DCoordinateSpace::new(80.0);

    assert_eq!(
        coordinates.text_rect(PanelPoint { x: 139.5, y: 53.5 }, 129.0, 22.0),
        PanelRect {
            x: 139.5,
            y: 4.5,
            width: 129.0,
            height: 22.0,
        }
    );
}

#[test]
fn direct2d_text_box_height_matches_compact_label_metrics() {
    assert_eq!(native_panel_visual_text_box_height("EchoIsland", 13), 24.0);
    assert_eq!(native_panel_visual_text_box_height("1", 15), 24.0);
    assert_eq!(native_panel_visual_text_box_height("2", 8), 16.0);
    assert_eq!(
        native_panel_visual_text_box_height("line one\nline two", 10),
        36.0
    );
}

#[test]
fn compact_shoulder_path_matches_mac_style_corner_curve() {
    let path = WindowsCompactShoulderPath::resolve(
        PanelRect {
            x: 26.0,
            y: 0.0,
            width: 6.0,
            height: 6.0,
        },
        crate::native_panel_renderer::facade::visual::NativePanelVisualShoulderSide::Left,
        0.0,
    )
    .expect("visible shoulder path");

    assert_point_near(path.start, PanelPoint { x: 26.0, y: 0.0 });
    assert_point_near(path.line_to_top_edge, PanelPoint { x: 32.0, y: 0.0 });
    assert_point_near(path.line_to_outer_edge, PanelPoint { x: 32.0, y: 6.0 });
    assert_point_near(path.curve_control_1, PanelPoint { x: 32.0, y: 2.28 });
    assert_point_near(path.curve_control_2, PanelPoint { x: 29.72, y: 0.0 });
    assert_point_near(path.curve_end, PanelPoint { x: 26.0, y: 0.0 });
}

#[test]
fn compact_pill_path_keeps_top_edge_flat_and_rounds_bottom_corners() {
    let path = WindowsCompactPillPath::resolve(
        PanelRect {
            x: 32.0,
            y: 0.0,
            width: 253.0,
            height: 37.0,
        },
        12.5,
    );

    assert_point_near(path.start, PanelPoint { x: 32.0, y: 0.0 });
    assert_point_near(path.top_right, PanelPoint { x: 285.0, y: 0.0 });
    assert_point_near(
        path.right_edge_bottom_arc_start,
        PanelPoint { x: 285.0, y: 24.5 },
    );
    assert_point_near(path.bottom_right_arc_end, PanelPoint { x: 272.5, y: 37.0 });
    assert_point_near(path.bottom_left_arc_start, PanelPoint { x: 44.5, y: 37.0 });
    assert_point_near(path.bottom_left_arc_end, PanelPoint { x: 32.0, y: 24.5 });
}

fn assert_point_near(actual: PanelPoint, expected: PanelPoint) {
    assert!(
        (actual.x - expected.x).abs() < 0.001,
        "expected x {} got {}",
        expected.x,
        actual.x
    );
    assert!(
        (actual.y - expected.y).abs() < 0.001,
        "expected y {} got {}",
        expected.y,
        actual.y
    );
}
