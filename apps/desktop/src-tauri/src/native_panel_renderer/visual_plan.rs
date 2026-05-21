mod card_input;
mod input;
mod mascot;

use crate::{
    native_panel_core::{
        ACTIVE_COUNT_SCROLL_TRAVEL, ACTIVE_COUNT_TEXT_OFFSET_X, ACTIVE_COUNT_TEXT_WIDTH,
        ActiveCountMarqueeInput, CARD_RADIUS, COMPACT_PILL_RADIUS, CompactBarContentLayout,
        CompactBarContentLayoutInput, EXPANDED_CARD_GAP, EXPANDED_CARD_OVERHANG, PanelPoint,
        PanelRect, compact_title, default_panel_card_metric_constants, ease_out_cubic, lerp,
        resolve_active_count_marquee_frame, resolve_compact_action_button_layout,
        resolve_compact_bar_content_layout, resolve_estimated_chat_body_height,
        resolve_estimated_text_width, resolve_next_stacked_card_frame,
        resolve_panel_chrome_visibility_spec,
    },
    native_panel_scene::SceneMascotPose,
};

use super::action_button_visual_spec::{
    ActionButtonVisibilitySpec, ActionButtonVisualSpec, ActionButtonVisualSpecInput,
    action_button_visual_frame_for_phase, resolve_action_button_visual_specs,
};
use super::card_visual_spec::{
    CardVisualBadgeRole, CardVisualBadgeSpec, CardVisualRowSpec, card_visual_action_hint_layout,
    card_visual_badge_layout, card_visual_body_layout, card_visual_body_line_paint_spec,
    card_visual_content_layout, card_visual_content_transition_frame,
    card_visual_header_text_paint_spec, card_visual_settings_row_layout,
    card_visual_shell_border_color, card_visual_shell_fill_color, card_visual_shell_reveal_frame,
    card_visual_stack_reveal_frame, card_visual_tool_pill_layout,
};
use super::completion_glow_visual_spec::{
    CompletionGlowVisualSpecInput, resolve_completion_glow_visual_spec,
};
use super::descriptors::NativePanelEdgeAction;
use super::mascot_visual_spec::{MascotVisualSpecInput, resolve_mascot_visual_spec};
use super::visual_primitives::{
    NativePanelVisualColor, NativePanelVisualPlan, NativePanelVisualPrimitive,
    NativePanelVisualShoulderSide, NativePanelVisualTextAlignment, NativePanelVisualTextRole,
    NativePanelVisualTextWeight, native_panel_visual_text_box_height,
    native_panel_visual_text_box_height_for_role,
};

use card_input::{
    card_visual_body_role_from_visual_role, card_visual_style_from_visual_style,
    visual_color_from_card_spec,
};
pub(crate) use card_input::{
    native_panel_visual_card_input_from_scene_card,
    native_panel_visual_card_input_from_scene_card_with_height,
};
pub(crate) use input::{
    NativePanelVisualActionButtonInput, NativePanelVisualCardBadgeInput,
    NativePanelVisualCardBodyLineInput, NativePanelVisualCardBodyRole, NativePanelVisualCardInput,
    NativePanelVisualCardRowInput, NativePanelVisualCardStyle, NativePanelVisualDisplayMode,
    NativePanelVisualPlanInput,
};
use mascot::{apply_mascot_chrome_alpha, push_mascot_primitives};

