use chrono::Utc;
use echoisland_runtime::SessionSnapshotView;

use super::*;
use crate::native_panel_scene::{SceneBadge, SceneCard};

fn session() -> SessionSnapshotView {
    SessionSnapshotView {
        session_id: "session-123456".to_string(),
        source: "claude".to_string(),
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
        last_user_prompt: Some("修复测试".to_string()),
        last_assistant_message: Some("正在运行测试".to_string()),
        tool_history_count: 0,
        tool_history: Vec::new(),
        last_activity: Utc::now(),
    }
}

#[test]
fn session_card_spec_preserves_mac_header_badges_and_chat_lines() {
    let session = session();
    let spec = card_visual_spec_from_scene_card_with_height(
        &SceneCard::Session {
            session: session.clone(),
            title: "EchoIsland".to_string(),
            status: SceneBadge {
                text: "Running".to_string(),
                emphasized: true,
            },
            snippet: session.last_assistant_message.clone(),
        },
        96.0,
    );

    assert_eq!(spec.title, "EchoIsland");
    assert!(spec.subtitle.as_deref().unwrap_or_default().contains("#"));
    assert_eq!(
        spec.badges,
        vec![
            CardVisualBadgeSpec {
                role: CardVisualBadgeRole::Status,
                text: "Running".to_string(),
                emphasized: true,
            },
            CardVisualBadgeSpec {
                role: CardVisualBadgeRole::Source,
                text: "Claude".to_string(),
                emphasized: false,
            },
        ]
    );
    assert_eq!(
        spec.body
            .iter()
            .map(|line| (line.role, line.prefix.as_deref(), line.max_lines))
            .collect::<Vec<_>>(),
        vec![
            (CardVisualBodyRole::Tool, Some("!"), 1),
            (CardVisualBodyRole::Assistant, Some("$"), 2),
            (CardVisualBodyRole::User, Some(">"), 1),
        ]
    );
    assert_eq!(spec.animation.collapsed_height, 64.0);
}

#[test]
fn card_spec_exposes_mac_card_animation_constants() {
    let spec = card_visual_spec_from_scene_card_with_height(&SceneCard::Empty, 52.0);

    assert_eq!(spec.animation.reveal_scale_x_from, 0.96);
    assert_eq!(spec.animation.reveal_scale_y_from, 0.82);
    assert_eq!(
        spec.animation.reveal_translate_y_from,
        crate::native_panel_core::PANEL_CARD_REVEAL_Y
    );
    assert_eq!(
        spec.animation.content_reveal_delay_progress,
        crate::native_panel_core::PANEL_CARD_CONTENT_REVEAL_DELAY_PROGRESS
    );
    assert_eq!(
        spec.animation.content_early_exit_progress,
        crate::native_panel_core::PANEL_CARD_CONTENT_EARLY_EXIT_PROGRESS
    );
}

#[test]
fn card_spec_exposes_shared_shell_palette() {
    let spec = card_visual_spec_from_scene_card_with_height(&SceneCard::Empty, 52.0);

    assert_eq!(
        spec.shell.border_color,
        card_visual_shell_border_color(CardVisualStyle::Empty)
    );
    assert_eq!(
        spec.shell.fill_color,
        card_visual_shell_fill_color(CardVisualStyle::Empty)
    );
    assert_eq!(
        card_visual_shell_border_color(CardVisualStyle::Completion),
        CardVisualColorSpec::rgb(46, 79, 61)
    );
    assert_eq!(
        card_visual_shell_fill_color(CardVisualStyle::Completion),
        CardVisualColorSpec::rgb(37, 37, 41)
    );
}

