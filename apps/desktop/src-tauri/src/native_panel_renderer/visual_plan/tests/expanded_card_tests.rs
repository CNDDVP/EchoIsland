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
fn expanded_visual_plan_draws_card_content_from_shared_inputs() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 180.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, weight, .. }
                if text == "Settings"
                    && *weight == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Semibold
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "EchoIsland v0.6.1"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Mute Sound"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Off"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Done"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Task complete"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Codex"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect { frame, radius, .. }
            if frame.width > 20.0
                && (frame.height - 22.0).abs() < 0.001
                && (*radius - 11.0).abs() < 0.001
    )));
}

#[test]
fn expanded_visual_plan_draws_settings_rows_as_surfaces_with_value_badges() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 180.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, radius, color }
                if (frame.height - crate::native_panel_core::SETTINGS_ROW_HEIGHT).abs() < 0.001
                    && (*radius - 8.0).abs() < 0.001
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(50, 84, 61)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, radius, color }
                if (frame.height - 18.0).abs() < 0.001
                    && (*radius - 9.0).abs() < 0.001
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(46, 68, 54)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, size, alignment, .. }
                if text == "Off"
                    && *size == 10
                    && *alignment
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Center
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(104, 222, 145)
        )));
}

#[test]
fn expanded_visual_plan_matches_mac_session_card_density_and_clips_long_body() {
    let long_body =
        "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz";
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 180.0;
    input.cards[1].title = "Finished".to_string();
    input.cards[1].body = Some(long_body.to_string());
    let plan = resolve_native_panel_visual_plan(&input);

    let NativePanelVisualPrimitive::Text { size, weight, .. } = text_primitive(&plan, "Finished")
    else {
        panic!("expected title text");
    };
    assert_eq!(*size, 12);
    assert_eq!(
        *weight,
        crate::native_panel_renderer::visual_primitives::NativePanelVisualTextWeight::Semibold
    );

    let NativePanelVisualPrimitive::Text { size, .. } = plan
            .primitives
            .iter()
            .find(|primitive| {
                matches!(primitive, NativePanelVisualPrimitive::Text { text, .. } if text.starts_with("#abcdef"))
            })
            .expect("expected meta text")
        else {
            panic!("expected meta text");
        };
    assert_eq!(*size, 9);

    let NativePanelVisualPrimitive::Text { size, .. } = text_primitive(&plan, "Codex") else {
        panic!("expected source badge text");
    };
    assert_eq!(*size, 10);
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect { frame, radius, .. }
            if (frame.height - 22.0).abs() < 0.001
                && (*radius - 11.0).abs() < 0.001
    )));

    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == long_body
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, size, .. }
            if text.starts_with("abcdefghijklmnopqrstuvwxyz")
                && text.contains('\n')
                && text.lines().count() == 2
                && *size == 10
    )));
}

#[test]
fn expanded_visual_plan_uses_mac_chat_line_tones_for_default_and_completion_cards() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 180.0;
    input.card_stack_content_height = 76.0;
    input.cards = vec![NativePanelVisualCardInput {
        style:
            crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Default,
        title: "EchoIsland".to_string(),
        subtitle: Some("#c1d5-7 · now".to_string()),
        body: Some("Assistant reply".to_string()),
        badge: Some(NativePanelVisualCardBadgeInput {
            text: "Idle".to_string(),
            emphasized: false,
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
        collapsed_height: 64.0,
        compact: false,
        removing: false,
    }];
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == "$"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(217, 120, 87)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == "Assistant reply"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(177, 183, 194)
        )));

    input.cards[0].style =
        crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Completion;
    let plan = resolve_native_panel_visual_plan(&input);
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == "$"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(104, 222, 145)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, color, .. }
                if *color
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(37, 37, 41)
                    && frame.height > 60.0
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, color, .. }
                if *color
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(46, 79, 61)
                    && frame.height > 60.0
        )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, color, .. }
                if *color
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(30, 40, 38)
                    && frame.height > 60.0
        )));
}