pub(crate) fn resolve_native_panel_visual_plan(
    input: &NativePanelVisualPlanInput,
) -> NativePanelVisualPlan {
    if input.display_mode == NativePanelVisualDisplayMode::Hidden || !input.window_state.visible {
        return NativePanelVisualPlan {
            hidden: true,
            primitives: Vec::new(),
        };
    }

    let mut primitives = Vec::new();
    let panel_frame = visual_panel_frame(input);
    let compact_frame = non_zero_rect(input.compact_bar_frame).unwrap_or(panel_frame);
    let shell_frame = non_zero_rect(input.shell_frame).unwrap_or(panel_frame);
    let content_frame = non_zero_rect(input.content_frame).unwrap_or(panel_frame);
    let expanded_display_mode = input.display_mode == NativePanelVisualDisplayMode::Expanded;
    let chrome_transition_progress = if expanded_display_mode {
        input.chrome_transition_progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let chrome_visibility = resolve_panel_chrome_visibility_spec(
        crate::native_panel_core::PanelChromeVisibilitySpecInput {
            edge_actions_visible: input.action_buttons_visible,
            expanded_display_mode,
            surface: input.surface,
            transition_visibility_progress: chrome_transition_progress,
        },
    );
    let action_button_visibility = chrome_visibility.action_buttons;

    if input.display_mode == NativePanelVisualDisplayMode::Compact {
        push_compact_island_background(&mut primitives, input, compact_frame);
    }
    if chrome_visibility.collapsed_mascot_visible {
        push_completion_glow_if_visible(&mut primitives, input, compact_frame);
    }

    if input.display_mode == NativePanelVisualDisplayMode::Expanded {
        primitives.push(NativePanelVisualPrimitive::RoundRect {
            frame: shell_frame,
            radius: crate::native_panel_core::EXPANDED_PANEL_RADIUS,
            color: NativePanelVisualColor::rgb(12, 12, 15),
        });

        if input.separator_visibility > 0.01 {
            primitives.push(NativePanelVisualPrimitive::Rect {
                frame: PanelRect {
                    x: shell_frame.x + 20.0,
                    y: compact_frame.y + compact_frame.height + 8.0,
                    width: (shell_frame.width - 40.0).max(0.0),
                    height: 1.0,
                },
                color: NativePanelVisualColor::rgb(62, 62, 70),
            });
        }

        push_expanded_card_shells(&mut primitives, input, shell_frame);
    }

    let compact_content = compact_content_layout(compact_frame, false);
    let collapsed_fade_progress = chrome_visibility.collapsed_exit_progress.clamp(0.0, 1.0);
    let collapsed_alpha = 1.0 - collapsed_fade_progress;
    let headline_alpha = if expanded_display_mode {
        1.0
    } else {
        collapsed_alpha
    };
    let headline_text = fit_text_to_width(
        &input.headline_text,
        compact_content.headline_width,
        13.0,
        1,
    );
    let headline_width =
        resolve_estimated_text_width(&headline_text, 13.0).min(compact_content.headline_width);
    let headline_center_x = compact_frame.x + compact_content.headline_center_x;
    primitives.push(NativePanelVisualPrimitive::Text {
        role: NativePanelVisualTextRole::CompactHeadline,
        origin: PanelPoint {
            x: headline_center_x - headline_width / 2.0,
            y: compact_frame.y + compact_headline_y(compact_frame.height),
        },
        max_width: headline_width,
        text: headline_text,
        color: if input.headline_emphasized {
            NativePanelVisualColor::rgb(255, 255, 255)
        } else {
            NativePanelVisualColor::rgb(230, 235, 245)
        },
        size: 13,
        weight: NativePanelVisualTextWeight::Semibold,
        alignment: NativePanelVisualTextAlignment::Center,
        alpha: headline_alpha,
    });

    if chrome_visibility.collapsed_metrics_visible
        && (!input.active_count.is_empty() || !input.total_count.is_empty())
    {
        let active_count_marquee = resolve_active_count_marquee_frame(ActiveCountMarqueeInput {
            text: &input.active_count,
            elapsed_ms: input.active_count_elapsed_ms,
        });
        let active_count_y = compact_frame.y + compact_digit_y(compact_frame.height);
        let active_count_x =
            compact_frame.x + compact_content.active_x + ACTIVE_COUNT_TEXT_OFFSET_X;
        let active_count_color = if input.active_count.parse::<usize>().unwrap_or_default() > 0 {
            NativePanelVisualColor::rgb(102, 222, 145)
        } else {
            NativePanelVisualColor::rgb(156, 166, 184)
        };
        primitives.push(NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CompactActiveCount,
            origin: PanelPoint {
                x: active_count_x,
                y: active_count_y - active_count_marquee.scroll_offset,
            },
            max_width: ACTIVE_COUNT_TEXT_WIDTH,
            text: active_count_marquee.current.clone(),
            color: active_count_color,
            size: 15,
            weight: NativePanelVisualTextWeight::Semibold,
            alignment: NativePanelVisualTextAlignment::Right,
            alpha: collapsed_alpha,
        });
        if active_count_marquee.show_next {
            primitives.push(NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::CompactActiveCountNext,
                origin: PanelPoint {
                    x: active_count_x,
                    y: active_count_y + ACTIVE_COUNT_SCROLL_TRAVEL
                        - active_count_marquee.scroll_offset,
                },
                max_width: ACTIVE_COUNT_TEXT_WIDTH,
                text: active_count_marquee.next.clone(),
                color: NativePanelVisualColor::rgb(102, 222, 145),
                size: 15,
                weight: NativePanelVisualTextWeight::Semibold,
                alignment: NativePanelVisualTextAlignment::Right,
                alpha: collapsed_alpha,
            });
        }
        if !input.total_count.is_empty() {
            primitives.push(NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::CompactSlash,
                origin: PanelPoint {
                    x: compact_frame.x + compact_content.slash_x,
                    y: compact_frame.y + compact_digit_y(compact_frame.height),
                },
                max_width: 10.0,
                text: "/".to_string(),
                color: NativePanelVisualColor::rgb(245, 247, 252),
                size: 15,
                weight: NativePanelVisualTextWeight::Semibold,
                alignment: NativePanelVisualTextAlignment::Center,
                alpha: collapsed_alpha,
            });
            primitives.push(NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::CompactTotalCount,
                origin: PanelPoint {
                    x: compact_frame.x + compact_content.total_x,
                    y: compact_frame.y + compact_digit_y(compact_frame.height),
                },
                max_width: 24.0,
                text: input.total_count.clone(),
                color: NativePanelVisualColor::rgb(245, 247, 252),
                size: 15,
                weight: NativePanelVisualTextWeight::Semibold,
                alignment: NativePanelVisualTextAlignment::Left,
                alpha: collapsed_alpha,
            });
        }
    }

    let _ = (content_frame, input.cards_visible);

    if chrome_visibility.collapsed_mascot_visible {
        let mascot_spec = resolve_mascot_visual_spec(MascotVisualSpecInput {
            body_center: PanelPoint {
                x: compact_frame.x + compact_content.mascot_center_x,
                y: compact_frame.y + compact_frame.height / 2.0,
            },
            completion_badge_anchor: PanelPoint {
                x: compact_frame.x + compact_content.mascot_center_x,
                y: compact_frame.y + compact_frame.height / 2.0,
            },
            radius: 11.0,
            pose: input.mascot_pose,
            debug_mode_enabled: input.mascot_debug_mode_enabled,
            completion_count: input.completion_count,
            elapsed_ms: input.mascot_elapsed_ms,
            motion_frame: input.mascot_motion_frame,
        });
        let mascot_start_index = primitives.len();
        push_mascot_primitives(&mut primitives, &mascot_spec);
        apply_mascot_chrome_alpha(&mut primitives[mascot_start_index..], collapsed_alpha);
    }

    if action_button_visibility.reserves_headline_space {
        let button_frames = input
            .action_buttons
            .iter()
            .map(|button| (button.action, button.frame))
            .collect::<Vec<_>>();
        for spec in resolve_action_button_visual_specs(ActionButtonVisualSpecInput {
            visible: true,
            compact_frame,
            buttons: &button_frames,
            debug_mode_enabled: input
                .action_buttons
                .iter()
                .any(|button| button.debug_mode_enabled),
        }) {
            push_action_button_icon(&mut primitives, &spec, action_button_visibility);
        }
    }

    NativePanelVisualPlan {
        hidden: false,
        primitives,
    }
}

fn push_completion_glow_if_visible(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    input: &NativePanelVisualPlanInput,
    compact_frame: PanelRect,
) {
    let Some(spec) = resolve_completion_glow_visual_spec(CompletionGlowVisualSpecInput {
        visible: input.glow_visible,
        frame: compact_frame,
        base_opacity: input.glow_opacity,
        elapsed_ms: input.mascot_elapsed_ms,
    }) else {
        return;
    };
    primitives.push(NativePanelVisualPrimitive::CompletionGlow {
        frame: spec.frame,
        opacity: spec.opacity,
    });
}

