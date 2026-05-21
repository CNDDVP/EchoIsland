#![allow(unused_imports)]

use super::super::{
    NativePanelVisualActionButtonInput, NativePanelVisualCardBadgeInput,
    NativePanelVisualCardBodyLineInput, NativePanelVisualCardBodyRole, NativePanelVisualCardInput,
    NativePanelVisualCardRowInput, NativePanelVisualCardStyle, NativePanelVisualDisplayMode,
    NativePanelVisualPlan, NativePanelVisualPlanInput, compact_digit_y,
    extend_visible_content_primitives, native_panel_visual_card_input_from_scene_card_with_height,
    resolve_native_panel_visual_plan,
};
use super::common::*;
use crate::{
    native_panel_core::{
        ACTIVE_COUNT_SCROLL_HOLD_MS, ACTIVE_COUNT_SCROLL_MOVE_MS, ACTIVE_COUNT_TEXT_OFFSET_X,
        ExpandedSurface, PanelPoint, PanelRect,
    },
    native_panel_renderer::{
        descriptors::{NativePanelEdgeAction, NativePanelHostWindowState},
        visual_primitives::{
            NativePanelVisualColor, NativePanelVisualMascotEllipseRole,
            NativePanelVisualMascotRoundRectRole, NativePanelVisualMascotTextRole,
            NativePanelVisualPrimitive, NativePanelVisualTextAlignment, NativePanelVisualTextRole,
            NativePanelVisualTextWeight, native_panel_visual_text_box_height,
        },
    },
    native_panel_scene::{SceneBadge, SceneCard, SceneMascotPose},
};
use chrono::Utc;
use echoisland_runtime::{PendingPermissionView, PendingQuestionView, SessionSnapshotView};

#[test]
fn visual_plan_is_empty_when_hidden() {
    let plan =
        resolve_native_panel_visual_plan(&visual_input(NativePanelVisualDisplayMode::Hidden));

    assert!(plan.hidden);
    assert!(plan.primitives.is_empty());
}

#[test]
fn visual_plan_contains_panel_content_and_icons() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.card_stack_frame.height = input.card_stack_content_height;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.hidden);
    assert!(plan.primitives.iter().any(|primitive| {
        matches!(primitive, NativePanelVisualPrimitive::Text { text, .. } if text == "Codex ready")
    }));
    assert!(!plan.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::MascotDot {
                pose: SceneMascotPose::Complete,
                ..
            }
        )
    }));
    assert!(plan.primitives.iter().any(|primitive| {
            matches!(primitive, NativePanelVisualPrimitive::Text { text, .. } if text == SETTINGS_ACTION_ICON_TEXT)
        }));
    assert!(plan.primitives.iter().any(|primitive| {
            matches!(primitive, NativePanelVisualPrimitive::Text { text, .. } if text == QUIT_ACTION_ICON_TEXT)
        }));
    assert!(plan.primitives.iter().any(|primitive| {
        matches!(primitive, NativePanelVisualPrimitive::Text { text, .. } if text == "Done")
    }));
}

#[test]
fn expanded_visual_plan_draws_action_icons_from_mac_button_layout_not_wide_hit_regions() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    let compact = input.compact_bar_frame;
    input.action_buttons = vec![
        NativePanelVisualActionButtonInput {
            action: NativePanelEdgeAction::Settings,
            frame: PanelRect {
                x: compact.x,
                y: compact.y,
                width: 58.0,
                height: compact.height,
            },
            debug_mode_enabled: false,
        },
        NativePanelVisualActionButtonInput {
            action: NativePanelEdgeAction::Quit,
            frame: PanelRect {
                x: compact.x + compact.width - 58.0,
                y: compact.y,
                width: 58.0,
                height: compact.height,
            },
            debug_mode_enabled: false,
        },
    ];

    let plan = resolve_native_panel_visual_plan(&input);
    let settings_center = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                origin,
                max_width,
                text,
                ..
            } if text == SETTINGS_ACTION_ICON_TEXT => Some(crate::native_panel_core::PanelPoint {
                x: origin.x + max_width / 2.0,
                y: origin.y,
            }),
            _ => None,
        })
        .expect("settings icon center");
    let quit_center = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                origin,
                max_width,
                text,
                ..
            } if text == QUIT_ACTION_ICON_TEXT => Some(crate::native_panel_core::PanelPoint {
                x: origin.x + max_width / 2.0,
                y: origin.y,
            }),
            _ => None,
        })
        .expect("quit icon center");
    let expected = crate::native_panel_core::resolve_compact_action_button_layout(compact);

    assert!(
        !plan
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, NativePanelVisualPrimitive::MascotDot { .. }))
    );
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CompactActiveCount,
            ..
        }
    )));
    assert!(
        (settings_center.x - (expected.settings.x + expected.settings.width / 2.0)).abs() <= 0.001
    );
    assert!((quit_center.x - (expected.quit.x + expected.quit.width / 2.0)).abs() <= 0.001);
}

