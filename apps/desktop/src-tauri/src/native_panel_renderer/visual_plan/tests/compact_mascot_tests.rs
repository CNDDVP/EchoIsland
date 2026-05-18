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
fn compact_visual_plan_draws_pill_and_shoulders_without_canvas_block() {
    let input = visual_input(NativePanelVisualDisplayMode::Compact);
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.hidden);
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect {
            frame,
            radius,
            ..
        } if *frame == input.compact_bar_frame
            && (*radius - crate::native_panel_core::COMPACT_PILL_RADIUS).abs() < 0.001
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::CompactShoulder {
                frame,
                side: crate::native_panel_renderer::visual_primitives::NativePanelVisualShoulderSide::Left,
                progress,
                ..
            } if *frame == input.left_shoulder_frame && *progress == 0.0
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::CompactShoulder {
                frame,
                side: crate::native_panel_renderer::visual_primitives::NativePanelVisualShoulderSide::Right,
                progress,
                ..
            } if *frame == input.right_shoulder_frame && *progress == 0.0
        )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect { frame, .. } if *frame == input.content_frame
    )));
}

#[test]
fn compact_visual_plan_does_not_draw_completion_glow_as_large_panel_block() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.glow_visible = true;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::CompletionGlow { frame, opacity }
            if *frame == input.compact_bar_frame && *opacity > 0.0
    )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect { frame, .. }
            if frame.width > input.compact_bar_frame.width
                || frame.height > input.compact_bar_frame.height
    )));
}

#[test]
fn compact_visual_plan_places_mascot_headline_and_count_text() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.completion_count = 0;
    input.mascot_pose = SceneMascotPose::Idle;
    input.compact_bar_frame.width = 253.0;
    input.compact_bar_frame.height = 37.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::MascotDot { center, radius, .. }
                if (center.x - (input.compact_bar_frame.x + 22.0)).abs() < 0.001
                    && (center.y - (input.compact_bar_frame.y + input.compact_bar_frame.height / 2.0)).abs() < 0.001
                    && (*radius - 11.0).abs() < 0.001
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text {
                origin,
                max_width,
                text,
                size,
                weight,
                alignment,
                ..
            } if text == "Codex ready"
                && ((origin.x + max_width / 2.0)
                    - (input.compact_bar_frame.x + input.compact_bar_frame.width / 2.0)).abs() < 0.001
                && (*max_width - crate::native_panel_core::resolve_estimated_text_width("Codex ready", 13.0)).abs() < 0.001
                && *size == 13
                && *weight == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Semibold
                && *alignment == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Center
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { origin, text, size, weight, alignment, .. }
                if text == "1"
                    && (origin.x - (input.compact_bar_frame.x + 197.0)).abs() < 0.001
                    && *size == 15
                    && *weight == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Semibold
                    && *alignment == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Right
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { origin, text, size, weight, alignment, .. }
                if text == "/"
                    && (origin.x - (input.compact_bar_frame.x + 217.0)).abs() < 0.001
                    && *size == 15
                    && *weight == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Semibold
                    && *alignment == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Center
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { origin, text, size, weight, alignment, .. }
                if text == "3"
                    && (origin.x - (input.compact_bar_frame.x + 229.0)).abs() < 0.001
                    && *size == 15
                    && *weight == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Semibold
                    && *alignment == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Left
        )));
}

#[test]
fn compact_visual_plan_omits_mascot_primitives_when_mascot_is_hidden() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.mascot_pose = SceneMascotPose::Hidden;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotDot { .. }
            | NativePanelVisualPrimitive::MascotEllipse { .. }
            | NativePanelVisualPrimitive::MascotRoundRect { .. }
            | NativePanelVisualPrimitive::MascotText { .. }
    )));
}

#[test]
fn compact_visual_plan_marks_compact_text_with_stable_roles() {
    let input = visual_input(NativePanelVisualDisplayMode::Compact);
    let plan = resolve_native_panel_visual_plan(&input);

    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::CompactHeadline),
        1
    );
    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::CompactActiveCount),
        1
    );
    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::CompactSlash),
        1
    );
    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::CompactTotalCount),
        1
    );
    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::ActionButtonSettings),
        0
    );
    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::ActionButtonQuit),
        0
    );
}