fn push_action_button_icon(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    spec: &ActionButtonVisualSpec,
    visibility: ActionButtonVisibilitySpec,
) {
    let frame = action_button_visual_frame_for_phase(
        spec.frame,
        visibility,
        action_button_outward_direction(spec.action),
    );
    let size = ((spec.size as f64) * visibility.scale).round().max(1.0) as i32;
    let text_height = native_panel_visual_text_box_height(&spec.text, size);
    primitives.push(NativePanelVisualPrimitive::Text {
        role: action_button_text_role(spec.action),
        origin: PanelPoint {
            x: frame.x,
            y: frame.y + ((frame.height - text_height) / 2.0).round() - 1.5,
        },
        max_width: frame.width,
        text: spec.text.clone(),
        color: spec.color,
        size,
        weight: spec.weight,
        alignment: NativePanelVisualTextAlignment::Center,
        alpha: visibility.opacity.clamp(0.0, 1.0),
    });
}

fn action_button_outward_direction(action: NativePanelEdgeAction) -> f64 {
    match action {
        NativePanelEdgeAction::Settings => -1.0,
        NativePanelEdgeAction::Quit => 1.0,
    }
}

fn action_button_text_role(action: NativePanelEdgeAction) -> NativePanelVisualTextRole {
    match action {
        NativePanelEdgeAction::Settings => NativePanelVisualTextRole::ActionButtonSettings,
        NativePanelEdgeAction::Quit => NativePanelVisualTextRole::ActionButtonQuit,
    }
}

