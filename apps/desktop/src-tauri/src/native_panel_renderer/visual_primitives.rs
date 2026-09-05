use crate::{
    native_panel_core::{PanelPoint, PanelRect},
    native_panel_scene::SceneMascotPose,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativePanelVisualColor {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
}

impl NativePanelVisualColor {
    pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

pub(crate) fn native_panel_visual_text_box_height(text: &str, size: i32) -> f64 {
    let line_count = text.lines().count().max(1) as f64;
    let line_height = if size >= 13 { 24.0 } else { size as f64 + 8.0 };
    line_count * line_height
}

pub(crate) fn native_panel_visual_text_box_height_for_role(
    role: NativePanelVisualTextRole,
    text: &str,
    size: i32,
) -> f64 {
    let line_count = text.lines().count().max(1) as f64;
    let line_height = match role {
        NativePanelVisualTextRole::CardBodyText | NativePanelVisualTextRole::CardBodyPrefix => {
            crate::native_panel_core::CARD_CHAT_LINE_HEIGHT
        }
        NativePanelVisualTextRole::CardStatusBadge
        | NativePanelVisualTextRole::CardSourceBadge
        | NativePanelVisualTextRole::CardToolName
        | NativePanelVisualTextRole::CardToolDescription
        | NativePanelVisualTextRole::CardActionHint
        | NativePanelVisualTextRole::CardSettingsValue => size as f64 + 3.0,
        _ => {
            return native_panel_visual_text_box_height(text, size);
        }
    };
    line_count * line_height.max(1.0)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum NativePanelVisualPrimitive {
    ClipStart {
        frame: PanelRect,
    },
    ClipEnd,
    CompletionGlow {
        frame: PanelRect,
        opacity: f64,
    },
    RoundRect {
        frame: PanelRect,
        radius: f64,
        color: NativePanelVisualColor,
    },
    Rect {
        frame: PanelRect,
        color: NativePanelVisualColor,
    },
    Ellipse {
        frame: PanelRect,
        color: NativePanelVisualColor,
    },
    StrokeLine {
        from: PanelPoint,
        to: PanelPoint,
        color: NativePanelVisualColor,
        width: i32,
    },
    Text {
        role: NativePanelVisualTextRole,
        origin: PanelPoint,
        max_width: f64,
        text: String,
        color: NativePanelVisualColor,
        size: i32,
        weight: NativePanelVisualTextWeight,
        alignment: NativePanelVisualTextAlignment,
        alpha: f64,
    },
    MascotRoundRect {
        role: NativePanelVisualMascotRoundRectRole,
        frame: PanelRect,
        radius: f64,
        color: NativePanelVisualColor,
        alpha: f64,
    },
    MascotEllipse {
        role: NativePanelVisualMascotEllipseRole,
        frame: PanelRect,
        color: NativePanelVisualColor,
        alpha: f64,
    },
    MascotText {
        role: NativePanelVisualMascotTextRole,
        origin: PanelPoint,
        max_width: f64,
        text: String,
        color: NativePanelVisualColor,
        size: i32,
        weight: NativePanelVisualTextWeight,
        alignment: NativePanelVisualTextAlignment,
        alpha: f64,
    },
    MascotSprite {
        sprite_path: String,
        source_rect: PanelRect,
        frame: PanelRect,
        opacity: f64,
    },
    MascotDot {
        center: PanelPoint,
        frame: PanelRect,
        radius: f64,
        corner_radius: f64,
        scale_x: f64,
        scale_y: f64,
        pose: SceneMascotPose,
        debug_mode_enabled: bool,
        fill: NativePanelVisualColor,
        stroke: NativePanelVisualColor,
        stroke_width: f64,
        shadow_opacity: f64,
        shadow_radius: f64,
        alpha: f64,
    },
    CompactShoulder {
        frame: PanelRect,
        side: NativePanelVisualShoulderSide,
        progress: f64,
        fill: NativePanelVisualColor,
        border: NativePanelVisualColor,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualTextWeight {
    Normal,
    Semibold,
    Bold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualTextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualTextRole {
    Unspecified,
    CompactHeadline,
    CompactActiveCount,
    CompactActiveCountNext,
    CompactSlash,
    CompactTotalCount,
    ActionButtonSettings,
    ActionButtonQuit,
    CardTitle,
    CardSubtitle,
    CardStatusBadge,
    CardSourceBadge,
    CardBodyPrefix,
    CardBodyText,
    CardToolName,
    CardToolDescription,
    CardActionHint,
    CardSettingsTitle,
    CardSettingsValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualMascotRoundRectRole {
    Mouth,
    MessageBubble,
    CompletionBadgeOutline,
    CompletionBadgeFill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualMascotEllipseRole {
    LeftEye,
    RightEye,
    MessageBubbleDot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualMascotTextRole {
    SleepLabel,
    CompletionBadgeLabel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualShoulderSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NativePanelVisualPlan {
    pub(crate) hidden: bool,
    pub(crate) primitives: Vec<NativePanelVisualPrimitive>,
}

pub(crate) fn native_panel_visual_text_primitive_by_role(
    plan: &NativePanelVisualPlan,
    expected_role: NativePanelVisualTextRole,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives.iter().find(|primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::Text { role, .. } if *role == expected_role
        )
    })
}

pub(crate) fn native_panel_visual_text_primitive_by_text<'a>(
    plan: &'a NativePanelVisualPlan,
    expected_text: &str,
) -> Option<&'a NativePanelVisualPrimitive> {
    plan.primitives.iter().find(|primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::Text { text, .. } if text == expected_text
        )
    })
}

pub(crate) fn native_panel_visual_completion_glow_primitive(
    plan: &NativePanelVisualPlan,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives
        .iter()
        .find(|primitive| matches!(primitive, NativePanelVisualPrimitive::CompletionGlow { .. }))
}

pub(crate) fn native_panel_visual_compact_shoulder_primitive(
    plan: &NativePanelVisualPlan,
    expected_side: NativePanelVisualShoulderSide,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives.iter().find(|primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::CompactShoulder { side, .. } if *side == expected_side
        )
    })
}

pub(crate) fn native_panel_visual_mascot_body_primitive(
    plan: &NativePanelVisualPlan,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives
        .iter()
        .find(|primitive| matches!(primitive, NativePanelVisualPrimitive::MascotDot { .. }))
}

pub(crate) fn native_panel_visual_mascot_sprite_primitive(
    plan: &NativePanelVisualPlan,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives
        .iter()
        .find(|primitive| matches!(primitive, NativePanelVisualPrimitive::MascotSprite { .. }))
}

pub(crate) fn native_panel_visual_mascot_round_rect_primitive(
    plan: &NativePanelVisualPlan,
    expected_role: NativePanelVisualMascotRoundRectRole,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives.iter().find(|primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::MascotRoundRect { role, .. } if *role == expected_role
        )
    })
}

