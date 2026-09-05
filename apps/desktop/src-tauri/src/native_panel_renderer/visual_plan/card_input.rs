use crate::native_panel_scene::SceneCard;

use super::{
    super::{
        card_visual_spec::{
            CardVisualBadgeRole, CardVisualBadgeSpec, CardVisualBodyRole, CardVisualBodySpec,
            CardVisualColorSpec, CardVisualRowSpec, CardVisualSpec, CardVisualStyle,
            card_visual_spec_from_scene_card_with_height,
        },
        visual_primitives::NativePanelVisualColor,
    },
    input::{
        NativePanelVisualCardBadgeInput, NativePanelVisualCardBodyLineInput,
        NativePanelVisualCardBodyRole, NativePanelVisualCardInput, NativePanelVisualCardRowInput,
        NativePanelVisualCardStyle,
    },
};

pub(crate) fn native_panel_visual_card_input_from_scene_card(
    card: &SceneCard,
) -> NativePanelVisualCardInput {
    native_panel_visual_card_input_from_scene_card_with_height(card, 72.0)
}

pub(crate) fn native_panel_visual_card_input_from_scene_card_with_height(
    card: &SceneCard,
    height: f64,
) -> NativePanelVisualCardInput {
    visual_card_input_from_spec(card_visual_spec_from_scene_card_with_height(card, height))
}

fn visual_card_input_from_spec(spec: CardVisualSpec) -> NativePanelVisualCardInput {
    NativePanelVisualCardInput {
        style: visual_card_style_from_spec(spec.style),
        title: spec.title,
        subtitle: spec.subtitle,
        body: None,
        badge: visual_card_badge_from_spec(&spec.badges, CardVisualBadgeRole::Status),
        source_badge: visual_card_badge_from_spec(&spec.badges, CardVisualBadgeRole::Source),
        body_prefix: None,
        body_lines: spec
            .body
            .iter()
            .map(visual_card_body_line_from_spec)
            .collect(),
        action_hint: spec.action_hint,
        rows: spec.rows.iter().map(visual_card_row_from_spec).collect(),
        height: spec.height,
        collapsed_height: spec.animation.collapsed_height,
        compact: spec.compact,
        removing: spec.removing,
    }
}

fn visual_card_body_line_from_spec(
    line: &CardVisualBodySpec,
) -> NativePanelVisualCardBodyLineInput {
    NativePanelVisualCardBodyLineInput {
        role: visual_card_body_role_from_spec(line.role),
        prefix: line.prefix.clone(),
        text: line.text.clone(),
        max_lines: line.max_lines,
    }
}

fn visual_card_body_role_from_spec(role: CardVisualBodyRole) -> NativePanelVisualCardBodyRole {
    match role {
        CardVisualBodyRole::Assistant => NativePanelVisualCardBodyRole::Assistant,
        CardVisualBodyRole::User => NativePanelVisualCardBodyRole::User,
        CardVisualBodyRole::Tool => NativePanelVisualCardBodyRole::Tool,
        CardVisualBodyRole::Plain => NativePanelVisualCardBodyRole::Plain,
        CardVisualBodyRole::ActionHint => NativePanelVisualCardBodyRole::ActionHint,
    }
}

pub(super) fn card_visual_body_role_from_visual_role(
    role: NativePanelVisualCardBodyRole,
) -> CardVisualBodyRole {
    match role {
        NativePanelVisualCardBodyRole::Assistant => CardVisualBodyRole::Assistant,
        NativePanelVisualCardBodyRole::User => CardVisualBodyRole::User,
        NativePanelVisualCardBodyRole::Tool => CardVisualBodyRole::Tool,
        NativePanelVisualCardBodyRole::Plain => CardVisualBodyRole::Plain,
        NativePanelVisualCardBodyRole::ActionHint => CardVisualBodyRole::ActionHint,
    }
}

fn visual_card_style_from_spec(style: CardVisualStyle) -> NativePanelVisualCardStyle {
    match style {
        CardVisualStyle::Default => NativePanelVisualCardStyle::Default,
        CardVisualStyle::Pending => NativePanelVisualCardStyle::Pending,
        CardVisualStyle::PendingApproval => NativePanelVisualCardStyle::PendingApproval,
        CardVisualStyle::PendingQuestion => NativePanelVisualCardStyle::PendingQuestion,
        CardVisualStyle::PromptAssist => NativePanelVisualCardStyle::PromptAssist,
        CardVisualStyle::Completion => NativePanelVisualCardStyle::Completion,
        CardVisualStyle::Settings => NativePanelVisualCardStyle::Settings,
        CardVisualStyle::Empty => NativePanelVisualCardStyle::Empty,
    }
}

pub(super) fn card_visual_style_from_visual_style(
    style: NativePanelVisualCardStyle,
) -> CardVisualStyle {
    match style {
        NativePanelVisualCardStyle::Default => CardVisualStyle::Default,
        NativePanelVisualCardStyle::Pending => CardVisualStyle::Pending,
        NativePanelVisualCardStyle::PendingApproval => CardVisualStyle::PendingApproval,
        NativePanelVisualCardStyle::PendingQuestion => CardVisualStyle::PendingQuestion,
        NativePanelVisualCardStyle::PromptAssist => CardVisualStyle::PromptAssist,
        NativePanelVisualCardStyle::Completion => CardVisualStyle::Completion,
        NativePanelVisualCardStyle::Settings => CardVisualStyle::Settings,
        NativePanelVisualCardStyle::Empty => CardVisualStyle::Empty,
    }
}

fn visual_card_badge_from_spec(
    badges: &[CardVisualBadgeSpec],
    role: CardVisualBadgeRole,
) -> Option<NativePanelVisualCardBadgeInput> {
    badges
        .iter()
        .find(|badge| badge.role == role)
        .map(|badge| NativePanelVisualCardBadgeInput {
            text: badge.text.clone(),
            emphasized: badge.emphasized,
        })
}

fn visual_card_row_from_spec(row: &CardVisualRowSpec) -> NativePanelVisualCardRowInput {
    NativePanelVisualCardRowInput {
        title: row.title.clone(),
        value: row.value.clone(),
        active: row.active,
    }
}

pub(super) fn visual_color_from_card_spec(color: CardVisualColorSpec) -> NativePanelVisualColor {
    NativePanelVisualColor::rgb(color.r, color.g, color.b)
}