fn push_text(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    x: f64,
    y: f64,
    max_width: f64,
    text: String,
    color: NativePanelVisualColor,
    size: i32,
) {
    push_text_with_style(
        primitives,
        x,
        y,
        max_width,
        text,
        color,
        size,
        NativePanelVisualTextWeight::Normal,
        NativePanelVisualTextAlignment::Left,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_text_with_style(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    x: f64,
    y: f64,
    max_width: f64,
    text: String,
    color: NativePanelVisualColor,
    size: i32,
    weight: NativePanelVisualTextWeight,
    alignment: NativePanelVisualTextAlignment,
) {
    primitives.push(NativePanelVisualPrimitive::Text {
        role: NativePanelVisualTextRole::Unspecified,
        origin: PanelPoint { x, y },
        max_width,
        text,
        color,
        size,
        weight,
        alignment,
        alpha: 1.0,
    });
}

fn rect_center(rect: PanelRect) -> PanelPoint {
    PanelPoint {
        x: rect.x + rect.width / 2.0,
        y: rect.y + rect.height / 2.0,
    }
}

fn centered_rect(center: PanelPoint, radius_x: f64, radius_y: f64) -> PanelRect {
    PanelRect {
        x: center.x - radius_x,
        y: center.y - radius_y,
        width: radius_x * 2.0,
        height: radius_y * 2.0,
    }
}

fn point_on_circle(center: PanelPoint, radius: f64, degrees: f64) -> PanelPoint {
    let radians = degrees.to_radians();
    PanelPoint {
        x: center.x + radius * radians.cos(),
        y: center.y + radius * radians.sin(),
    }
}

fn visual_panel_frame(input: &NativePanelVisualPlanInput) -> PanelRect {
    non_zero_rect(input.content_frame)
        .or_else(|| {
            input.window_state.frame.map(|frame| PanelRect {
                x: 0.0,
                y: 0.0,
                width: frame.width,
                height: frame.height,
            })
        })
        .unwrap_or(input.panel_frame)
}

fn compact_content_layout(
    compact_frame: PanelRect,
    actions_active: bool,
) -> CompactBarContentLayout {
    let mut layout = resolve_compact_bar_content_layout(CompactBarContentLayoutInput {
        bar_width: compact_frame.width,
        bar_height: compact_frame.height,
    });
    if actions_active {
        let action_layout = resolve_compact_action_button_layout(PanelRect {
            x: 0.0,
            y: 0.0,
            width: compact_frame.width,
            height: compact_frame.height,
        });
        let side_gap = 4.0;
        let safe_left = action_layout.settings.x + action_layout.settings.width + side_gap;
        let safe_right = action_layout.quit.x - side_gap;
        let center_x = compact_frame.width / 2.0;
        let centered_safe_width = ((center_x - safe_left).min(safe_right - center_x) * 2.0)
            .clamp(0.0, layout.headline_width);
        if centered_safe_width > 0.0 {
            layout.headline_width = centered_safe_width;
            layout.headline_x = (compact_frame.width / 2.0) - centered_safe_width / 2.0;
            layout.headline_center_x = compact_frame.width / 2.0;
        }
    }
    layout
}

fn compact_headline_y(bar_height: f64) -> f64 {
    ((bar_height - 24.0) / 2.0).round() - 1.5
}

fn compact_digit_y(bar_height: f64) -> f64 {
    ((bar_height - crate::native_panel_core::ACTIVE_COUNT_LABEL_HEIGHT) / 2.0).round() - 1.5
}

fn push_compact_island_background(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    input: &NativePanelVisualPlanInput,
    compact_frame: PanelRect,
) {
    push_compact_shoulder_primitive(
        primitives,
        input.left_shoulder_frame,
        NativePanelVisualShoulderSide::Left,
        input.shoulder_progress,
    );
    push_compact_shoulder_primitive(
        primitives,
        input.right_shoulder_frame,
        NativePanelVisualShoulderSide::Right,
        input.shoulder_progress,
    );
    primitives.push(NativePanelVisualPrimitive::RoundRect {
        frame: compact_frame,
        radius: COMPACT_PILL_RADIUS,
        color: NativePanelVisualColor::rgb(12, 12, 15),
    });
}

fn push_expanded_card_shells(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    input: &NativePanelVisualPlanInput,
    shell_frame: PanelRect,
) {
    if !input.cards_visible || input.cards.is_empty() || input.separator_visibility <= 0.01 {
        return;
    }

    let mut cursor_y = input.card_stack_content_height;
    let stack_overflow_y =
        (input.card_stack_content_height - input.card_stack_frame.height).max(0.0);
    let clip_bounds = PanelRect {
        x: shell_frame.x + input.card_stack_frame.x,
        y: shell_frame.y + input.card_stack_frame.y,
        width: input.card_stack_frame.width,
        height: input.card_stack_frame.height,
    };
    for (index, card) in input.cards.iter().enumerate() {
        let single_empty_card =
            input.cards.len() == 1 && card.style == NativePanelVisualCardStyle::Empty;
        let card_height = if single_empty_card {
            card.height
                .min(cursor_y.max(input.card_stack_frame.height))
                .max(card.collapsed_height)
        } else {
            card.height
        };
        let Some(frame) = resolve_next_stacked_card_frame(
            &mut cursor_y,
            index > 0,
            card_height,
            input.card_stack_frame.width,
            EXPANDED_CARD_GAP,
            EXPANDED_CARD_OVERHANG,
        ) else {
            break;
        };
        let stable_card_frame = PanelRect {
            x: shell_frame.x + input.card_stack_frame.x + frame.x,
            y: shell_frame.y + input.card_stack_frame.y + frame.y - stack_overflow_y,
            width: frame.width,
            height: frame.height,
        };
        let stable_visible_height =
            clip_rect_vertically(stable_card_frame, clip_bounds).map(|frame| frame.height);
        if stable_visible_height.is_none_or(|height| height <= 6.0) && !single_empty_card {
            continue;
        }
        let phase =
            card_visual_stack_reveal_frame(input.separator_visibility, input.cards.len(), index)
                .card_phase;
        if phase <= 0.001 {
            continue;
        }
        let expanded_frame = PanelRect {
            x: shell_frame.x + input.card_stack_frame.x + frame.x,
            y: shell_frame.y + input.card_stack_frame.y + frame.y - stack_overflow_y,
            width: frame.width,
            height: frame.height,
        };
        let unclipped_card_frame =
            card_visual_shell_reveal_frame(expanded_frame, card.collapsed_height, phase);
        let Some(card_frame) = clip_rect_vertically(unclipped_card_frame, clip_bounds) else {
            continue;
        };
        if card_frame.height <= 6.0 {
            continue;
        }

        let content_visible =
            single_empty_card || card_frame.height >= card.collapsed_height.min(48.0);
        push_card_shell(primitives, card, card_frame);
        if content_visible {
            let content_layout_frame = if single_empty_card {
                card_frame
            } else {
                unclipped_card_frame
            };
            push_expanded_card_content(primitives, card, content_layout_frame, card_frame, phase);
        }
    }
}

fn push_card_shell(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    card: &NativePanelVisualCardInput,
    frame: PanelRect,
) {
    let radius = CARD_RADIUS.min(frame.height / 2.0);
    primitives.push(NativePanelVisualPrimitive::RoundRect {
        frame,
        radius,
        color: card_shell_border_color(card.style),
    });

    let inner = inset_rect(frame, 1.0);
    if inner.width <= 0.0 || inner.height <= 0.0 {
        return;
    }
    primitives.push(NativePanelVisualPrimitive::RoundRect {
        frame: inner,
        radius: (radius - 1.0).max(0.0).min(inner.height / 2.0),
        color: card_shell_fill_color(card.style),
    });
}

fn card_shell_border_color(style: NativePanelVisualCardStyle) -> NativePanelVisualColor {
    visual_color_from_card_spec(card_visual_shell_border_color(
        card_visual_style_from_visual_style(style),
    ))
}

fn card_shell_fill_color(style: NativePanelVisualCardStyle) -> NativePanelVisualColor {
    visual_color_from_card_spec(card_visual_shell_fill_color(
        card_visual_style_from_visual_style(style),
    ))
}

fn push_expanded_card_content(
    output: &mut Vec<NativePanelVisualPrimitive>,
    card: &NativePanelVisualCardInput,
    frame: PanelRect,
    visible_frame: PanelRect,
    phase: f64,
) {
    let content_reveal = card_visual_content_transition_frame(phase, card.removing);
    if content_reveal.visibility_progress <= 0.001 || frame.height < card.collapsed_height.min(48.0)
    {
        return;
    }

    let fade_base = card_shell_fill_color(card.style);
    let content_layout = card_visual_content_layout(frame);
    if content_layout.content_width <= 8.0 {
        return;
    }

    let mut content_primitives = Vec::new();
    let primitives = &mut content_primitives;
    let header_text_spec =
        card_visual_header_text_paint_spec(card_visual_style_from_visual_style(card.style));
    if card.style == NativePanelVisualCardStyle::Empty {
        primitives.push(NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CardTitle,
            origin: PanelPoint {
                x: frame.x,
                y: content_layout.empty_title_y,
            },
            max_width: frame.width,
            text: card.title.clone(),
            color: visual_color_from_card_spec(header_text_spec.title.color),
            size: header_text_spec.title.size,
            weight: NativePanelVisualTextWeight::Semibold,
            alignment: NativePanelVisualTextAlignment::Center,
            alpha: 1.0,
        });
        apply_card_content_reveal_to_primitives(
            &mut content_primitives,
            content_reveal.translate_y,
            content_reveal.visibility_progress,
            fade_base,
        );
        extend_visible_content_primitives(output, content_primitives, visible_frame);
        return;
    }

    let mut badge_right = frame.x + frame.width - 12.0;
    let status_left = if let Some(badge) = &card.badge {
        push_expanded_card_badge(
            primitives,
            badge,
            badge_right,
            content_layout.title_y,
            card.style,
            CardVisualBadgeRole::Status,
        )
    } else {
        badge_right
    };
    badge_right = status_left;
    let source_left = if let Some(badge) = &card.source_badge {
        push_expanded_card_badge(
            primitives,
            badge,
            badge_right - 6.0,
            content_layout.title_y,
            card.style,
            CardVisualBadgeRole::Source,
        )
    } else {
        status_left
    };
    let title_width = (source_left - content_layout.content_x - 8.0).max(32.0);
    let title_text = fit_text_to_width(
        &compact_title(&card.title, header_text_spec.title_max_chars),
        title_width,
        header_text_spec.title.size as f64,
        1,
    );
    primitives.push(NativePanelVisualPrimitive::Text {
        role: NativePanelVisualTextRole::CardTitle,
        origin: PanelPoint {
            x: content_layout.content_x,
            y: content_layout.title_y,
        },
        max_width: title_width,
        text: title_text,
        color: visual_color_from_card_spec(header_text_spec.title.color),
        size: header_text_spec.title.size,
        weight: NativePanelVisualTextWeight::Semibold,
        alignment: NativePanelVisualTextAlignment::Left,
        alpha: 1.0,
    });

    if let Some(subtitle) = &card.subtitle {
        primitives.push(NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CardSubtitle,
            origin: PanelPoint {
                x: content_layout.content_x,
                y: content_layout.subtitle_y,
            },
            max_width: content_layout.content_width,
            text: fit_text_to_width(
                subtitle,
                content_layout.content_width,
                header_text_spec.subtitle.size as f64,
                1,
            ),
            color: visual_color_from_card_spec(header_text_spec.subtitle.color),
            size: header_text_spec.subtitle.size,
            weight: NativePanelVisualTextWeight::Normal,
            alignment: NativePanelVisualTextAlignment::Left,
            alpha: 1.0,
        });
    }

    if !card.rows.is_empty() {
        push_expanded_settings_rows(
            primitives,
            card,
            frame,
            content_layout.content_x,
            content_layout.content_width,
        );
        apply_card_content_reveal_to_primitives(
            &mut content_primitives,
            content_reveal.translate_y,
            content_reveal.visibility_progress,
            fade_base,
        );
        extend_visible_content_primitives(output, content_primitives, visible_frame);
        return;
    }

    if card.body.is_some() || !card.body_lines.is_empty() {
        push_expanded_card_body_line(primitives, card, frame, card.body.as_deref().unwrap_or(""));
    }

    if let Some(action_hint) = &card.action_hint {
        push_pending_action_hint_pill(primitives, frame, action_hint);
    }

    apply_card_content_reveal_to_primitives(
        &mut content_primitives,
        content_reveal.translate_y,
        content_reveal.visibility_progress,
        fade_base,
    );
    extend_visible_content_primitives(output, content_primitives, visible_frame);
}