#[test]
fn expanded_visual_plan_marks_action_button_text_with_stable_roles() {
    let input = visual_input(NativePanelVisualDisplayMode::Expanded);
    let plan = resolve_native_panel_visual_plan(&input);

    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::ActionButtonSettings),
        1
    );
    assert_eq!(
        text_role_count(&plan, NativePanelVisualTextRole::ActionButtonQuit),
        1
    );
}

#[test]
fn expanded_visual_plan_uses_settings_icon_color_for_debug_mode() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    for button in &mut input.action_buttons {
        button.debug_mode_enabled = true;
    }

    let plan = resolve_native_panel_visual_plan(&input);
    let settings = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::ActionButtonSettings,
                color,
                ..
            } => Some(*color),
            _ => None,
        })
        .expect("settings icon");
    let quit = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::ActionButtonQuit,
                color,
                ..
            } => Some(*color),
            _ => None,
        })
        .expect("quit icon");

    assert_eq!(settings, NativePanelVisualColor::rgb(102, 222, 145));
    assert_eq!(quit, NativePanelVisualColor::rgb(255, 82, 82));
}

#[test]
fn expanded_visual_plan_marks_card_content_with_stable_roles() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 180.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(text_role_count(&plan, NativePanelVisualTextRole::CardTitle) >= 1);
    assert!(text_role_count(&plan, NativePanelVisualTextRole::CardSubtitle) >= 1);
    assert!(text_role_count(&plan, NativePanelVisualTextRole::CardStatusBadge) >= 1);
    assert!(text_role_count(&plan, NativePanelVisualTextRole::CardSourceBadge) >= 1);
    assert!(text_role_count(&plan, NativePanelVisualTextRole::CardBodyPrefix) >= 1);
    assert!(text_role_count(&plan, NativePanelVisualTextRole::CardBodyText) >= 1);
}

#[test]
fn compact_visual_plan_uses_shared_active_count_marquee_frame() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.compact_bar_frame.width = 253.0;
    input.compact_bar_frame.height = 37.0;
    input.active_count = "23".to_string();
    input.active_count_elapsed_ms = ACTIVE_COUNT_SCROLL_HOLD_MS + (ACTIVE_COUNT_SCROLL_MOVE_MS / 2);
    let plan = resolve_native_panel_visual_plan(&input);
    let current = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text { origin, text, .. } if text == "2" => Some(*origin),
            _ => None,
        })
        .expect("current active count digit");
    let next = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text { origin, text, .. } if text == "3" => Some(*origin),
            _ => None,
        })
        .expect("next active count digit");

    assert!(
        current.y < input.compact_bar_frame.y + compact_digit_y(input.compact_bar_frame.height)
    );
    assert!(next.y > current.y);
}

#[test]
fn compact_visual_plan_positions_active_count_marquee_in_slot() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.compact_bar_frame.width = 253.0;
    input.compact_bar_frame.height = 37.0;
    input.active_count = "22".to_string();
    input.total_count = "3".to_string();
    input.active_count_elapsed_ms = 0;
    let plan = resolve_native_panel_visual_plan(&input);
    let compact_content = crate::native_panel_core::resolve_compact_bar_content_layout(
        crate::native_panel_core::CompactBarContentLayoutInput {
            bar_width: input.compact_bar_frame.width,
            bar_height: input.compact_bar_frame.height,
        },
    );

    let NativePanelVisualPrimitive::Text {
        origin,
        max_width,
        alignment,
        ..
    } = text_primitive(&plan, "2")
    else {
        panic!("active count text should be text primitive");
    };

    assert!(
        (origin.x
            - (input.compact_bar_frame.x + compact_content.active_x + ACTIVE_COUNT_TEXT_OFFSET_X))
            .abs()
            < 0.001
    );
    assert!((*max_width - crate::native_panel_core::ACTIVE_COUNT_TEXT_WIDTH).abs() < 0.001);
    assert_eq!(
        *alignment,
        crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Right
    );
}

#[test]
fn compact_visual_plan_clips_headline_to_single_line() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.headline_text =
            "apps/desktop/src-tauri/src/windows_native_panel/host_runtime.rs\nsrc/native_panel_renderer"
                .to_string();
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, size, .. }
            if *size == 13
                && !text.contains('\n')
                && text.starts_with("apps/desktop")
                && text.ends_with("...")
    )));
}