#[test]
fn card_spec_exposes_shared_header_badge_palette_and_metrics() {
    let completion_status = card_visual_badge_paint_spec(
        CardVisualStyle::Completion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Status,
            text: "Done".to_string(),
            emphasized: true,
        },
    );
    let source = card_visual_badge_paint_spec(
        CardVisualStyle::Completion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Source,
            text: "Claude".to_string(),
            emphasized: false,
        },
    );
    let pending = card_visual_badge_paint_spec(
        CardVisualStyle::PendingApproval,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Status,
            text: "Approval".to_string(),
            emphasized: true,
        },
    );
    let question = card_visual_badge_paint_spec(
        CardVisualStyle::PendingQuestion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Status,
            text: "Question".to_string(),
            emphasized: true,
        },
    );

    assert_eq!(
        completion_status.background_color,
        CardVisualColorSpec::rgb(58, 84, 65)
    );
    assert_eq!(
        completion_status.foreground_color,
        CardVisualColorSpec::rgb(102, 222, 145)
    );
    assert_eq!(
        source.background_color,
        CardVisualColorSpec::rgb(84, 63, 42)
    );
    assert_eq!(
        source.foreground_color,
        CardVisualColorSpec::rgb(255, 199, 122)
    );
    assert_eq!(
        pending.background_color,
        CardVisualColorSpec::rgb(70, 53, 36)
    );
    assert_eq!(
        pending.foreground_color,
        CardVisualColorSpec::rgb(255, 184, 77)
    );
    assert_eq!(
        question.background_color,
        CardVisualColorSpec::rgb(61, 52, 83)
    );
    assert_eq!(
        question.foreground_color,
        CardVisualColorSpec::rgb(201, 176, 255)
    );
    assert_eq!(completion_status.height, 22.0);
    assert_eq!(completion_status.radius, 11.0);
    assert_eq!(completion_status.text_offset_y, 2.0);
    assert_eq!(completion_status.width, 64.0);
    assert_eq!(source.width, 64.0);
    assert_eq!(pending.width, 64.0);
    assert_eq!(question.width, 64.0);
}

#[test]
fn card_spec_uses_source_specific_header_badge_colors() {
    let claude = card_visual_badge_paint_spec(
        CardVisualStyle::Completion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Source,
            text: "Claude".to_string(),
            emphasized: false,
        },
    );
    let codex = card_visual_badge_paint_spec(
        CardVisualStyle::Completion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Source,
            text: "Codex".to_string(),
            emphasized: false,
        },
    );
    let feishu = card_visual_badge_paint_spec(
        CardVisualStyle::Completion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Source,
            text: "Feishu".to_string(),
            emphasized: false,
        },
    );
    let gemini = card_visual_badge_paint_spec(
        CardVisualStyle::Completion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Source,
            text: "Gemini".to_string(),
            emphasized: false,
        },
    );
    let unknown = card_visual_badge_paint_spec(
        CardVisualStyle::Completion,
        &CardVisualBadgeSpec {
            role: CardVisualBadgeRole::Source,
            text: "Other".to_string(),
            emphasized: false,
        },
    );

    assert_eq!(
        claude.background_color,
        CardVisualColorSpec::rgb(84, 63, 42)
    );
    assert_eq!(
        claude.foreground_color,
        CardVisualColorSpec::rgb(255, 199, 122)
    );
    assert_eq!(
        codex.background_color,
        CardVisualColorSpec::rgb(78, 91, 104)
    );
    assert_eq!(
        codex.foreground_color,
        CardVisualColorSpec::rgb(218, 234, 246)
    );
    assert_eq!(
        gemini.background_color,
        CardVisualColorSpec::rgb(42, 68, 52)
    );
    assert_eq!(
        gemini.foreground_color,
        CardVisualColorSpec::rgb(118, 224, 142)
    );
    assert_eq!(
        feishu.background_color,
        CardVisualColorSpec::rgb(38, 55, 78)
    );
    assert_eq!(
        feishu.foreground_color,
        CardVisualColorSpec::rgb(126, 178, 255)
    );
    assert_eq!(
        unknown.background_color,
        CardVisualColorSpec::rgb(76, 45, 67)
    );
    assert_eq!(
        unknown.foreground_color,
        CardVisualColorSpec::rgb(255, 139, 214)
    );
}