fn apply_card_content_reveal_to_primitives(
    primitives: &mut [NativePanelVisualPrimitive],
    translate_y: f64,
    progress: f64,
    fade_base: NativePanelVisualColor,
) {
    for primitive in primitives {
        translate_primitive_y(primitive, translate_y);
        fade_primitive_color(primitive, fade_base, progress);
    }
}

fn translate_primitive_y(primitive: &mut NativePanelVisualPrimitive, translate_y: f64) {
    match primitive {
        NativePanelVisualPrimitive::RoundRect { frame, .. }
        | NativePanelVisualPrimitive::Rect { frame, .. }
        | NativePanelVisualPrimitive::Ellipse { frame, .. }
        | NativePanelVisualPrimitive::MascotRoundRect { frame, .. }
        | NativePanelVisualPrimitive::MascotEllipse { frame, .. }
        | NativePanelVisualPrimitive::MascotSprite { frame, .. }
        | NativePanelVisualPrimitive::CompactShoulder { frame, .. }
        | NativePanelVisualPrimitive::CompletionGlow { frame, .. }
        | NativePanelVisualPrimitive::ClipStart { frame } => {
            frame.y += translate_y;
        }
        NativePanelVisualPrimitive::StrokeLine { from, to, .. } => {
            from.y += translate_y;
            to.y += translate_y;
        }
        NativePanelVisualPrimitive::Text { origin, .. } => {
            origin.y += translate_y;
        }
        NativePanelVisualPrimitive::MascotText { origin, .. } => {
            origin.y += translate_y;
        }
        NativePanelVisualPrimitive::MascotDot { center, frame, .. } => {
            center.y += translate_y;
            frame.y += translate_y;
        }
        NativePanelVisualPrimitive::ClipEnd => {}
    }
}

fn fade_primitive_color(
    primitive: &mut NativePanelVisualPrimitive,
    fade_base: NativePanelVisualColor,
    progress: f64,
) {
    match primitive {
        NativePanelVisualPrimitive::RoundRect { color, .. }
        | NativePanelVisualPrimitive::Rect { color, .. }
        | NativePanelVisualPrimitive::Ellipse { color, .. }
        | NativePanelVisualPrimitive::MascotRoundRect { color, .. }
        | NativePanelVisualPrimitive::MascotEllipse { color, .. }
        | NativePanelVisualPrimitive::StrokeLine { color, .. }
        | NativePanelVisualPrimitive::Text { color, .. } => {
            *color = blend_visual_color(fade_base, *color, progress);
        }
        NativePanelVisualPrimitive::MascotText { color, alpha, .. } => {
            *color = blend_visual_color(fade_base, *color, progress);
            *alpha *= progress.clamp(0.0, 1.0);
        }
        NativePanelVisualPrimitive::CompactShoulder { fill, border, .. } => {
            *fill = blend_visual_color(fade_base, *fill, progress);
            *border = blend_visual_color(fade_base, *border, progress);
        }
        NativePanelVisualPrimitive::CompletionGlow { opacity, .. } => {
            *opacity *= progress.clamp(0.0, 1.0);
        }
        NativePanelVisualPrimitive::MascotSprite { opacity, .. } => {
            *opacity *= progress.clamp(0.0, 1.0);
        }
        NativePanelVisualPrimitive::MascotDot { .. }
        | NativePanelVisualPrimitive::ClipStart { .. }
        | NativePanelVisualPrimitive::ClipEnd => {}
    }
}

fn blend_visual_color(
    from: NativePanelVisualColor,
    to: NativePanelVisualColor,
    progress: f64,
) -> NativePanelVisualColor {
    let progress = progress.clamp(0.0, 1.0);
    NativePanelVisualColor::rgb(
        lerp(from.r as f64, to.r as f64, progress).round() as u8,
        lerp(from.g as f64, to.g as f64, progress).round() as u8,
        lerp(from.b as f64, to.b as f64, progress).round() as u8,
    )
}

