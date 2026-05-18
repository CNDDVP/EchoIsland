#![allow(unused_imports)]

use super::super::*;
use super::common::*;
use chrono::Utc;
use echoisland_runtime::{
    PendingPermissionView, PendingQuestionView, RuntimeSnapshot, SessionSnapshotView,
};
use std::time::{Duration, Instant};

#[test]
fn settings_surface_card_height_grows_with_row_count() {
    assert_eq!(resolve_settings_surface_card_height(4), 206.0);
    assert_eq!(resolve_settings_surface_card_height(5), 244.0);
}

#[test]
fn panel_style_resolver_hides_actions_before_threshold() {
    let resolved = resolve_panel_style(PanelStyleResolverInput {
        shell_visible: true,
        separator_visibility: 0.5,
        shared_visible: false,
        bar_progress: 0.3,
        height_progress: 0.0,
        headline_emphasized: true,
        edge_actions_visible: false,
        compact_pill_radius: 20.0,
        panel_morph_pill_radius: 24.0,
        expanded_panel_radius: 28.0,
    });

    assert!(resolved.highlight_alpha > 0.0);
    assert!(resolved.actions_hidden);
    assert_eq!(resolved.action_alpha, 0.0);
    assert!(!resolved.use_compact_corner_mask);
}

#[test]
fn panel_style_resolver_reveals_actions_and_morphs_shell() {
    let resolved = resolve_panel_style(PanelStyleResolverInput {
        shell_visible: true,
        separator_visibility: 0.5,
        shared_visible: true,
        bar_progress: 1.0,
        height_progress: 1.0,
        headline_emphasized: false,
        edge_actions_visible: true,
        compact_pill_radius: 20.0,
        panel_morph_pill_radius: 24.0,
        expanded_panel_radius: 28.0,
    });

    assert!(resolved.cards_hidden);
    assert!(!resolved.actions_hidden);
    assert_eq!(resolved.action_alpha, 1.0);
    assert_eq!(resolved.action_scale, 1.0);
    assert_eq!(resolved.pill_corner_radius, 24.0);
    assert_eq!(resolved.pill_border_width, 0.0);
    assert_eq!(resolved.expanded_corner_radius, 28.0);
    assert!(!resolved.use_compact_corner_mask);
}

#[test]
fn render_progress_clamps_transition_frame_values() {
    let progress = resolve_panel_render_progress(PanelTransitionFrame {
        canvas_height: 120.0,
        visible_height: 120.0,
        bar_progress: -0.4,
        height_progress: 1.4,
        shoulder_progress: 0.5,
        drop_progress: 2.0,
        cards_progress: 0.0,
    });

    assert_eq!(
        progress,
        PanelRenderProgress {
            bar: 0.0,
            height: 1.0,
            shoulder: 0.5,
            drop: 1.0,
        }
    );
}

#[test]
fn centered_top_frame_snaps_geometry_to_whole_points() {
    let frame = resolve_centered_top_frame(
        PanelRect {
            x: 0.0,
            y: 0.0,
            width: 1512.0,
            height: 982.0,
        },
        PanelSize {
            width: 419.6,
            height: 152.4,
        },
    );

    assert_eq!(
        frame,
        PanelRect {
            x: 546.0,
            y: 830.0,
            width: 420.0,
            height: 152.0,
        }
    );
}

#[test]
fn centered_top_frame_preserves_left_edge_when_panel_is_wider_than_screen() {
    let frame = resolve_centered_top_frame(
        PanelRect {
            x: 100.0,
            y: 200.0,
            width: 300.0,
            height: 800.0,
        },
        PanelSize {
            width: 420.0,
            height: 80.0,
        },
    );

    assert_eq!(frame.x, 100.0);
    assert_eq!(frame.y, 920.0);
    assert_eq!(frame.width, 420.0);
    assert_eq!(frame.height, 80.0);
}

#[test]
fn rect_helpers_compose_and_hit_test_without_platform_types() {
    let parent = PanelRect {
        x: 100.0,
        y: 200.0,
        width: 420.0,
        height: 180.0,
    };
    let child = PanelRect {
        x: 20.0,
        y: 12.0,
        width: 80.0,
        height: 30.0,
    };
    let absolute = absolute_rect(parent, child);

    assert_eq!(
        absolute,
        PanelRect {
            x: 120.0,
            y: 212.0,
            width: 80.0,
            height: 30.0,
        }
    );
    assert_eq!(compose_local_rect(parent, child), absolute);
    assert!(point_in_rect(PanelPoint { x: 200.0, y: 242.0 }, absolute));
    assert!(!point_in_rect(PanelPoint { x: 200.1, y: 242.0 }, absolute));
}

