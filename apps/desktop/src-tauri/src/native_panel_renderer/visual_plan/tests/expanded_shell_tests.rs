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
fn expanded_visual_plan_draws_question_pending_tone_distinct_from_approval() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 160.0;
    input.card_stack_content_height = 160.0;
    input.cards = vec![
        native_panel_visual_card_input_from_scene_card_with_height(
            &SceneCard::PendingPermission {
                pending: pending_permission(),
                count: 1,
            },
            72.0,
        ),
        native_panel_visual_card_input_from_scene_card_with_height(
            &SceneCard::PendingQuestion {
                pending: pending_question(),
                count: 2,
            },
            72.0,
        ),
    ];

    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { color, .. }
                if *color
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(87, 61, 39)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { color, .. }
                if *color
                    == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(74, 62, 103)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == "?"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(201, 176, 255)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == "2"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(201, 176, 255)
        )));
}

#[test]
fn expanded_visual_plan_draws_pending_action_hint_as_bottom_pill() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 96.0;
    input.card_stack_content_height = 96.0;
    input.cards = vec![native_panel_visual_card_input_from_scene_card_with_height(
        &SceneCard::PendingPermission {
            pending: pending_permission(),
            count: 1,
        },
        72.0,
    )];

    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, radius, color }
                if (frame.height - crate::native_panel_core::CARD_PENDING_ACTION_HEIGHT).abs() < 0.001
                    && (*radius - crate::native_panel_core::CARD_PENDING_ACTION_HEIGHT / 2.0).abs() < 0.001
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(49, 49, 53)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, size, .. }
                if text == "Allow / Deny in terminal"
                    && *size == 10
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(230, 235, 245)
        )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, .. }
                if text == "Allow / Deny in terminal"
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(104, 213, 145)
        )));
}

#[test]
fn expanded_visual_plan_draws_tool_body_role_as_mac_style_pill() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 120.0;
    input.card_stack_content_height = 120.0;
    input.cards = vec![NativePanelVisualCardInput {
        style:
            crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Default,
        title: "EchoIsland".to_string(),
        subtitle: Some("#c1d5-7 · now".to_string()),
        body: Some("Bash cargo test".to_string()),
        badge: Some(NativePanelVisualCardBadgeInput {
            text: "Running".to_string(),
            emphasized: true,
        }),
        source_badge: Some(NativePanelVisualCardBadgeInput {
            text: "Codex".to_string(),
            emphasized: false,
        }),
        body_prefix: Some("!".to_string()),
        body_lines: vec![NativePanelVisualCardBodyLineInput {
            role: NativePanelVisualCardBodyRole::Tool,
            prefix: Some("!".to_string()),
            text: "Bash cargo test".to_string(),
            max_lines: 1,
        }],
        action_hint: None,
        rows: Vec::new(),
        height: 120.0,
        collapsed_height: 64.0,
        compact: false,
        removing: false,
    }];

    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, radius, color }
                if (frame.height - 22.0).abs() < 0.001
                    && (*radius - 5.0).abs() < 0.001
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(60, 60, 64)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::RoundRect { frame, radius, color }
                if (frame.height - 20.0).abs() < 0.001
                    && (*radius - 4.0).abs() < 0.001
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(47, 47, 52)
        )));
    assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, color, size, .. }
                if text == "Bash"
                    && *size == 9
                    && *color
                        == crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(125, 242, 163)
        )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "Bash cargo test"
    )));
}

#[test]
fn expanded_visual_plan_does_not_draw_card_text_outside_clipped_shell() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.88;
    input.card_stack_frame.height = 96.0;
    input.card_stack_content_height = 140.0;
    input.cards = vec![NativePanelVisualCardInput {
        style:
            crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Default,
        title: "EchoIsland".to_string(),
        subtitle: Some("#c1d5-7 路 gpt-5.4 路 7m".to_string()),
        body: None,
        badge: Some(NativePanelVisualCardBadgeInput {
            text: "Idle".to_string(),
            emphasized: false,
        }),
        source_badge: Some(NativePanelVisualCardBadgeInput {
            text: "Codex".to_string(),
            emphasized: true,
        }),
        body_prefix: None,
        body_lines: vec![
            NativePanelVisualCardBodyLineInput {
                role: NativePanelVisualCardBodyRole::User,
                prefix: Some(">".to_string()),
                text: "第二次移出鼠标还是有问题，之前没有抽代码没有这个问题".to_string(),
                max_lines: 1,
            },
            NativePanelVisualCardBodyLineInput {
                role: NativePanelVisualCardBodyRole::Assistant,
                prefix: Some("$".to_string()),
                text: "已检查并修了一处抽共享代码后引入的布局抖动点".to_string(),
                max_lines: 2,
            },
        ],
        action_hint: None,
        rows: Vec::new(),
        height: 140.0,
        collapsed_height: 64.0,
        compact: false,
        removing: false,
    }];

    let plan = resolve_native_panel_visual_plan(&input);

    assert!(plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. } if text == "EchoIsland"
    )));
    assert!(!plan.primitives.iter().any(|primitive| matches!(
        primitive,
        NativePanelVisualPrimitive::Text { text, .. }
            if text.contains("第二次移出鼠标") || text.contains("已检查并修了")
    )));
}

#[test]
fn expanded_visual_plan_reveals_card_content_before_shell_is_mostly_open() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.22;
    input.card_stack_frame.height = input.card_stack_content_height;
    input.cards = vec![NativePanelVisualCardInput {
            style: crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Completion,
            title: "Done".to_string(),
            subtitle: Some("#abcdef · now".to_string()),
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
        }];

    let plan = resolve_native_panel_visual_plan(&input);

    let title = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                text,
                origin,
                color,
                ..
            } if text == "Done" => Some((*origin, *color)),
            _ => None,
        })
        .expect("title should start revealing before the shell is mostly open");

    let stable_title_y = input.shell_frame.y
        + input.card_stack_frame.y
        + (input.card_stack_content_height - input.cards[0].height)
        + input.cards[0].height
        - 24.0;
    assert!(title.0.y < stable_title_y);
    assert_ne!(
        title.1,
        crate::native_panel_renderer::visual_primitives::NativePanelVisualColor::rgb(245, 247, 252,)
    );
}

#[test]
fn expanded_visual_plan_fades_removing_status_card_content_before_shell_exit() {
    let mut input = visual_input(NativePanelVisualDisplayMode::Expanded);
    input.separator_visibility = 0.22;
    input.card_stack_frame.height = input.card_stack_content_height;
    input.cards = vec![NativePanelVisualCardInput {
            style: crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Completion,
            title: "Done".to_string(),
            subtitle: Some("#abcdef · now".to_string()),
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
            removing: true,
        }];

    let plan = resolve_native_panel_visual_plan(&input);

    assert!(!plan.primitives.iter().any(|primitive| {
        matches!(primitive, NativePanelVisualPrimitive::Text { text, .. } if text == "Done")
    }));
    assert!(plan.primitives.iter().any(|primitive| {
            matches!(primitive, NativePanelVisualPrimitive::RoundRect { frame, .. } if frame.height > 40.0)
        }));
}