#[test]
fn expanded_visual_plan_draws_platform_matching_action_icon_glyphs() {
    let input = visual_input(NativePanelVisualDisplayMode::Expanded);
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text {
                text,
                size,
                weight,
                alignment,
                color,
                ..
            } if text == SETTINGS_ACTION_ICON_TEXT
                && *size == SETTINGS_ACTION_ICON_SIZE
                && *weight
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Normal
                && *alignment
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Center
                && *color
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(
                        245, 247, 252,
                    )
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text {
                text,
                size,
                weight,
                alignment,
                color,
                ..
            } if text == QUIT_ACTION_ICON_TEXT
                && *size == QUIT_ACTION_ICON_SIZE
                && *weight
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Bold
                && *alignment
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Center
                && *color
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(
                        255, 82, 82,
                    )
        )));
}

#[test]
fn expanded_visual_plan_draws_action_buttons_above_collapsing_mascot() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.chrome_transition_progress = 0.5;
    input.mascot_pose = SceneMascotPose::Idle;

    let plan = resolve_native_panel_visual_plan(&input);
    let mascot_index = plan
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, NativePanelVisualPrimitive::MascotDot { .. }))
        .expect("collapsing mascot dot");
    let settings_index = plan
        .primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                NativePanelVisualPrimitive::Text {
                    role: NativePanelVisualTextRole::ActionButtonSettings,
                    text,
                    alpha,
                    ..
                } if text == SETTINGS_ACTION_ICON_TEXT && *alpha > 0.0
            )
        })
        .expect("visible settings action icon");

    assert!(settings_index > mascot_index);
}

#[test]
fn expanded_visual_plan_uses_windows_settings_icon_glyph() {
    let input = visual_input(NativePanelVisualDisplayMode::Expanded);
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text {
                text,
                size,
                weight,
                alignment,
                ..
            } if text == "\u{E713}"
                && *size == 16
                && *weight
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Normal
                && *alignment
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Center
        )));
}

#[test]
fn expanded_visual_plan_applies_shared_action_button_transition_phase() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.compact_bar_frame.width = crate::native_panel_core::DEFAULT_COMPACT_PILL_WIDTH;
    input.separator_visibility = 0.0;
    input.chrome_transition_progress = 0.0;
    let hidden_plan = resolve_native_panel_visual_plan(&input);

    match text_primitive(&hidden_plan, SETTINGS_ACTION_ICON_TEXT) {
        NativePanelVisualPrimitive::Text { alpha, .. } => {
            assert_eq!(*alpha, 0.0);
        }
        _ => unreachable!(),
    }

    input.compact_bar_frame.width = crate::native_panel_core::DEFAULT_COMPACT_PILL_WIDTH
        + (crate::native_panel_core::DEFAULT_EXPANDED_PILL_WIDTH
            - crate::native_panel_core::DEFAULT_COMPACT_PILL_WIDTH)
            * 0.75;
    input.chrome_transition_progress = 0.75;
    let mid_plan = resolve_native_panel_visual_plan(&input);
    let mid_settings = text_primitive(&mid_plan, SETTINGS_ACTION_ICON_TEXT);
    let mut full_input = visual_input(NativePanelVisualDisplayMode::Expanded);
    full_input.compact_bar_frame.width = crate::native_panel_core::DEFAULT_EXPANDED_PILL_WIDTH;
    full_input.separator_visibility = 0.0;
    full_input.chrome_transition_progress = 1.0;
    let full_plan = resolve_native_panel_visual_plan(&full_input);
    let full_settings_y = match text_primitive(&full_plan, SETTINGS_ACTION_ICON_TEXT) {
        NativePanelVisualPrimitive::Text { origin, .. } => origin.y,
        _ => unreachable!(),
    };

    match mid_settings {
        NativePanelVisualPrimitive::Text {
            origin,
            color,
            alpha,
            ..
        } => {
            assert!(origin.y < full_settings_y);
            assert_eq!(
                *color,
                crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(
                    245, 247, 252,
                )
            );
            assert!(*alpha > 0.0 && *alpha < 1.0);
        }
        _ => unreachable!(),
    }
}