#[test]
fn rect_nearly_equal_uses_tolerance_for_all_edges() {
    assert!(rects_nearly_equal(
        PanelRect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        },
        PanelRect {
            x: 10.4,
            y: 20.4,
            width: 30.4,
            height: 40.4,
        },
        0.5,
    ));
    assert!(!rects_nearly_equal(
        PanelRect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        },
        PanelRect {
            x: 10.5,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        },
        0.5,
    ));
}

#[test]
fn island_bar_frame_interpolates_width_and_preserves_top_offset() {
    let compact = resolve_island_bar_frame(
        PanelSize {
            width: 420.0,
            height: 164.0,
        },
        0.0,
        253.0,
        366.0,
        40.0,
        4.5,
    );
    let expanded = resolve_island_bar_frame(
        PanelSize {
            width: 420.0,
            height: 164.0,
        },
        1.0,
        253.0,
        366.0,
        40.0,
        4.5,
    );

    assert_eq!(compact.width, 253.0);
    assert_eq!(expanded.width, 366.0);
    assert_eq!(compact.y, 119.5);
    assert_eq!(expanded.y, 119.5);
}

#[test]
fn expanded_background_frame_clamps_height_and_interpolates_width() {
    let frame = resolve_expanded_background_frame(
        PanelSize {
            width: 420.0,
            height: 164.0,
        },
        500.0,
        0.5,
        1.0,
        253.0,
        366.0,
        40.0,
        4.5,
        80.0,
    );

    assert_eq!(frame.width, 309.5);
    assert_eq!(frame.height, 159.5);
    assert_eq!(frame.y, 0.0);
}

#[test]
fn expanded_body_frames_match_insets() {
    let container = PanelRect {
        x: 12.0,
        y: 0.0,
        width: 366.0,
        height: 164.0,
    };
    let cards = resolve_expanded_cards_frame(container, 40.0, 8.0, 10.0, 14.0);
    let separator = resolve_expanded_separator_frame(container, 40.0, 14.0);

    assert_eq!(
        cards,
        PanelRect {
            x: 14.0,
            y: 10.0,
            width: 338.0,
            height: 106.0,
        }
    );
    assert_eq!(
        separator,
        PanelRect {
            x: 14.0,
            y: 123.5,
            width: 338.0,
            height: 1.0,
        }
    );
}

#[test]
fn compact_bar_content_layout_centers_headline_and_keeps_counts_trailing() {
    let compact = resolve_compact_bar_content_layout(CompactBarContentLayoutInput {
        bar_width: 253.0,
        bar_height: 37.0,
    });
    let expanded = resolve_compact_bar_content_layout(CompactBarContentLayoutInput {
        bar_width: 283.0,
        bar_height: 37.0,
    });

    assert_eq!(compact.mascot_center_x, 22.0);
    assert_eq!(compact.headline_x, 48.5);
    assert_eq!(compact.headline_width, 156.0);
    assert_eq!(compact.headline_center_x, 126.5);
    assert_eq!(compact.active_x, 208.0);
    assert_eq!(compact.slash_x, 217.0);
    assert_eq!(compact.total_x, 229.0);

    assert_eq!(expanded.headline_x, 63.5);
    assert_eq!(expanded.headline_width, 156.0);
    assert_eq!(expanded.headline_center_x, 141.5);
    assert_eq!(expanded.active_x, 238.0);
    assert_eq!(expanded.slash_x, 247.0);
    assert_eq!(expanded.total_x, 259.0);
}

#[test]
fn compact_action_button_layout_replaces_mascot_and_count_slots() {
    let compact_frame = PanelRect {
        x: 10.0,
        y: 5.0,
        width: DEFAULT_EXPANDED_PILL_WIDTH,
        height: DEFAULT_COMPACT_PILL_HEIGHT,
    };
    let content = resolve_compact_bar_content_layout(CompactBarContentLayoutInput {
        bar_width: compact_frame.width,
        bar_height: compact_frame.height,
    });
    let action_layout = resolve_compact_action_button_layout(compact_frame);
    let settings_center_x = action_layout.settings.x + action_layout.settings.width / 2.0;
    let quit_center_x = action_layout.quit.x + action_layout.quit.width / 2.0;
    let count_center_x = compact_frame.x + (content.active_x + content.total_x + 24.0) / 2.0;

    assert_eq!(settings_center_x, compact_frame.x + content.mascot_center_x);
    assert_eq!(quit_center_x, count_center_x);
    assert_eq!(action_layout.settings.y, action_layout.quit.y);
}