#[test]
fn card_spec_exposes_shared_settings_row_surface_and_value_badge_metrics() {
    let active = card_visual_settings_row_paint_spec(&CardVisualRowSpec {
        title: "Mascot".to_string(),
        value: "On".to_string(),
        active: true,
    });
    let inactive = card_visual_settings_row_paint_spec(&CardVisualRowSpec {
        title: "Sound".to_string(),
        value: "Off".to_string(),
        active: false,
    });

    assert_eq!(active.border_radius, 8.0);
    assert_eq!(active.fill_radius, 7.0);
    assert_eq!(active.border_color, CardVisualColorSpec::rgb(50, 84, 61));
    assert_eq!(active.fill_color, CardVisualColorSpec::rgb(42, 50, 44));
    assert_eq!(
        active.value_badge.background_color,
        CardVisualColorSpec::rgb(46, 68, 54)
    );
    assert_eq!(
        active.value_badge.foreground_color,
        CardVisualColorSpec::rgb(104, 222, 145)
    );
    assert_eq!(inactive.border_color, CardVisualColorSpec::rgb(50, 50, 56));
    assert_eq!(inactive.fill_color, CardVisualColorSpec::rgb(43, 43, 48));
    assert_eq!(
        inactive.value_badge.background_color,
        CardVisualColorSpec::rgb(54, 54, 58)
    );
    assert_eq!(
        inactive.value_badge.foreground_color,
        CardVisualColorSpec::rgb(230, 235, 245)
    );
    assert_eq!(active.title_size, 11);
    assert_eq!(active.value_badge.text_size, 10);
    assert_eq!(active.value_badge.width, 44.0);
    assert_eq!(active.value_badge.text_offset_y, 2.0);

    let width_preset = card_visual_settings_row_paint_spec(&CardVisualRowSpec {
        title: "Island Width".to_string(),
        value: "M".to_string(),
        active: true,
    });
    assert_eq!(width_preset.value_badge.width, active.value_badge.width);

    let display_preset = card_visual_settings_row_paint_spec(&CardVisualRowSpec {
        title: "Island Display".to_string(),
        value: "2/3".to_string(),
        active: true,
    });
    let update_preset = card_visual_settings_row_paint_spec(&CardVisualRowSpec {
        title: "Update & Upgrade".to_string(),
        value: "Go".to_string(),
        active: false,
    });
    assert_eq!(display_preset.value_badge.width, active.value_badge.width);
    assert_eq!(update_preset.value_badge.width, active.value_badge.width);
}

#[test]
fn card_spec_exposes_shared_body_line_palette_and_metrics() {
    let assistant = card_visual_body_line_paint_spec(
        CardVisualStyle::Default,
        CardVisualBodyRole::Assistant,
        Some("$"),
    );
    let user = card_visual_body_line_paint_spec(
        CardVisualStyle::Default,
        CardVisualBodyRole::User,
        Some(">"),
    );
    let completion = card_visual_body_line_paint_spec(
        CardVisualStyle::Completion,
        CardVisualBodyRole::Assistant,
        Some("$"),
    );
    let pending_question = card_visual_body_line_paint_spec(
        CardVisualStyle::PendingQuestion,
        CardVisualBodyRole::Plain,
        Some("?"),
    );

    assert_eq!(
        assistant.prefix_color,
        CardVisualColorSpec::rgb(217, 120, 87)
    );
    assert_eq!(user.prefix_color, CardVisualColorSpec::rgb(104, 222, 145));
    assert_eq!(user.text_color, CardVisualColorSpec::rgb(218, 222, 229));
    assert_eq!(
        completion.prefix_color,
        CardVisualColorSpec::rgb(104, 222, 145)
    );
    assert_eq!(
        pending_question.prefix_color,
        CardVisualColorSpec::rgb(201, 176, 255)
    );
    assert_eq!(assistant.prefix_size, 10);
    assert_eq!(assistant.text_size, 10);
}

