use super::super::{
    NativePanelVisualActionButtonInput, NativePanelVisualCardBadgeInput,
    NativePanelVisualCardBodyLineInput, NativePanelVisualCardBodyRole, NativePanelVisualCardInput,
    NativePanelVisualCardRowInput, NativePanelVisualCardStyle, NativePanelVisualDisplayMode,
    NativePanelVisualPlan, NativePanelVisualPlanInput, compact_digit_y,
    extend_visible_content_primitives, native_panel_visual_card_input_from_scene_card_with_height,
    resolve_native_panel_visual_plan,
};
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

pub(super) const SETTINGS_ACTION_ICON_TEXT: &str = "\u{E713}";
pub(super) const QUIT_ACTION_ICON_TEXT: &str = "\u{E7E8}";
pub(super) const SETTINGS_ACTION_ICON_SIZE: i32 = 16;
pub(super) const QUIT_ACTION_ICON_SIZE: i32 = 16;

pub(super) fn visual_input(
    display_mode: NativePanelVisualDisplayMode,
) -> NativePanelVisualPlanInput {
    let compact_bar_width = if display_mode == NativePanelVisualDisplayMode::Expanded {
        crate::native_panel_core::DEFAULT_EXPANDED_PILL_WIDTH
    } else {
        240.0
    };
    NativePanelVisualPlanInput {
            window_state: NativePanelHostWindowState {
                frame: Some(PanelRect {
                    x: 100.0,
                    y: 20.0,
                    width: 320.0,
                    height: 160.0,
                }),
                visible: display_mode != NativePanelVisualDisplayMode::Hidden,
                preferred_display_index: 0,
            },
            display_mode,
            surface: ExpandedSurface::Default,
            panel_frame: PanelRect {
                x: 100.0,
                y: 20.0,
                width: 320.0,
                height: 160.0,
            },
            compact_bar_frame: PanelRect {
                x: (320.0 - compact_bar_width) / 2.0,
                y: 12.0,
                width: compact_bar_width,
                height: 36.0,
            },
            left_shoulder_frame: PanelRect {
                x: 34.0,
                y: 42.0,
                width: 6.0,
                height: 6.0,
            },
            right_shoulder_frame: PanelRect {
                x: 280.0,
                y: 42.0,
                width: 6.0,
                height: 6.0,
            },
            shoulder_progress: 0.0,
            content_frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 160.0,
            },
            card_stack_frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 160.0,
            },
            card_stack_content_height: 180.0,
            shell_frame: PanelRect {
                x: 20.0,
                y: 0.0,
                width: 280.0,
                height: 150.0,
            },
            headline_text: "Codex ready".to_string(),
            headline_emphasized: false,
            active_count: "1".to_string(),
            active_count_elapsed_ms: 0,
            total_count: "3".to_string(),
            separator_visibility: 0.88,
            chrome_transition_progress: if display_mode == NativePanelVisualDisplayMode::Expanded {
                1.0
            } else {
                0.0
            },
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
                },
            ],
            glow_visible: true,
            glow_opacity: 0.78,
            action_buttons_visible: true,
            action_buttons: vec![
                NativePanelVisualActionButtonInput {
                    action: NativePanelEdgeAction::Settings,
                    frame: PanelRect {
                        x: 250.0,
                        y: 20.0,
                        width: 18.0,
                        height: 18.0,
                    },
                    debug_mode_enabled: false,
                },
                NativePanelVisualActionButtonInput {
                    action: NativePanelEdgeAction::Quit,
                    frame: PanelRect {
                        x: 280.0,
                        y: 20.0,
                        width: 18.0,
                        height: 18.0,
                    },
                    debug_mode_enabled: false,
                },
            ],
            completion_count: 2,
            mascot_elapsed_ms: 0,
            mascot_motion_frame: None,
            mascot_pose: SceneMascotPose::Complete,
            mascot_debug_mode_enabled: false,
        }
}