fn extend_visible_content_primitives(
    output: &mut Vec<NativePanelVisualPrimitive>,
    primitives: Vec<NativePanelVisualPrimitive>,
    visible_frame: PanelRect,
) {
    let visible_primitives = primitives
        .into_iter()
        .filter(|primitive| primitive_intersects_vertical_bounds(primitive, visible_frame))
        .collect::<Vec<_>>();
    if visible_primitives.is_empty() {
        return;
    }

    output.push(NativePanelVisualPrimitive::ClipStart {
        frame: visible_frame,
    });
    output.extend(visible_primitives);
    output.push(NativePanelVisualPrimitive::ClipEnd);
}

fn push_expanded_card_body_line(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    card: &NativePanelVisualCardInput,
    frame: PanelRect,
    body: &str,
) {
    let metrics = default_panel_card_metric_constants();
    let body_layout = card_visual_body_layout(frame, card.action_hint.is_some());
    let mut cursor_y = body_layout.initial_y;
    let body_lines = expanded_card_body_lines(card, body);
    for (index, line) in body_lines.iter().enumerate() {
        if line.role == NativePanelVisualCardBodyRole::Tool {
            push_expanded_tool_pill_line(primitives, frame, cursor_y, &line.text);
            cursor_y += 22.0;
            if index + 1 < body_lines.len() {
                cursor_y += metrics.tool_gap;
            }
            continue;
        }

        let body_text =
            fit_text_to_lines(&line.text, body_layout.body_width, 10.0, line.max_lines).join("\n");
        let body_height = resolve_estimated_chat_body_height(
            &body_text,
            body_layout.body_width,
            line.max_lines as isize,
            metrics,
        );
        if let Some(prefix) = &line.prefix {
            let line_spec = card_visual_body_line_paint_spec(
                card_visual_style_from_visual_style(card.style),
                card_visual_body_role_from_visual_role(line.role),
                Some(prefix),
            );
            primitives.push(NativePanelVisualPrimitive::Text {
                role: NativePanelVisualTextRole::CardBodyPrefix,
                origin: PanelPoint {
                    x: body_layout.prefix_x,
                    y: cursor_y,
                },
                max_width: 10.0,
                text: prefix.clone(),
                color: visual_color_from_card_spec(line_spec.prefix_color),
                size: line_spec.prefix_size,
                weight: NativePanelVisualTextWeight::Bold,
                alignment: NativePanelVisualTextAlignment::Center,
                alpha: 1.0,
            });
        }
        let line_spec = card_visual_body_line_paint_spec(
            card_visual_style_from_visual_style(card.style),
            card_visual_body_role_from_visual_role(line.role),
            line.prefix.as_deref(),
        );
        primitives.push(NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CardBodyText,
            origin: PanelPoint {
                x: body_layout.text_x,
                y: cursor_y,
            },
            max_width: body_layout.body_width,
            text: body_text,
            color: visual_color_from_card_spec(line_spec.text_color),
            size: line_spec.text_size,
            weight: NativePanelVisualTextWeight::Normal,
            alignment: NativePanelVisualTextAlignment::Left,
            alpha: 1.0,
        });
        cursor_y += body_height;
        if index + 1 < body_lines.len() {
            cursor_y += metrics.chat_gap;
        }
    }
}

fn push_pending_action_hint_pill(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    frame: PanelRect,
    action_hint: &str,
) {
    let Some(layout) = card_visual_action_hint_layout(frame, action_hint) else {
        return;
    };
    primitives.push(NativePanelVisualPrimitive::RoundRect {
        frame: layout.pill_frame,
        radius: layout.paint.radius,
        color: visual_color_from_card_spec(layout.paint.background_color),
    });
    primitives.push(NativePanelVisualPrimitive::Text {
        role: NativePanelVisualTextRole::CardActionHint,
        origin: layout.text_origin,
        max_width: layout.text_max_width,
        text: fit_text_to_width(
            &layout.paint.text,
            layout.text_max_width,
            layout.paint.text_size as f64,
            1,
        ),
        color: visual_color_from_card_spec(layout.paint.foreground_color),
        size: layout.paint.text_size,
        weight: NativePanelVisualTextWeight::Semibold,
        alignment: NativePanelVisualTextAlignment::Left,
        alpha: 1.0,
    });
}