#[test]
fn card_spec_exposes_shared_tool_pill_palette_and_metrics() {
    let bash = card_visual_tool_pill_paint_spec("Bash cargo test").expect("bash tool");
    let edit = card_visual_tool_pill_paint_spec("Edit src/main.rs").expect("edit tool");
    let unknown = card_visual_tool_pill_paint_spec("Custom task").expect("custom tool");

    assert_eq!(bash.tool_name, "Bash");
    assert_eq!(bash.description.as_deref(), Some("cargo test"));
    assert_eq!(
        bash.tool_name_color,
        CardVisualColorSpec::rgb(125, 242, 163)
    );
    assert_eq!(
        edit.tool_name_color,
        CardVisualColorSpec::rgb(135, 171, 255)
    );
    assert_eq!(
        unknown.tool_name_color,
        CardVisualColorSpec::rgb(245, 247, 252)
    );
    assert_eq!(bash.border_color, CardVisualColorSpec::rgb(60, 60, 64));
    assert_eq!(bash.background_color, CardVisualColorSpec::rgb(47, 47, 52));
    assert_eq!(
        bash.description_color,
        CardVisualColorSpec::rgb(214, 218, 225)
    );
    assert_eq!(bash.height, 22.0);
    assert_eq!(bash.radius, 5.0);
    assert!(bash.width > edit.tool_name_width);
}

#[test]
fn card_spec_exposes_shared_action_hint_pill_palette_and_metrics() {
    let spec =
        card_visual_action_hint_paint_spec("Allow / Deny in terminal").expect("action hint spec");

    assert_eq!(spec.height, 18.0);
    assert_eq!(spec.radius, 9.0);
    assert_eq!(spec.text_inset_x, 9.0);
    assert_eq!(spec.text_offset_y, 4.0);
    assert_eq!(spec.text_size, 10);
    assert_eq!(spec.background_color, CardVisualColorSpec::rgb(49, 49, 53));
    assert_eq!(
        spec.foreground_color,
        CardVisualColorSpec::rgb(230, 235, 245)
    );
    assert!(spec.width > 32.0);
    assert!(card_visual_action_hint_paint_spec("   ").is_none());
}

#[test]
fn card_spec_exposes_shared_header_text_palette_and_metrics() {
    let regular = card_visual_header_text_paint_spec(CardVisualStyle::Default);
    let empty = card_visual_header_text_paint_spec(CardVisualStyle::Empty);

    assert_eq!(regular.title.color, CardVisualColorSpec::rgb(245, 247, 252));
    assert_eq!(regular.title.size, 12);
    assert_eq!(regular.title_max_chars, 30);
    assert_eq!(
        regular.subtitle.color,
        CardVisualColorSpec::rgb(171, 179, 194)
    );
    assert_eq!(regular.subtitle.size, 9);
    assert_eq!(empty.title.color, CardVisualColorSpec::rgb(171, 179, 194));
    assert_eq!(empty.title.size, 12);
}

#[test]
fn card_spec_exposes_shared_content_reveal_animation_frame() {
    let hidden = card_visual_content_reveal_frame(0.0);
    let open = card_visual_content_reveal_frame(1.0);

    assert_eq!(hidden.visibility_progress, 0.0);
    assert_eq!(hidden.translate_y, -5.0);
    assert_eq!(open.visibility_progress, 1.0);
    assert_eq!(open.translate_y, 0.0);
    assert!(card_visual_content_reveal_frame(0.5).visibility_progress > 0.0);
}

#[test]
fn card_spec_exposes_shared_content_exit_animation_frame() {
    let opening_mid = card_visual_content_transition_frame(0.5, false);
    let closing_mid = card_visual_content_transition_frame(0.5, true);
    let closing_hidden = card_visual_content_transition_frame(
        crate::native_panel_core::PANEL_CARD_CONTENT_EARLY_EXIT_PROGRESS - 0.01,
        true,
    );

    assert!(closing_mid.visibility_progress < opening_mid.visibility_progress);
    assert_eq!(closing_hidden.visibility_progress, 0.0);
    assert_eq!(
        card_visual_content_transition_frame(1.0, true).visibility_progress,
        1.0
    );
}