#[test]
fn panel_chrome_visibility_swaps_default_chrome_for_actions_during_expansion() {
    let collapsed = resolve_panel_chrome_visibility_spec(PanelChromeVisibilitySpecInput {
        expanded_display_mode: false,
        surface: ExpandedSurface::Default,
        edge_actions_visible: false,
        transition_visibility_progress: 0.0,
    });

    assert!(collapsed.collapsed_mascot_visible);
    assert!(collapsed.collapsed_metrics_visible);
    assert!(!collapsed.action_buttons.visible);

    let early_expanded = resolve_panel_chrome_visibility_spec(PanelChromeVisibilitySpecInput {
        expanded_display_mode: true,
        surface: ExpandedSurface::Default,
        edge_actions_visible: true,
        transition_visibility_progress: 0.5,
    });

    assert!(early_expanded.collapsed_mascot_visible);
    assert!(early_expanded.collapsed_metrics_visible);
    assert!(early_expanded.collapsed_exit_progress > 0.0);
    assert!(early_expanded.collapsed_exit_progress < 1.0);
    assert!(early_expanded.action_buttons.visible);
    assert!(early_expanded.action_buttons.scale < 1.0);

    let expanded = resolve_panel_chrome_visibility_spec(PanelChromeVisibilitySpecInput {
        expanded_display_mode: true,
        surface: ExpandedSurface::Default,
        edge_actions_visible: true,
        transition_visibility_progress: 1.0,
    });

    assert!(!expanded.collapsed_mascot_visible);
    assert!(!expanded.collapsed_metrics_visible);
    assert_eq!(expanded.collapsed_exit_progress, 1.0);
    assert!(expanded.action_buttons.visible);
}

#[test]
fn panel_chrome_visibility_keeps_default_chrome_without_actions_for_message_cards() {
    let message = resolve_panel_chrome_visibility_spec(PanelChromeVisibilitySpecInput {
        expanded_display_mode: true,
        surface: ExpandedSurface::Status,
        edge_actions_visible: true,
        transition_visibility_progress: 1.0,
    });

    assert!(message.collapsed_mascot_visible);
    assert!(message.collapsed_metrics_visible);
    assert_eq!(message.collapsed_exit_progress, 0.0);
    assert!(!message.action_buttons.visible);
}

#[test]
fn panel_chrome_transition_progress_tracks_open_without_separator_inference() {
    let width_stage = resolve_panel_chrome_transition_progress(PanelAnimationDescriptor {
        kind: PanelAnimationKind::Open,
        canvas_height: COLLAPSED_PANEL_HEIGHT,
        visible_height: COLLAPSED_PANEL_HEIGHT,
        width_progress: 0.8,
        height_progress: 0.0,
        shoulder_progress: 1.0,
        drop_progress: 0.0,
        cards_progress: 0.0,
    });
    let height_stage = resolve_panel_chrome_transition_progress(PanelAnimationDescriptor {
        kind: PanelAnimationKind::Open,
        canvas_height: 220.0,
        visible_height: 160.0,
        width_progress: 1.0,
        height_progress: 0.4,
        shoulder_progress: 1.0,
        drop_progress: 0.4,
        cards_progress: 0.0,
    });

    assert_eq!(width_stage, 0.4);
    assert_eq!(height_stage, 0.7);
}

#[test]
fn active_count_marquee_keeps_single_digit_static() {
    let frame = resolve_active_count_marquee_frame(ActiveCountMarqueeInput {
        text: "7",
        elapsed_ms: ACTIVE_COUNT_SCROLL_HOLD_MS + 120,
    });

    assert_eq!(frame.current, "7");
    assert_eq!(frame.next, "7");
    assert!(!frame.show_next);
    assert_eq!(frame.scroll_offset, 0.0);
}

#[test]
fn active_count_marquee_holds_then_scrolls_between_digits() {
    let held = resolve_active_count_marquee_frame(ActiveCountMarqueeInput {
        text: "23",
        elapsed_ms: ACTIVE_COUNT_SCROLL_HOLD_MS - 1,
    });
    let moving = resolve_active_count_marquee_frame(ActiveCountMarqueeInput {
        text: "23",
        elapsed_ms: ACTIVE_COUNT_SCROLL_HOLD_MS + (ACTIVE_COUNT_SCROLL_MOVE_MS / 2),
    });
    let moved = resolve_active_count_marquee_frame(ActiveCountMarqueeInput {
        text: "23",
        elapsed_ms: ACTIVE_COUNT_SCROLL_HOLD_MS + ACTIVE_COUNT_SCROLL_MOVE_MS + 1,
    });

    assert_eq!(held.current, "2");
    assert_eq!(held.next, "3");
    assert!(held.show_next);
    assert_eq!(held.scroll_offset, 0.0);
    assert_eq!(moving.current, "2");
    assert_eq!(moving.next, "3");
    assert!(moving.scroll_offset > 0.0);
    assert!(moving.scroll_offset < ACTIVE_COUNT_SCROLL_TRAVEL);
    assert_eq!(moved.scroll_offset, ACTIVE_COUNT_SCROLL_TRAVEL);
}