fn push_expanded_tool_pill_line(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    frame: PanelRect,
    y: f64,
    text: &str,
) {
    let Some(layout) = card_visual_tool_pill_layout(frame, y, text) else {
        return;
    };

    primitives.push(NativePanelVisualPrimitive::RoundRect {
        frame: layout.pill_frame,
        radius: layout.paint.radius,
        color: visual_color_from_card_spec(layout.paint.border_color),
    });
    let fill_frame = inset_rect(layout.pill_frame, 1.0);
    if fill_frame.width > 0.0 && fill_frame.height > 0.0 {
        primitives.push(NativePanelVisualPrimitive::RoundRect {
            frame: fill_frame,
            radius: (layout.paint.radius - 1.0).max(0.0),
            color: visual_color_from_card_spec(layout.paint.background_color),
        });
    }

    primitives.push(NativePanelVisualPrimitive::Text {
        role: NativePanelVisualTextRole::CardToolName,
        origin: layout.tool_name_origin,
        max_width: layout.tool_name_max_width,
        text: layout.paint.tool_name.clone(),
        color: visual_color_from_card_spec(layout.paint.tool_name_color),
        size: layout.paint.text_size,
        weight: NativePanelVisualTextWeight::Bold,
        alignment: NativePanelVisualTextAlignment::Left,
        alpha: 1.0,
    });

    if let Some(description) = layout.description {
        primitives.push(NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CardToolDescription,
            origin: description.origin,
            max_width: description.max_width,
            text: fit_text_to_width(
                &description.text,
                description.max_width,
                layout.paint.text_size as f64,
                1,
            ),
            color: visual_color_from_card_spec(layout.paint.description_color),
            size: layout.paint.text_size,
            weight: NativePanelVisualTextWeight::Normal,
            alignment: NativePanelVisualTextAlignment::Left,
            alpha: 1.0,
        });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExpandedCardBodyLine {
    role: NativePanelVisualCardBodyRole,
    prefix: Option<String>,
    text: String,
    max_lines: usize,
}

fn expanded_card_body_lines(
    card: &NativePanelVisualCardInput,
    body: &str,
) -> Vec<ExpandedCardBodyLine> {
    if !card.body_lines.is_empty() {
        return card
            .body_lines
            .iter()
            .filter_map(|line| {
                let text = line.text.trim();
                (!text.is_empty()).then(|| ExpandedCardBodyLine {
                    role: line.role,
                    prefix: line.prefix.clone(),
                    text: text.to_string(),
                    max_lines: line.max_lines.max(1),
                })
            })
            .collect();
    }

    let raw_lines = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let prefixes = card
        .body_prefix
        .as_deref()
        .unwrap_or_default()
        .chars()
        .map(|ch| ch.to_string())
        .collect::<Vec<_>>();
    if raw_lines.len() > 1 && prefixes.len() >= raw_lines.len() {
        return raw_lines
            .into_iter()
            .zip(prefixes)
            .map(|(text, prefix)| ExpandedCardBodyLine {
                role: expanded_card_body_role_for_prefix(Some(prefix.as_str())),
                max_lines: expanded_card_body_max_lines_for_prefix(
                    card.style,
                    Some(prefix.as_str()),
                ),
                prefix: Some(prefix),
                text: text.to_string(),
            })
            .collect();
    }

    let prefix = card.body_prefix.clone();
    vec![ExpandedCardBodyLine {
        role: expanded_card_body_role_for_prefix(prefix.as_deref()),
        max_lines: expanded_card_body_max_lines_for_prefix(card.style, prefix.as_deref()),
        prefix,
        text: body.to_string(),
    }]
}

fn expanded_card_body_role_for_prefix(prefix: Option<&str>) -> NativePanelVisualCardBodyRole {
    match prefix {
        Some("$") => NativePanelVisualCardBodyRole::Assistant,
        Some(">") => NativePanelVisualCardBodyRole::User,
        Some("!") => NativePanelVisualCardBodyRole::Tool,
        _ => NativePanelVisualCardBodyRole::Plain,
    }
}

fn expanded_card_body_max_lines_for_prefix(
    style: NativePanelVisualCardStyle,
    prefix: Option<&str>,
) -> usize {
    match (style, prefix) {
        (NativePanelVisualCardStyle::Default, Some(">")) => 1,
        (NativePanelVisualCardStyle::Default, Some("$"))
        | (NativePanelVisualCardStyle::Completion, _)
        | (NativePanelVisualCardStyle::Pending, _)
        | (NativePanelVisualCardStyle::PendingApproval, _)
        | (NativePanelVisualCardStyle::PendingQuestion, _)
        | (NativePanelVisualCardStyle::PromptAssist, _) => 2,
        _ => 1,
    }
}

fn push_expanded_card_badge(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    badge: &NativePanelVisualCardBadgeInput,
    right: f64,
    title_y: f64,
    style: NativePanelVisualCardStyle,
    role: CardVisualBadgeRole,
) -> f64 {
    let badge_spec = CardVisualBadgeSpec {
        role,
        text: badge.text.clone(),
        emphasized: badge.emphasized,
    };
    let layout = card_visual_badge_layout(
        card_visual_style_from_visual_style(style),
        &badge_spec,
        right,
        title_y,
    );
    primitives.push(NativePanelVisualPrimitive::RoundRect {
        frame: layout.badge_frame,
        radius: layout.paint.radius,
        color: visual_color_from_card_spec(layout.paint.background_color),
    });
    primitives.push(NativePanelVisualPrimitive::Text {
        role: card_badge_text_role(role),
        origin: layout.text_origin,
        max_width: layout.text_max_width,
        text: badge.text.clone(),
        color: visual_color_from_card_spec(layout.paint.foreground_color),
        size: layout.paint.text_size,
        weight: NativePanelVisualTextWeight::Semibold,
        alignment: NativePanelVisualTextAlignment::Center,
        alpha: 1.0,
    });
    layout.badge_frame.x
}

fn card_badge_text_role(role: CardVisualBadgeRole) -> NativePanelVisualTextRole {
    match role {
        CardVisualBadgeRole::Status => NativePanelVisualTextRole::CardStatusBadge,
        CardVisualBadgeRole::Source => NativePanelVisualTextRole::CardSourceBadge,
    }
}

fn fit_text_to_width(text: &str, width: f64, font_size: f64, max_lines: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return String::new();
    }
    let max_width = width.max(font_size) * max_lines.max(1) as f64;
    if resolve_estimated_text_width(&normalized, font_size) <= max_width {
        return normalized;
    }

    let mut clipped = String::new();
    for ch in normalized.chars() {
        let candidate = format!("{clipped}{ch}...");
        if resolve_estimated_text_width(&candidate, font_size) > max_width {
            break;
        }
        clipped.push(ch);
    }
    if clipped.is_empty() {
        "...".to_string()
    } else {
        format!("{}...", clipped.trim_end())
    }
}

fn fit_text_to_lines(text: &str, width: f64, font_size: f64, max_lines: usize) -> Vec<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Vec::new();
    }

    let max_lines = max_lines.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in normalized.chars() {
        let candidate = format!("{current}{ch}");
        if !current.is_empty() && resolve_estimated_text_width(&candidate, font_size) > width {
            lines.push(current.trim_end().to_string());
            current.clear();
            if lines.len() == max_lines {
                break;
            }
        }
        current.push(ch);
    }
    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
    }
    if !text_fits_in_lines(&normalized, &lines)
        && let Some(last) = lines.last_mut()
    {
        *last = ellipsize_text_to_width(last, width, font_size);
    }
    lines
}

fn text_fits_in_lines(original: &str, lines: &[String]) -> bool {
    lines.join("").chars().count() >= original.chars().count()
}

fn ellipsize_text_to_width(text: &str, width: f64, font_size: f64) -> String {
    let ellipsis = "...";
    if resolve_estimated_text_width(text, font_size) <= width
        && !text.ends_with(ellipsis)
        && resolve_estimated_text_width(&format!("{text}{ellipsis}"), font_size) <= width
    {
        return text.to_string();
    }

    let mut clipped = String::new();
    for ch in text.chars() {
        let candidate = format!("{clipped}{ch}{ellipsis}");
        if resolve_estimated_text_width(&candidate, font_size) > width {
            break;
        }
        clipped.push(ch);
    }
    if clipped.is_empty() {
        ellipsis.to_string()
    } else {
        format!("{}{}", clipped.trim_end(), ellipsis)
    }
}