#[test]
fn expanded_visual_plan_clips_title_before_source_badge_on_narrow_cards() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame = PanelRect {
        x: 36.0,
        y: 36.0,
        width: 230.0,
        height: 112.0,
    };
    input.card_stack_content_height = 112.0;
    input.cards = vec![NativePanelVisualCardInput {
        style: NativePanelVisualCardStyle::Completion,
        title: "bone_fix_change_display_for_long_running_task".to_string(),
        subtitle: Some("#f3de-7 · gpt-5.5 · now".to_string()),
        body: Some("可以，但要看你的 Unity 动画是什么格式。".to_string()),
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
        height: 100.0,
        collapsed_height: 64.0,
        compact: false,
        removing: false,
    }];

    let plan = resolve_native_panel_visual_plan(&input);
    let title = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::CardTitle,
                origin,
                max_width,
                text,
                ..
            } => Some((*origin, *max_width, text.clone())),
            _ => None,
        })
        .expect("card title");
    let source_badge_text = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::CardSourceBadge,
                origin,
                ..
            } => Some(*origin),
            _ => None,
        })
        .expect("source badge text");

    assert!(
        title.0.x + title.1 <= source_badge_text.x - 6.0,
        "title max width must stop before source badge: title={:?}, source_text={:?}",
        title,
        source_badge_text
    );
    assert!(
        title.2.ends_with("..."),
        "narrow title should be ellipsized: {:?}",
        title.2
    );
}

#[test]
fn expanded_visual_plan_draws_session_reply_and_prompt_lines() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 112.0;
    input.card_stack_content_height = 112.0;
    input.cards = vec![NativePanelVisualCardInput {
        style:
            crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Default,
        title: "EchoIsland".to_string(),
        subtitle: Some("#c1d5-7 路 7m".to_string()),
        body: Some("Assistant reply".to_string() + "\n" + "User prompt"),
        badge: Some(NativePanelVisualCardBadgeInput {
            text: "Idle".to_string(),
            emphasized: false,
        }),
        source_badge: Some(NativePanelVisualCardBadgeInput {
            text: "Codex".to_string(),
            emphasized: false,
        }),
        body_prefix: Some("$>".to_string()),
        body_lines: Vec::new(),
        action_hint: None,
        rows: Vec::new(),
        height: 112.0,
        collapsed_height: 64.0,
        compact: false,
        removing: false,
    }];
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == "$"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(217, 120, 87)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == ">"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(104, 222, 145)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Assistant reply"
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "User prompt"
    )));
}

#[test]
fn expanded_visual_plan_aligns_body_prefix_to_first_wrapped_text_line() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame = PanelRect {
        x: 0.0,
        y: 0.0,
        width: 190.0,
        height: 118.0,
    };
    input.card_stack_content_height = 118.0;
    input.cards = vec![NativePanelVisualCardInput {
        style:
            crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Default,
        title: "EchoIsland".to_string(),
        subtitle: Some("#c1d5-7".to_string()),
        body: None,
        badge: Some(NativePanelVisualCardBadgeInput {
            text: "Idle".to_string(),
            emphasized: false,
        }),
        source_badge: Some(NativePanelVisualCardBadgeInput {
            text: "Codex".to_string(),
            emphasized: false,
        }),
        body_prefix: None,
        body_lines: vec![NativePanelVisualCardBodyLineInput {
            role: NativePanelVisualCardBodyRole::Assistant,
            prefix: Some("$".to_string()),
            text: "This assistant reply should wrap into two visible lines".to_string(),
            max_lines: 2,
        }],
        action_hint: None,
        rows: Vec::new(),
        height: 118.0,
        collapsed_height: 64.0,
        compact: false,
        removing: false,
    }];
    let plan = resolve_native_panel_visual_plan(&input);

    let prefix_y = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                role, text, origin, ..
            } if *role == NativePanelVisualTextRole::CardBodyPrefix && text == "$" => {
                Some(origin.y)
            }
            _ => None,
        })
        .expect("assistant prefix");
    let body_y = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                role, text, origin, ..
            } if *role == NativePanelVisualTextRole::CardBodyText && text.contains('\n') => {
                Some(origin.y)
            }
            _ => None,
        })
        .expect("wrapped assistant body");

    assert_eq!(
        prefix_y, body_y,
        "wrapped body prefixes should stay aligned with the first visible text line"
    );
}