#[test]
fn expanded_visual_plan_fades_compact_counts_with_alpha_not_color_blend() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.active_count = "1".to_string();
    input.total_count = "3".to_string();
    input.chrome_transition_progress = 0.4;

    let plan = resolve_native_panel_visual_plan(&input);
    let NativePanelVisualPrimitive::Text {
        role: NativePanelVisualTextRole::CompactActiveCount,
        color,
        alpha,
        ..
    } = plan
        .primitives
        .iter()
        .find(|primitive| {
            matches!(
                primitive,
                NativePanelVisualPrimitive::Text {
                    role: NativePanelVisualTextRole::CompactActiveCount,
                    ..
                }
            )
        })
        .expect("active count text")
    else {
        panic!("active count should be text");
    };

    assert_eq!(*color, NativePanelVisualColor::rgb(102, 222, 145));
    assert!((*alpha - 0.6).abs() < 0.001);
}

#[test]
fn expanded_visual_plan_places_headline_after_settings_icon_when_actions_are_visible() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    use_wide_action_button_hit_regions(&mut input);
    input.compact_bar_frame.width = 283.0;
    let plan = resolve_native_panel_visual_plan(&input);
    let settings_right = match text_primitive(&plan, SETTINGS_ACTION_ICON_TEXT) {
        NativePanelVisualPrimitive::Text {
            origin, max_width, ..
        } => origin.x + max_width,
        _ => unreachable!(),
    };
    let headline_visual_left = match text_primitive(&plan, "Codex ready") {
        NativePanelVisualPrimitive::Text {
            origin,
            max_width,
            text,
            size,
            ..
        } => centered_text_visual_bounds(origin.x, *max_width, text, *size).0,
        _ => unreachable!(),
    };

    assert!(settings_right + 4.0 <= headline_visual_left);
}

#[test]
fn expanded_visual_plan_keeps_headline_center_stable_when_actions_are_visible() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.action_buttons_visible = false;
    let base_plan = resolve_native_panel_visual_plan(&input);
    let (_, _, base_headline_center_x) = headline_text_frame(&base_plan);

    use_wide_action_button_hit_regions(&mut input);
    input.action_buttons_visible = true;
    let plan = resolve_native_panel_visual_plan(&input);
    let (_, _, headline_center_x) = headline_text_frame(&plan);

    assert!((headline_center_x - base_headline_center_x).abs() <= 0.001);
}

#[test]
fn expanded_visual_plan_keeps_headline_visible_when_default_chrome_exits() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.surface = ExpandedSurface::Default;
    input.action_buttons_visible = true;
    input.chrome_transition_progress = 1.0;

    let plan = resolve_native_panel_visual_plan(&input);
    let NativePanelVisualPrimitive::Text {
        alpha, role, text, ..
    } = text_primitive(&plan, "Codex ready")
    else {
        panic!("headline should be text");
    };

    assert_eq!(*role, NativePanelVisualTextRole::CompactHeadline);
    assert_eq!(text, "Codex ready");
    assert_eq!(*alpha, 1.0);
}