fn push_expanded_settings_rows(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    card: &NativePanelVisualCardInput,
    frame: PanelRect,
    _content_x: f64,
    _content_width: f64,
) {
    for (index, row) in card.rows.iter().enumerate() {
        let row_spec = CardVisualRowSpec {
            title: row.title.clone(),
            value: row.value.clone(),
            active: row.active,
        };
        let Some(layout) = card_visual_settings_row_layout(frame, index, &row_spec) else {
            break;
        };
        primitives.push(NativePanelVisualPrimitive::RoundRect {
            frame: layout.row_frame,
            radius: layout.paint.border_radius,
            color: visual_color_from_card_spec(layout.paint.border_color),
        });
        primitives.push(NativePanelVisualPrimitive::RoundRect {
            frame: layout.row_inner_frame,
            radius: layout.paint.fill_radius,
            color: visual_color_from_card_spec(layout.paint.fill_color),
        });

        primitives.push(NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CardSettingsTitle,
            origin: layout.title_origin,
            max_width: layout.title_max_width,
            text: row.title.clone(),
            color: visual_color_from_card_spec(layout.paint.title_color),
            size: layout.paint.title_size,
            weight: NativePanelVisualTextWeight::Semibold,
            alignment: NativePanelVisualTextAlignment::Left,
            alpha: 1.0,
        });
        primitives.push(NativePanelVisualPrimitive::RoundRect {
            frame: layout.value_badge_frame,
            radius: layout.paint.value_badge.radius,
            color: visual_color_from_card_spec(layout.paint.value_badge.background_color),
        });
        primitives.push(NativePanelVisualPrimitive::Text {
            role: NativePanelVisualTextRole::CardSettingsValue,
            origin: layout.value_origin,
            max_width: layout.value_max_width,
            text: fit_text_to_width(
                &row.value,
                layout.value_max_width,
                layout.paint.value_badge.text_size as f64,
                1,
            ),
            color: visual_color_from_card_spec(layout.paint.value_badge.foreground_color),
            size: layout.paint.value_badge.text_size,
            weight: NativePanelVisualTextWeight::Semibold,
            alignment: NativePanelVisualTextAlignment::Center,
            alpha: 1.0,
        });
    }
}

fn push_compact_shoulder_primitive(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    frame: PanelRect,
    side: NativePanelVisualShoulderSide,
    progress: f64,
) {
    if frame.width <= 0.0 || frame.height <= 0.0 {
        return;
    }
    let progress = progress.clamp(0.0, 1.0);
    if progress >= 0.999 {
        return;
    }
    primitives.push(NativePanelVisualPrimitive::CompactShoulder {
        frame,
        side,
        progress,
        fill: NativePanelVisualColor::rgb(12, 12, 15),
        border: NativePanelVisualColor::rgb(44, 44, 50),
    });
}

fn non_zero_rect(rect: PanelRect) -> Option<PanelRect> {
    (rect.width > 0.0 && rect.height > 0.0).then_some(rect)
}

fn inset_rect(rect: PanelRect, inset: f64) -> PanelRect {
    PanelRect {
        x: rect.x + inset,
        y: rect.y + inset,
        width: (rect.width - inset * 2.0).max(0.0),
        height: (rect.height - inset * 2.0).max(0.0),
    }
}

fn clip_rect_vertically(rect: PanelRect, bounds: PanelRect) -> Option<PanelRect> {
    let bottom = rect.y.max(bounds.y);
    let top = (rect.y + rect.height).min(bounds.y + bounds.height);
    (top > bottom).then_some(PanelRect {
        x: rect.x,
        y: bottom,
        width: rect.width,
        height: top - bottom,
    })
}

fn primitive_intersects_vertical_bounds(
    primitive: &NativePanelVisualPrimitive,
    bounds: PanelRect,
) -> bool {
    let Some((bottom, top)) = primitive_vertical_bounds(primitive) else {
        return true;
    };
    top > bounds.y && bottom < bounds.y + bounds.height
}

fn primitive_fits_vertical_bounds(
    primitive: &NativePanelVisualPrimitive,
    bounds: PanelRect,
) -> bool {
    let Some((bottom, top)) = primitive_vertical_bounds(primitive) else {
        return true;
    };
    bottom >= bounds.y && top <= bounds.y + bounds.height
}

fn primitive_vertical_bounds(primitive: &NativePanelVisualPrimitive) -> Option<(f64, f64)> {
    match primitive {
        NativePanelVisualPrimitive::RoundRect { frame, .. }
        | NativePanelVisualPrimitive::Rect { frame, .. }
        | NativePanelVisualPrimitive::Ellipse { frame, .. }
        | NativePanelVisualPrimitive::MascotRoundRect { frame, .. }
        | NativePanelVisualPrimitive::MascotEllipse { frame, .. }
        | NativePanelVisualPrimitive::MascotSprite { frame, .. }
        | NativePanelVisualPrimitive::CompactShoulder { frame, .. }
        | NativePanelVisualPrimitive::CompletionGlow { frame, .. }
        | NativePanelVisualPrimitive::ClipStart { frame } => {
            Some((frame.y, frame.y + frame.height))
        }
        NativePanelVisualPrimitive::StrokeLine { from, to, .. } => {
            Some((from.y.min(to.y), from.y.max(to.y)))
        }
        NativePanelVisualPrimitive::Text {
            origin,
            text,
            size,
            role,
            ..
        } => {
            let height = native_panel_visual_text_box_height_for_role(*role, text, *size);
            Some((origin.y, origin.y + height))
        }
        NativePanelVisualPrimitive::MascotText {
            origin, text, size, ..
        } => {
            let height = native_panel_visual_text_box_height(text, *size);
            Some((origin.y, origin.y + height))
        }
        NativePanelVisualPrimitive::MascotDot { frame, .. } => {
            Some((frame.y, frame.y + frame.height))
        }
        NativePanelVisualPrimitive::ClipEnd => None,
    }
}

#[cfg(test)]
mod tests;