#[test]
fn panel_layout_clamps_visible_height_and_resolves_child_frames() {
    let layout = resolve_panel_layout(PanelLayoutInput {
        screen_frame: PanelRect {
            x: 0.0,
            y: 0.0,
            width: 1512.0,
            height: 982.0,
        },
        metrics: PanelGeometryMetrics {
            compact_height: 38.0,
            compact_width: 126.0,
            expanded_width: 356.0,
            panel_width: 356.0,
        },
        canvas_height: 120.0,
        visible_height: 220.0,
        bar_progress: 1.0,
        height_progress: 1.0,
        drop_progress: 0.0,
        content_visibility: 1.0,
        collapsed_height: 80.0,
        drop_distance: 8.0,
        content_top_gap: 8.0,
        content_bottom_inset: 10.0,
        cards_side_inset: 14.0,
        shoulder_size: 6.0,
        separator_side_inset: 14.0,
    });

    assert_eq!(layout.panel_frame.x, 578.0);
    assert_eq!(layout.content_frame.height, 120.0);
    assert_eq!(layout.expanded_frame.height, 120.0);
    assert_eq!(layout.cards_frame.height, 64.0);
    assert_eq!(layout.left_shoulder_frame.x, -6.0);
    assert_eq!(layout.right_shoulder_frame.x, 356.0);
    assert!(layout.shell_visible);
    assert_eq!(layout.separator_visibility, 0.88);
}

#[test]
fn panel_screen_widths_match_non_camera_housing_defaults() {
    let input = PanelScreenWidthInput {
        top_area: PanelScreenTopArea {
            screen_width: 1440.0,
            auxiliary_left_width: 0.0,
            auxiliary_right_width: 0.0,
        },
        compact_height: 37.0,
        default_compact_width: 253.0,
        expanded_width_delta: 30.0,
        default_expanded_width: 283.0,
        default_canvas_width: 420.0,
    };

    assert!(!resolve_panel_screen_has_camera_housing(input.top_area));
    assert_eq!(resolve_panel_notch_width(input.top_area), 240.0);
    assert_eq!(resolve_panel_shell_width(input), 253.0);
    assert_eq!(resolve_panel_expanded_width(input), 283.0);
    assert_eq!(resolve_panel_canvas_width(input), 420.0);
}

#[test]
fn panel_screen_widths_expand_around_camera_housing() {
    let input = PanelScreenWidthInput {
        top_area: PanelScreenTopArea {
            screen_width: 1512.0,
            auxiliary_left_width: 651.0,
            auxiliary_right_width: 651.0,
        },
        compact_height: 37.0,
        default_compact_width: 253.0,
        expanded_width_delta: 30.0,
        default_expanded_width: 283.0,
        default_canvas_width: 420.0,
    };

    assert!(resolve_panel_screen_has_camera_housing(input.top_area));
    assert_eq!(resolve_panel_notch_width(input.top_area), 210.0);
    assert_eq!(resolve_panel_shell_width(input), 320.144);
    assert_eq!(resolve_panel_expanded_width(input), 350.144);
    assert_eq!(resolve_panel_canvas_width(input), 420.0);
}

#[test]
fn panel_screen_width_fallbacks_preserve_existing_clamps() {
    assert_eq!(resolve_fallback_panel_expanded_width(500.0, 253.0), 253.0);
    assert_eq!(resolve_fallback_panel_expanded_width(200.0, 253.0), 200.0);
    assert_eq!(resolve_fallback_panel_expanded_width(0.0, 253.0), 1.0);
    assert_eq!(resolve_fallback_panel_canvas_width(300.0, 420.0), 420.0);
    assert_eq!(resolve_fallback_panel_canvas_width(500.0, 420.0), 500.0);
}

#[test]
fn expanded_cards_width_never_goes_negative() {
    assert_eq!(resolve_expanded_cards_width(366.0, 14.0), 338.0);
    assert_eq!(resolve_expanded_cards_width(20.0, 14.0), 0.0);
}