#[test]
fn compact_visual_plan_keeps_headline_stable_when_expanded_actions_are_visible() {
    let mut compact = visual_input(NativePanelVisualDisplayMode::Compact);
    compact.completion_count = 0;
    compact.compact_bar_frame.width = 253.0;
    compact.action_buttons_visible = false;

    let mut expanded = compact.clone();
    expanded.display_mode = NativePanelVisualDisplayMode::Expanded;
    expanded.compact_bar_frame.x -= 15.0;
    expanded.compact_bar_frame.width = 283.0;
    expanded.action_buttons_visible = true;

    let compact_plan = resolve_native_panel_visual_plan(&compact);
    let expanded_plan = resolve_native_panel_visual_plan(&expanded);

    assert_eq!(
        headline_text_frame(&compact_plan),
        headline_text_frame(&expanded_plan)
    );
}

#[test]
fn compact_visual_plan_keeps_headline_center_stable_for_compact_width_preset() {
    let mut compact = visual_input(NativePanelVisualDisplayMode::Compact);
    compact.completion_count = 0;
    compact.compact_bar_frame.width = 233.0;
    compact.action_buttons_visible = false;

    let mut expanded = compact.clone();
    expanded.display_mode = NativePanelVisualDisplayMode::Expanded;
    expanded.compact_bar_frame.x -= 15.0;
    expanded.compact_bar_frame.width = 263.0;
    expanded.action_buttons_visible = true;

    let compact_plan = resolve_native_panel_visual_plan(&compact);
    let expanded_plan = resolve_native_panel_visual_plan(&expanded);
    let (_, _, compact_center_x) = headline_text_frame(&compact_plan);
    let (_, _, expanded_center_x) = headline_text_frame(&expanded_plan);

    assert!((compact_center_x - expanded_center_x).abs() <= 0.001);
}

#[test]
fn compact_visual_plan_places_mascot_face_in_mac_coordinate_order() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.completion_count = 0;
    input.mascot_pose = SceneMascotPose::Complete;
    let plan = resolve_native_panel_visual_plan(&input);
    let mascot_center = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::MascotDot { center, .. } => Some(*center),
            _ => None,
        })
        .expect("mascot primitive");

    let face_color =
        crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(255, 255, 255);
    let eye_centers = plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            NativePanelVisualPrimitive::MascotEllipse { frame, color, .. }
                if *color == face_color =>
            {
                Some(frame.y + frame.height / 2.0)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(eye_centers.len(), 2);
    assert!(eye_centers.iter().all(|eye_y| *eye_y > mascot_center.y));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotRoundRect { role, frame, color, .. }
            if *role == NativePanelVisualMascotRoundRectRole::Mouth
                && *color == face_color
                && frame.y + frame.height / 2.0 < mascot_center.y
    )));
}

#[test]
fn compact_visual_plan_uses_shared_mascot_visual_frame_for_blink() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.completion_count = 0;
    input.mascot_pose = SceneMascotPose::Idle;
    let mut open_input = input.clone();
    open_input.mascot_elapsed_ms = 0;
    input.mascot_elapsed_ms = 4535;
    let open_plan = resolve_native_panel_visual_plan(&open_input);
    let plan = resolve_native_panel_visual_plan(&input);
    let face_color =
        crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(255, 255, 255);
    let open_eye_heights = open_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            NativePanelVisualPrimitive::MascotEllipse { frame, color, .. }
                if *color == face_color =>
            {
                Some(frame.height)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let eye_heights = plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            NativePanelVisualPrimitive::MascotEllipse { frame, color, .. }
                if *color == face_color =>
            {
                Some(frame.height)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(eye_heights.len(), 2);
    assert_eq!(open_eye_heights.len(), 2);
    assert!(
        eye_heights
            .iter()
            .zip(open_eye_heights.iter())
            .all(|(closed, open)| *closed < *open)
    );
}

#[test]
fn compact_visual_plan_uses_mac_style_non_uniform_mascot_motion() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.completion_count = 0;
    input.mascot_pose = SceneMascotPose::Running;
    input.mascot_elapsed_ms = 100;
    let plan = resolve_native_panel_visual_plan(&input);

    let NativePanelVisualPrimitive::MascotDot {
        center,
        radius,
        scale_x,
        scale_y,
        ..
    } = plan
        .primitives
        .iter()
        .find(|primitive| matches!(primitive, NativePanelVisualPrimitive::MascotDot { .. }))
        .expect("mascot primitive")
    else {
        panic!("mascot primitive should be MascotDot");
    };

    let compact_content = crate::native_panel_core::resolve_compact_bar_content_layout(
        crate::native_panel_core::CompactBarContentLayoutInput {
            bar_width: input.compact_bar_frame.width,
            bar_height: input.compact_bar_frame.height,
        },
    );
    assert!(center.x > input.compact_bar_frame.x + compact_content.mascot_center_x);
    assert!((*scale_x - *scale_y).abs() > 0.02);
    assert!((*radius - 11.0).abs() < 0.001);
}

