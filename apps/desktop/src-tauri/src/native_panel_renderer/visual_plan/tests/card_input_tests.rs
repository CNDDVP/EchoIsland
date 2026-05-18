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
fn visual_card_input_preserves_structured_body_roles_from_shared_spec() {
    let session = session_with_chat_lines();
    let card = native_panel_visual_card_input_from_scene_card_with_height(
        &SceneCard::Session {
            session,
            title: "EchoIsland".to_string(),
            status: SceneBadge {
                text: "Running".to_string(),
                emphasized: true,
            },
            snippet: Some("Adjusting layout".to_string()),
        },
        112.0,
    );

    assert_eq!(
        card.body_lines
            .iter()
            .map(|line| (
                line.role,
                line.prefix.as_deref(),
                line.text.as_str(),
                line.max_lines
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                NativePanelVisualCardBodyRole::Tool,
                Some("!"),
                "Bash cargo test",
                1,
            ),
            (
                NativePanelVisualCardBodyRole::Assistant,
                Some("$"),
                "Adjusting layout",
                2,
            ),
            (
                NativePanelVisualCardBodyRole::User,
                Some(">"),
                "Fix Windows card",
                1,
            ),
        ]
    );
    assert!(card.body.is_none());
    assert!(card.body_prefix.is_none());
}

#[test]
fn visual_card_input_preserves_pending_tones_from_shared_spec() {
    let approval = native_panel_visual_card_input_from_scene_card_with_height(
        &SceneCard::PendingPermission {
            pending: pending_permission(),
            count: 1,
        },
        72.0,
    );
    let question = native_panel_visual_card_input_from_scene_card_with_height(
        &SceneCard::PendingQuestion {
            pending: pending_question(),
            count: 1,
        },
        72.0,
    );
    let prompt = native_panel_visual_card_input_from_scene_card_with_height(
        &SceneCard::PromptAssist {
            session: session_with_chat_lines(),
        },
        72.0,
    );

    assert_eq!(approval.style, NativePanelVisualCardStyle::PendingApproval);
    assert_eq!(question.style, NativePanelVisualCardStyle::PendingQuestion);
    assert_eq!(prompt.style, NativePanelVisualCardStyle::PromptAssist);
}