#[test]
fn expanded_visual_plan_keeps_partially_clipped_card_shell_without_content() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 42.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect {
            radius,
            ..
        } if (*radius - crate::native_panel_core::CARD_RADIUS).abs() < 0.001
    )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. }
            if text == "Settings"
                || text == "Mute Sound"
                || text == "Done"
                || text == "Task complete"
    )));
}

#[test]
fn expanded_visual_plan_anchors_overflowing_card_stack_to_visible_bottom() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 180.0;
    input.card_stack_content_height = 260.0;
    input.cards[0].height = 180.0;
    input.cards[0].collapsed_height = 120.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect {
            frame,
            radius,
            ..
        } if (frame.y - (input.shell_frame.y + input.card_stack_frame.y)).abs() < 0.001
            && (frame.height - input.cards[0].height).abs() < 0.001
            && (*radius - crate::native_panel_core::CARD_RADIUS).abs() < 0.001
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Settings"
    )));
}

#[test]
fn expanded_visual_plan_does_not_relayout_content_from_top_clipped_card() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 60.0;
    input.card_stack_content_height = 100.0;
    input.cards = vec![NativePanelVisualCardInput {
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
            height: 100.0,
            collapsed_height: 52.0,
            compact: false,
            removing: false,
        }];
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Done"
    )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Task complete"
    )));
}

#[test]
fn visible_content_extension_clips_partially_visible_text_without_dropping_it() {
    let visible_frame = PanelRect {
        x: 0.0,
        y: 0.0,
        width: 120.0,
        height: 20.0,
    };
    let text = NativePanelVisualPrimitive::Text {
        role: NativePanelVisualTextRole::CardBodyText,
        origin: PanelPoint { x: 8.0, y: 14.0 },
        max_width: 100.0,
        text: "Task complete".to_string(),
        color: NativePanelVisualColor::rgb(245, 247, 252),
        size: 10,
        weight: NativePanelVisualTextWeight::Normal,
        alignment: NativePanelVisualTextAlignment::Left,
        alpha: 1.0,
    };
    let mut output = Vec::new();
    extend_visible_content_primitives(&mut output, vec![text], visible_frame);

    assert!(output.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Task complete"
    )));
    assert!(matches!(
        output.first(),
        Some(NativePanelVisualPrimitive::ClipStart { frame }) if *frame == visible_frame
    ));
    assert!(matches!(
        output.last(),
        Some(NativePanelVisualPrimitive::ClipEnd)
    ));
}

#[test]
fn expanded_visual_plan_centers_empty_card_prompt() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_content_height = 84.0;
    input.cards = vec![NativePanelVisualCardInput {
        style:
            crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Empty,
        title: "No active sessions".to_string(),
        subtitle: None,
        body: Some("EchoIsland is watching for new activity.".to_string()),
        badge: None,
        source_badge: None,
        body_prefix: None,
        body_lines: Vec::new(),
        action_hint: None,
        rows: Vec::new(),
        height: 84.0,
        collapsed_height: 34.0,
        compact: true,
        removing: false,
    }];
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text {
                text,
                origin,
                max_width,
                size,
                alignment,
                ..
            } if text == "No active sessions"
                && (origin.x - (input.shell_frame.x + input.card_stack_frame.x)).abs() < 0.001
                && (*max_width - input.card_stack_frame.width).abs() < 0.001
                && *size == 12
                && *alignment
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualTextAlignment::Center
        )));
}