#[test]
fn compact_visual_plan_carries_shared_mascot_body_style() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.completion_count = 0;
    input.mascot_pose = SceneMascotPose::Sleepy;
    let plan = resolve_native_panel_visual_plan(&input);

    let NativePanelVisualPrimitive::MascotDot {
        frame,
        corner_radius,
        fill,
        stroke,
        stroke_width,
        ..
    } = plan
        .primitives
        .iter()
        .find(|primitive| matches!(primitive, NativePanelVisualPrimitive::MascotDot { .. }))
        .expect("mascot primitive")
    else {
        panic!("mascot primitive should be MascotDot");
    };

    assert!(frame.width > frame.height);
    assert!((*corner_radius - (frame.width.min(frame.height) * 0.28).max(4.0)).abs() < 0.001);
    assert_eq!(*fill, NativePanelVisualColor::rgb(3, 3, 3));
    assert_eq!(*stroke, NativePanelVisualColor::rgb(255, 122, 36));
    assert_eq!(*stroke_width, 2.2);
}

#[test]
fn compact_visual_plan_draws_mac_sleepy_and_wake_angry_mascot_details() {
    let mut sleepy_input = visual_input(NativePanelVisualDisplayMode::Compact);
    sleepy_input.completion_count = 0;
    sleepy_input.mascot_pose = SceneMascotPose::Sleepy;
    sleepy_input.mascot_elapsed_ms = 4550;
    let sleepy_plan = resolve_native_panel_visual_plan(&sleepy_input);

    assert!(sleepy_plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotText { role, text, .. }
            if *role == NativePanelVisualMascotTextRole::SleepLabel && text == "Z"
    )));

    let mut wake_input = visual_input(NativePanelVisualDisplayMode::Compact);
    wake_input.completion_count = 0;
    wake_input.mascot_pose = SceneMascotPose::WakeAngry;
    wake_input.mascot_elapsed_ms = 0;
    let wake_plan = resolve_native_panel_visual_plan(&wake_input);

    assert!(wake_plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotDot { scale_x, scale_y, .. }
            if *scale_x > 1.04 && *scale_y < 0.97
    )));
}

#[test]
fn compact_visual_plan_places_completion_badge_on_mascot_and_keeps_active_count() {
    let input = visual_input(NativePanelVisualDisplayMode::Compact);
    let plan = resolve_native_panel_visual_plan(&input);
    let mascot_center = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::MascotDot { center, .. } => Some(*center),
            _ => None,
        })
        .expect("mascot primitive");
    let badge_anchor = PanelPoint {
        x: mascot_center.x,
        y: input.compact_bar_frame.y + input.compact_bar_frame.height / 2.0,
    };

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { origin, text, size, .. }
            if text == "1"
                && (origin.x - (input.compact_bar_frame.x + 184.0)).abs() < 0.001
                && *size == 15
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { origin, text, size, .. }
            if text == "/"
                && (origin.x - (input.compact_bar_frame.x + 204.0)).abs() < 0.001
                && *size == 15
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { origin, text, size, .. }
            if text == "3"
                && (origin.x - (input.compact_bar_frame.x + 216.0)).abs() < 0.001
                && *size == 15
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::MascotRoundRect {
                role,
                frame,
                radius,
                color,
                ..
            } if *role == NativePanelVisualMascotRoundRectRole::CompletionBadgeOutline
                && (frame.x - (badge_anchor.x + 5.0)).abs() < 0.001
                && (frame.y - (badge_anchor.y + 1.0)).abs() < 0.001
                && (frame.width - 15.0).abs() < 0.001
                && (frame.height - 15.0).abs() < 0.001
                && (*radius - 7.5).abs() < 0.001
                && *color == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(240, 255, 246)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::MascotRoundRect {
                role,
                frame,
                radius,
                color,
                ..
            } if *role == NativePanelVisualMascotRoundRectRole::CompletionBadgeFill
                && (frame.x - (badge_anchor.x + 6.0)).abs() < 0.001
                && (frame.y - (badge_anchor.y + 2.0)).abs() < 0.001
                && (frame.width - 13.0).abs() < 0.001
                && (frame.height - 13.0).abs() < 0.001
                && (*radius - 6.5).abs() < 0.001
                && *color == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(102, 222, 145)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotText { role, origin, text, size, .. }
            if *role == NativePanelVisualMascotTextRole::CompletionBadgeLabel
                && text == "2"
                && (origin.x - (badge_anchor.x + 10.0)).abs() < 0.001
                && (origin.y - (badge_anchor.y + 0.5)).abs() < 0.001
                && *size == 8
    )));
}