#[test]
fn native_panel_host_frame_interpolates_width_and_uses_canvas_height() {
    let frame = resolve_native_panel_host_frame(
        PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 180.2,
            visible_height: 140.0,
            width_progress: 0.5,
            height_progress: 0.0,
            shoulder_progress: 0.0,
            drop_progress: 0.0,
            cards_progress: 0.0,
        },
        PanelRect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        },
        400.0,
        700.0,
    );

    assert_eq!(
        frame,
        PanelRect {
            x: 445.0,
            y: 720.0,
            width: 550.0,
            height: 180.0,
        }
    );
}

#[test]
fn expanded_total_height_prefers_larger_shared_height_and_caps_body() {
    assert_eq!(
        resolve_expanded_total_height(84.0, Some(124.0), 40.0, 8.0, 10.0, 220.0),
        182.0
    );
    assert_eq!(
        resolve_expanded_total_height(84.0, Some(260.0), 40.0, 8.0, 10.0, 220.0),
        278.0
    );
}

#[test]
fn panel_transition_canvas_height_uses_largest_height() {
    assert_eq!(
        resolve_panel_transition_canvas_height(80.0, 164.0, 80.0),
        164.0
    );
    assert_eq!(
        resolve_panel_transition_canvas_height(196.0, 80.0, 80.0),
        196.0
    );
    assert_eq!(
        resolve_panel_transition_canvas_height(20.0, 30.0, 80.0),
        80.0
    );
}

#[test]
fn next_stacked_card_frame_applies_gap_and_overhang() {
    let mut cursor_y = 180.0;
    let first = resolve_next_stacked_card_frame(&mut cursor_y, false, 60.0, 320.0, 12.0, 4.0);
    let second = resolve_next_stacked_card_frame(&mut cursor_y, true, 70.0, 320.0, 12.0, 4.0);
    let missing = resolve_next_stacked_card_frame(&mut cursor_y, true, 80.0, 320.0, 12.0, 4.0);

    assert_eq!(
        first,
        Some(PanelRect {
            x: -4.0,
            y: 120.0,
            width: 328.0,
            height: 60.0,
        })
    );
    assert_eq!(
        second,
        Some(PanelRect {
            x: -4.0,
            y: 38.0,
            width: 328.0,
            height: 70.0,
        })
    );
    assert_eq!(missing, None);
    assert_eq!(cursor_y, 26.0);
}

#[test]
fn card_metrics_estimate_text_and_body_height() {
    let metrics = test_card_metrics();

    assert_eq!(resolve_card_chat_body_width(390.0, metrics), 355.0);
    assert!((resolve_estimated_text_width("Aa, 中", 10.0) - 30.2).abs() < 0.0001);
    assert_eq!(
        resolve_estimated_chat_line_count("short\nsecond line", 60.0, 3),
        3
    );
    assert_eq!(
        resolve_estimated_chat_body_height("short\nsecond line", 60.0, 3, metrics),
        42.0
    );
}

#[test]
fn card_metrics_resolve_card_heights() {
    let metrics = test_card_metrics();

    assert_eq!(
        resolve_pending_like_card_height("needs approval", 92.0, 120.0, 355.0, metrics),
        105.0
    );
    assert_eq!(resolve_session_card_collapsed_height(100.0, true), 52.0);
    assert_eq!(resolve_session_card_collapsed_height(100.0, false), 58.0);

    let content_height = resolve_session_card_content_height(SessionCardContentInput {
        prompt: Some("prompt"),
        reply: Some("reply"),
        has_tool: true,
        default_body_width: 355.0,
        metrics,
    });
    assert_eq!(content_height, 67.0);
    assert_eq!(
        resolve_session_card_height(false, true, content_height, metrics),
        119.0
    );
    assert_eq!(
        resolve_session_card_height(true, true, content_height, metrics),
        58.0
    );
    assert_eq!(
        resolve_completion_card_height("Task complete", 355.0, metrics),
        76.0
    );
}

#[test]
fn stacked_cards_total_height_uses_empty_height_and_gap_sum() {
    assert_eq!(resolve_stacked_cards_total_height(&[], 12.0, 84.0), 84.0);
    assert_eq!(
        resolve_stacked_cards_total_height(&[92.0, 108.0, 76.0], 12.0, 84.0),
        300.0
    );
    assert_eq!(
        resolve_stacked_cards_total_height(&[92.0, 108.0, 76.0], EXPANDED_CARD_GAP, 84.0),
        292.0
    );
}