#[test]
fn expanded_visual_plan_keeps_single_empty_card_when_viewport_is_shorter_than_empty_height() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 70.0;
    input.card_stack_content_height = 70.0;
    input.cards = vec![NativePanelVisualCardInput {
        style:
            crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Empty,
        title: "No active sessions".to_string(),
        subtitle: None,
        body: Some("EchoIsland is watching for new activity.".to_string()),
        badge: None,
        source_badge: None,
        body_prefix: None,
        body_lines: Vec::new(),
        action_hint: None,
        rows: Vec::new(),
        height: crate::native_panel_core::EMPTY_CARD_HEIGHT,
        collapsed_height: 34.0,
        compact: true,
        removing: false,
    }];
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect {
            frame,
            radius,
            ..
        } if (frame.height - input.card_stack_frame.height).abs() < 0.001
            && (*radius - crate::native_panel_core::CARD_RADIUS).abs() < 0.001
    )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "No active sessions"
    )));
}

#[test]
fn expanded_visual_plan_does_not_fill_transparent_canvas_background() {
    let input = visual_input(NativePanelVisualDisplayMode::Expanded);
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect {
            frame,
            color,
            ..
        } if *frame == input.content_frame
            && *color
                == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(
                    18, 18, 22,
                )
    )));
}

#[test]
fn expanded_visual_plan_keeps_shell_color_stable_with_compact_island() {
    let input = visual_input(NativePanelVisualDisplayMode::Expanded);
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect {
            frame,
            radius,
            color,
        } if *frame == input.shell_frame
            && (*radius - crate::native_panel_core::EXPANDED_PANEL_RADIUS).abs() < 0.001
            && *color
                == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(
                    12, 12, 15,
                )
    )));
}

#[test]
fn expanded_visual_plan_draws_card_shells_from_shared_stack_layout() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_content_height = 176.0;
    input.card_stack_frame.height = input.card_stack_content_height;
    let plan = resolve_native_panel_visual_plan(&input);

    let card_shells = plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            NativePanelVisualPrimitive::RoundRect { frame, radius, .. }
                if frame.width > 80.0
                    && (*radius - crate::native_panel_core::CARD_RADIUS).abs() < 0.001 =>
            {
                Some(*frame)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        card_shells,
        vec![
            PanelRect {
                x: input.shell_frame.x + input.card_stack_frame.x,
                y: input.shell_frame.y + input.card_stack_frame.y + 84.0,
                width: input.card_stack_frame.width,
                height: 92.0,
            },
            PanelRect {
                x: input.shell_frame.x + input.card_stack_frame.x,
                y: input.shell_frame.y + input.card_stack_frame.y,
                width: input.card_stack_frame.width,
                height: 76.0,
            },
        ]
    );
}

#[test]
fn expanded_visual_plan_reveals_card_shells_with_staggered_collapsed_height() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.44;
    input.card_stack_frame.height = input.card_stack_content_height;
    let plan = resolve_native_panel_visual_plan(&input);

    let card_shells = plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            NativePanelVisualPrimitive::RoundRect { frame, radius, .. }
                if frame.width > 80.0
                    && (*radius - crate::native_panel_core::CARD_RADIUS).abs() < 0.001 =>
            {
                Some(*frame)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(card_shells.len(), 2);
    assert!(card_shells[0].height > input.cards[0].collapsed_height);
    assert!(card_shells[0].height < input.cards[0].height);
    assert!(card_shells[1].height > input.cards[1].collapsed_height);
    assert!(card_shells[1].height < input.cards[1].height);
}

#[test]
fn expanded_visual_plan_hides_card_shells_before_card_reveal_progress() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.0;
    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::RoundRect {
            radius,
            ..
        } if (*radius - crate::native_panel_core::CARD_RADIUS).abs() < 0.001
    )));
}