#[test]
fn compact_visual_plan_keeps_completion_badge_across_non_complete_mascot_poses() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Compact);
    input.completion_count = 2;
    input.mascot_pose = SceneMascotPose::MessageBubble;
    input.mascot_elapsed_ms = 500;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::MascotRoundRect { role, color, .. }
                if *role == NativePanelVisualMascotRoundRectRole::MessageBubble
                    && *color == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(102, 222, 145)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotText { role, text, size, .. }
            if *role == NativePanelVisualMascotTextRole::CompletionBadgeLabel
                && text == "2" && *size == 8
    )));
}

#[test]
fn compact_visual_plan_animates_message_bubble_like_mac_pop() {
    let mut early = visual_input(NativePanelVisualDisplayMode::Compact);
    early.completion_count = 0;
    early.mascot_pose = SceneMascotPose::MessageBubble;
    early.mascot_elapsed_ms = 120;
    let early_plan = resolve_native_panel_visual_plan(&early);

    let mut open = early.clone();
    open.mascot_elapsed_ms = 500;
    let open_plan = resolve_native_panel_visual_plan(&open);

    let early_bubble = mascot_green_bubble_frame(&early_plan).expect("early bubble");
    let open_bubble = mascot_green_bubble_frame(&open_plan).expect("open bubble");
    assert!(open_bubble.y > early_bubble.y);
}

#[test]
fn compact_visual_plan_animates_sleep_label_like_mac_rise() {
    let mut early = visual_input(NativePanelVisualDisplayMode::Compact);
    early.mascot_pose = SceneMascotPose::Sleepy;
    early.mascot_elapsed_ms = 200;
    let early_plan = resolve_native_panel_visual_plan(&early);

    let mut risen = early.clone();
    risen.mascot_elapsed_ms = 1500;
    let risen_plan = resolve_native_panel_visual_plan(&risen);

    let early_origin = mascot_sleep_label_origin(&early_plan).expect("early sleep label");
    let risen_origin = mascot_sleep_label_origin(&risen_plan).expect("risen sleep label");
    assert!(risen_origin.x > early_origin.x);
    assert!(risen_origin.y > early_origin.y);
}

#[test]
fn expanded_visual_plan_hides_completion_badge_even_when_completion_count_is_cached() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.completion_count = 2;
    input.mascot_pose = SceneMascotPose::Complete;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::MascotRoundRect { role, color, .. }
                if *role == NativePanelVisualMascotRoundRectRole::CompletionBadgeFill
                    && *color == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(102, 222, 145)
        )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotText { role, text, size, .. }
            if *role == NativePanelVisualMascotTextRole::CompletionBadgeLabel
                && text == "2" && *size == 8
    )));
}

#[test]
fn expanded_visual_plan_hides_mascot_bubble_for_status_message_state() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.completion_count = 2;
    input.mascot_pose = SceneMascotPose::MessageBubble;
    input.mascot_elapsed_ms = 500;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::MascotRoundRect { role, color, .. }
                if *role == NativePanelVisualMascotRoundRectRole::MessageBubble
                    && *color == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(102, 222, 145)
        )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::MascotText { role, text, size, .. }
            if *role == NativePanelVisualMascotTextRole::CompletionBadgeLabel
                && text == "2" && *size == 8
    )));
}

#[test]
fn status_visual_plan_keeps_compact_mascot_and_counts_without_action_buttons() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.surface = ExpandedSurface::Status;
    input.active_count = "2".to_string();
    input.total_count = "4".to_string();
    input.action_buttons_visible = true;
    input.chrome_transition_progress = 1.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(
        plan.primitives
            .iter()
            .any(|primitive| matches!(primitive, NativePanelVisualPrimitive::MascotDot { .. }))
    );
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CompactActiveCount,
            text,
            ..
        } if text == "2"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CompactTotalCount,
            text,
            ..
        } if text == "4"
    )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::ActionButtonSettings
                | NativePanelVisualTextRole::ActionButtonQuit,
            ..
        }
    )));
}