#[test]
fn card_spec_exposes_shared_stack_reveal_animation_frame() {
    let first = card_visual_stack_reveal_frame(0.88, 2, 0);
    let second = card_visual_stack_reveal_frame(0.88, 2, 1);
    let hidden = card_visual_stack_reveal_frame(0.0, 2, 0);

    assert_eq!(first.progress, 1.0);
    assert_eq!(first.card_phase, 1.0);
    assert_eq!(second.card_phase, 1.0);
    assert_eq!(hidden.progress, 0.0);
    assert_eq!(hidden.card_phase, 0.0);
    assert!(card_visual_stack_reveal_frame(0.44, 2, 0).card_phase > 0.0);
}

#[test]
fn card_spec_exposes_shared_stack_stagger_phase_for_enter_and_exit() {
    let first_entering = card_visual_staggered_phase(0.5, 0, 3, true);
    let second_entering = card_visual_staggered_phase(0.5, 1, 3, true);
    let first_exiting = card_visual_staggered_phase(0.5, 0, 3, false);
    let last_exiting = card_visual_staggered_phase(0.5, 2, 3, false);

    assert!(first_entering > second_entering);
    assert!(last_exiting > first_exiting);
    assert_eq!(card_visual_staggered_phase(0.0, 0, 3, true), 0.0);
    assert_eq!(card_visual_staggered_phase(1.0, 2, 3, false), 1.0);
}

#[test]
fn card_spec_exposes_shared_content_visibility_phase() {
    assert_eq!(card_visual_content_visibility_phase(0.10, true), 0.0);
    assert_eq!(card_visual_content_visibility_phase(0.18, true), 0.0);
    assert!(card_visual_content_visibility_phase(0.24, true) > 0.0);
    assert_eq!(card_visual_content_visibility_phase(0.0, false), 1.0);
    assert!(card_visual_content_visibility_phase(0.30, false) < 1.0);
    assert_eq!(card_visual_content_visibility_phase(1.0, false), 0.0);
}

#[test]
fn card_spec_exposes_shared_shell_reveal_frame() {
    let expanded = PanelRect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
    };

    let collapsed = card_visual_shell_reveal_frame(expanded, 52.0, 0.0);
    let open = card_visual_shell_reveal_frame(expanded, 52.0, 1.0);

    assert_eq!(collapsed.width, 192.0);
    assert_eq!(collapsed.height, 52.0);
    assert_eq!(collapsed.x, 14.0);
    assert_eq!(collapsed.y, 68.0);
    assert_eq!(open, expanded);
}

#[test]
fn card_spec_exposes_shared_content_layout_metrics() {
    let frame = PanelRect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
    };
    let layout = card_visual_content_layout(frame);

    assert_eq!(layout.content_x, 20.0);
    assert_eq!(layout.content_width, 180.0);
    assert_eq!(layout.title_y, 96.0);
    assert_eq!(layout.subtitle_y, 80.0);
    assert_eq!(layout.empty_title_y, 60.0);
}

#[test]
fn card_spec_exposes_shared_body_layout_metrics() {
    let frame = PanelRect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
    };
    let default = card_visual_body_layout(frame, false);
    let with_hint = card_visual_body_layout(frame, true);

    assert_eq!(default.prefix_x, 20.0);
    assert_eq!(default.text_x, 35.0);
    assert_eq!(default.body_width, 165.0);
    assert_eq!(default.initial_y, 28.0);
    assert_eq!(with_hint.initial_y, 53.0);
}