pub(crate) fn native_panel_visual_mascot_ellipse_primitive(
    plan: &NativePanelVisualPlan,
    expected_role: NativePanelVisualMascotEllipseRole,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives.iter().find(|primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::MascotEllipse { role, .. } if *role == expected_role
        )
    })
}

pub(crate) fn native_panel_visual_mascot_ellipse_primitives_by_role(
    plan: &NativePanelVisualPlan,
    expected_role: NativePanelVisualMascotEllipseRole,
) -> impl Iterator<Item = &NativePanelVisualPrimitive> {
    plan.primitives.iter().filter(move |primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::MascotEllipse { role, .. } if *role == expected_role
        )
    })
}

pub(crate) fn native_panel_visual_mascot_text_primitive(
    plan: &NativePanelVisualPlan,
    expected_role: NativePanelVisualMascotTextRole,
) -> Option<&NativePanelVisualPrimitive> {
    plan.primitives.iter().find(|primitive| {
        matches!(
            primitive,
            NativePanelVisualPrimitive::MascotText { role, .. } if *role == expected_role
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        NativePanelVisualColor, NativePanelVisualMascotEllipseRole,
        NativePanelVisualMascotRoundRectRole, NativePanelVisualMascotTextRole,
        NativePanelVisualPlan, NativePanelVisualPrimitive, NativePanelVisualShoulderSide,
        NativePanelVisualTextRole, native_panel_visual_text_box_height,
        native_panel_visual_text_box_height_for_role,
    };
    use crate::{
        native_panel_core::{PanelPoint, PanelRect},
        native_panel_scene::SceneMascotPose,
    };

    #[test]
    fn visual_plan_carries_platform_neutral_primitives() {
        let plan = NativePanelVisualPlan {
            hidden: false,
            primitives: vec![
                NativePanelVisualPrimitive::CompletionGlow {
                    frame: PanelRect {
                        x: 0.0,
                        y: 0.0,
                        width: 120.0,
                        height: 48.0,
                    },
                    opacity: 0.5,
                },
                NativePanelVisualPrimitive::Text {
                    role: NativePanelVisualTextRole::CompactHeadline,
                    origin: PanelPoint { x: 12.0, y: 14.0 },
                    max_width: 120.0,
                    text: "EchoIsland".to_string(),
                    color: NativePanelVisualColor::rgb(230, 235, 245),
                    size: 13,
                    weight: super::NativePanelVisualTextWeight::Semibold,
                    alignment: super::NativePanelVisualTextAlignment::Center,
                    alpha: 1.0,
                },
                NativePanelVisualPrimitive::RoundRect {
                    frame: PanelRect {
                        x: 0.0,
                        y: 0.0,
                        width: 120.0,
                        height: 48.0,
                    },
                    radius: 24.0,
                    color: NativePanelVisualColor::rgb(18, 18, 22),
                },
                NativePanelVisualPrimitive::MascotRoundRect {
                    role: NativePanelVisualMascotRoundRectRole::Mouth,
                    frame: PanelRect {
                        x: 20.0,
                        y: 18.0,
                        width: 8.0,
                        height: 3.0,
                    },
                    radius: 1.5,
                    color: NativePanelVisualColor::rgb(255, 255, 255),
                    alpha: 1.0,
                },
                NativePanelVisualPrimitive::MascotEllipse {
                    role: NativePanelVisualMascotEllipseRole::LeftEye,
                    frame: PanelRect {
                        x: 16.0,
                        y: 22.0,
                        width: 4.0,
                        height: 4.0,
                    },
                    color: NativePanelVisualColor::rgb(255, 255, 255),
                    alpha: 1.0,
                },
                NativePanelVisualPrimitive::MascotText {
                    role: NativePanelVisualMascotTextRole::SleepLabel,
                    origin: PanelPoint { x: 28.0, y: 28.0 },
                    max_width: 10.0,
                    text: "Z".to_string(),
                    color: NativePanelVisualColor::rgb(255, 122, 36),
                    size: 9,
                    weight: super::NativePanelVisualTextWeight::Bold,
                    alignment: super::NativePanelVisualTextAlignment::Center,
                    alpha: 0.5,
                },
                NativePanelVisualPrimitive::MascotSprite {
                    sprite_path: "mascot/default/spritesheet.png".to_string(),
                    source_rect: PanelRect {
                        x: 0.0,
                        y: 0.0,
                        width: 102.4,
                        height: 192.0,
                    },
                    frame: PanelRect {
                        x: 6.0,
                        y: 4.0,
                        width: 54.0,
                        height: 54.0,
                    },
                    opacity: 1.0,
                },
                NativePanelVisualPrimitive::MascotDot {
                    center: PanelPoint { x: 24.0, y: 24.0 },
                    frame: PanelRect {
                        x: 12.0,
                        y: 14.0,
                        width: 24.0,
                        height: 20.0,
                    },
                    radius: 10.0,
                    corner_radius: 6.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    pose: SceneMascotPose::Complete,
                    debug_mode_enabled: false,
                    fill: NativePanelVisualColor::rgb(5, 5, 5),
                    stroke: NativePanelVisualColor::rgb(255, 122, 36),
                    stroke_width: 2.2,
                    shadow_opacity: 0.0,
                    shadow_radius: 0.0,
                    alpha: 1.0,
                },
                NativePanelVisualPrimitive::CompactShoulder {
                    frame: PanelRect {
                        x: -6.0,
                        y: 31.0,
                        width: 6.0,
                        height: 6.0,
                    },
                    side: NativePanelVisualShoulderSide::Left,
                    progress: 0.0,
                    fill: NativePanelVisualColor::rgb(12, 12, 15),
                    border: NativePanelVisualColor::rgb(44, 44, 50),
                },
            ],
        };

        assert!(!plan.hidden);
        assert_eq!(plan.primitives.len(), 9);
    }

    #[test]
    fn visual_text_box_height_is_shared_for_platform_text_rendering() {
        assert_eq!(native_panel_visual_text_box_height("EchoIsland", 13), 24.0);
        assert_eq!(native_panel_visual_text_box_height("2", 8), 16.0);
        assert_eq!(
            native_panel_visual_text_box_height("line one\nline two", 10),
            36.0
        );
    }

    #[test]
    fn visual_text_box_height_uses_tight_boxes_for_card_pill_text_roles() {
        assert_eq!(
            native_panel_visual_text_box_height_for_role(
                NativePanelVisualTextRole::CardStatusBadge,
                "空闲",
                10
            ),
            13.0
        );
        assert_eq!(
            native_panel_visual_text_box_height_for_role(
                NativePanelVisualTextRole::CardToolName,
                "Bash",
                9
            ),
            12.0
        );
        assert_eq!(
            native_panel_visual_text_box_height_for_role(
                NativePanelVisualTextRole::CompactHeadline,
                "EchoIsland",
                13
            ),
            24.0
        );
    }

    #[test]
    fn visual_text_box_height_uses_card_chat_line_height_for_body_roles() {
        assert_eq!(
            native_panel_visual_text_box_height_for_role(
                NativePanelVisualTextRole::CardBodyText,
                "line one\nline two",
                10
            ),
            crate::native_panel_core::CARD_CHAT_LINE_HEIGHT * 2.0
        );
        assert_eq!(
            native_panel_visual_text_box_height_for_role(
                NativePanelVisualTextRole::CardBodyPrefix,
                "$",
                10
            ),
            crate::native_panel_core::CARD_CHAT_LINE_HEIGHT
        );
    }
}