pub(super) fn mascot_green_bubble_frame(plan: &NativePanelVisualPlan) -> Option<PanelRect> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::MascotRoundRect { role, frame, .. }
                if *role == NativePanelVisualMascotRoundRectRole::MessageBubble =>
            {
                Some(*frame)
            }
            _ => None,
        })
}

pub(super) fn text_role_count(
    plan: &NativePanelVisualPlan,
    role: NativePanelVisualTextRole,
) -> usize {
    plan.primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                NativePanelVisualPrimitive::Text { role: primitive_role, .. }
                    if *primitive_role == role
            )
        })
        .count()
}

pub(super) fn mascot_sleep_label_origin(plan: &NativePanelVisualPlan) -> Option<PanelPoint> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::MascotText { role, origin, .. }
                if *role == NativePanelVisualMascotTextRole::SleepLabel =>
            {
                Some(*origin)
            }
            _ => None,
        })
}

pub(super) fn session_with_chat_lines() -> SessionSnapshotView {
    SessionSnapshotView {
        session_id: "session-123456".to_string(),
        source: "codex".to_string(),
        project_name: Some("EchoIsland".to_string()),
        cwd: None,
        model: None,
        terminal_app: None,
        terminal_bundle: None,
        host_app: None,
        window_title: None,
        tty: None,
        terminal_pid: None,
        cli_pid: None,
        iterm_session_id: None,
        kitty_window_id: None,
        tmux_env: None,
        tmux_pane: None,
        tmux_client_tty: None,
        status: "running".to_string(),
        current_tool: Some("Bash".to_string()),
        tool_description: Some("cargo test".to_string()),
        last_user_prompt: Some("Fix Windows card".to_string()),
        last_assistant_message: Some("Adjusting layout".to_string()),
        tool_history_count: 0,
        tool_history: Vec::new(),
        last_activity: Utc::now(),
    }
}

pub(super) fn pending_permission() -> PendingPermissionView {
    PendingPermissionView {
        request_id: "approval-1".to_string(),
        session_id: "session-approval".to_string(),
        source: "claude".to_string(),
        tool_name: Some("Bash".to_string()),
        tool_description: Some("Run command".to_string()),
        requested_at: Utc::now(),
    }
}

pub(super) fn pending_question() -> PendingQuestionView {
    PendingQuestionView {
        request_id: "question-1".to_string(),
        session_id: "session-question".to_string(),
        source: "claude".to_string(),
        header: Some("Pick one".to_string()),
        text: "Choose the deployment target".to_string(),
        options: vec!["Local".to_string(), "Production".to_string()],
        requested_at: Utc::now(),
    }
}

pub(super) fn headline_text_frame(
    plan: &crate::native_panel_renderer::visual_primitives::NativePanelVisualPlan,
) -> (f64, f64, f64) {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            NativePanelVisualPrimitive::Text {
                origin,
                max_width,
                text,
                size,
                ..
            } if text == "Codex ready" && *size == 13 => {
                Some((origin.x, *max_width, origin.x + *max_width / 2.0))
            }
            _ => None,
        })
        .expect("headline text")
}

pub(super) fn centered_text_visual_bounds(
    origin_x: f64,
    max_width: f64,
    text: &str,
    size: i32,
) -> (f64, f64) {
    let estimated_width =
        crate::native_panel_core::resolve_estimated_text_width(text, size as f64).min(max_width);
    let left = origin_x + (max_width - estimated_width) / 2.0;
    (left, left + estimated_width)
}

pub(super) fn text_primitive<'a>(
    plan: &'a crate::native_panel_renderer::visual_primitives::NativePanelVisualPlan,
    expected: &str,
) -> &'a NativePanelVisualPrimitive {
    plan.primitives
            .iter()
            .find(|primitive| {
                matches!(primitive, NativePanelVisualPrimitive::Text { text, .. } if text == expected)
            })
            .unwrap_or_else(|| panic!("missing text primitive {expected}"))
}

pub(super) fn use_wide_action_button_hit_regions(input: &mut NativePanelVisualPlanInput) {
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
}