#[test]
fn card_spec_exposes_shared_action_hint_layout() {
    let frame = PanelRect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
    };

    let layout = card_visual_action_hint_layout(frame, "Go").expect("action hint layout");

    assert_eq!(
        layout.pill_frame,
        PanelRect {
            x: 20.0,
            y: 29.0,
            width: 32.0,
            height: 18.0,
        }
    );
    assert_eq!(layout.text_origin.x, 29.0);
    assert_eq!(layout.text_origin.y, 33.0);
    assert_eq!(layout.text_max_width, 14.0);
    assert!(card_visual_action_hint_layout(frame, "   ").is_none());
}

#[test]
fn card_spec_exposes_shared_tool_pill_layout() {
    let frame = PanelRect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
    };

    let layout =
        card_visual_tool_pill_layout(frame, 42.0, "Bash cargo test").expect("tool pill layout");

    assert_eq!(layout.pill_frame.x, 20.0);
    assert_eq!(layout.pill_frame.y, 42.0);
    assert_eq!(layout.pill_frame.height, 22.0);
    assert_eq!(layout.tool_name_origin.x, 27.0);
    assert_eq!(layout.tool_name_origin.y, 47.0);
    assert!(layout.tool_name_max_width > 0.0);
    let description = layout.description.expect("description layout");
    assert!(description.origin.x > layout.tool_name_origin.x);
    assert_eq!(description.origin.y, 47.0);
    assert!(description.max_width > 0.0);
}

#[test]
fn card_spec_exposes_shared_header_badge_layout() {
    let badge = CardVisualBadgeSpec {
        role: CardVisualBadgeRole::Status,
        text: "Done".to_string(),
        emphasized: true,
    };

    let layout = card_visual_badge_layout(CardVisualStyle::Completion, &badge, 210.0, 96.0);

    assert_eq!(layout.badge_frame.y, 93.0);
    assert_eq!(layout.badge_frame.height, 22.0);
    assert_eq!(layout.badge_frame.x + layout.badge_frame.width, 210.0);
    assert_eq!(layout.text_origin.x, layout.badge_frame.x + 7.0);
    assert_eq!(layout.text_origin.y, 98.0);
    assert_eq!(
        layout.paint.foreground_color,
        CardVisualColorSpec::rgb(102, 222, 145)
    );
}

#[test]
fn card_spec_exposes_shared_single_line_text_box_frame() {
    let settings_value = card_visual_single_line_text_box_frame(44.0, 18.0, 9.0, 2.0, 10.0).frame;
    let status_badge = card_visual_single_line_text_box_frame(64.0, 22.0, 7.0, 2.0, 10.0).frame;

    assert_eq!(
        settings_value,
        PanelRect {
            x: 9.0,
            y: 3.0,
            width: 26.0,
            height: 13.0,
        }
    );
    assert_eq!(
        status_badge,
        PanelRect {
            x: 7.0,
            y: 5.0,
            width: 50.0,
            height: 13.0,
        }
    );
}

#[test]
fn card_spec_exposes_shared_settings_row_layout() {
    let frame = PanelRect {
        x: 10.0,
        y: 20.0,
        width: 200.0,
        height: 100.0,
    };
    let row = CardVisualRowSpec {
        title: "Mascot".to_string(),
        value: "On".to_string(),
        active: true,
    };

    let layout = card_visual_settings_row_layout(frame, 0, &row).expect("settings row layout");

    assert_eq!(
        layout.row_frame,
        PanelRect {
            x: 24.0,
            y: 44.0,
            width: 172.0,
            height: 30.0,
        }
    );
    assert_eq!(
        layout.row_inner_frame,
        PanelRect {
            x: 25.0,
            y: 45.0,
            width: 170.0,
            height: 28.0,
        }
    );
    assert_eq!(layout.value_badge_frame.x, 142.0);
    assert_eq!(layout.value_badge_frame.y, 50.0);
    assert_eq!(layout.title_origin.x, 36.0);
    assert_eq!(layout.title_origin.y, 51.0);
    assert_eq!(layout.title_max_width, 95.0);
    assert_eq!(layout.value_origin.x, 151.0);
    assert_eq!(layout.value_origin.y, 53.0);
}
