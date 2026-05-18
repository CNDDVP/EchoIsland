use std::sync::OnceLock;

use crate::{native_panel_core::PanelPoint, native_panel_scene::SceneMascotPose};

use super::super::{
    env_flags::native_panel_enabled_from_env_value,
    mascot_sprite_spec::{
        MascotSpriteFrameInput, MascotSpriteFrameSpec, MascotSpriteManifest,
        parse_mascot_sprite_manifest, resolve_mascot_sprite_frame,
    },
    mascot_visual_spec::{
        MascotCompletionBadgeVisualSpec, MascotMessageBubbleVisualSpec, MascotRoundRectVisualSpec,
        MascotTextVisualSpec, MascotVisualSpec,
    },
    visual_primitives::{
        NativePanelVisualMascotEllipseRole, NativePanelVisualMascotRoundRectRole,
        NativePanelVisualMascotTextRole, NativePanelVisualPrimitive,
        NativePanelVisualTextAlignment,
    },
};

const DEFAULT_MASCOT_SPRITE_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/mascot/default/pet.json"
));
const DEFAULT_MASCOT_SPRITE_PATH: &str = "mascot/default/spritesheet.png";
static DEFAULT_MASCOT_SPRITE_MANIFEST_CACHE: OnceLock<Option<MascotSpriteManifest>> =
    OnceLock::new();

pub(super) fn apply_mascot_chrome_alpha(primitives: &mut [NativePanelVisualPrimitive], alpha: f64) {
    let alpha = alpha.clamp(0.0, 1.0);
    for primitive in primitives {
        match primitive {
            NativePanelVisualPrimitive::MascotDot {
                alpha: dot_alpha,
                shadow_opacity,
                ..
            } => {
                *dot_alpha *= alpha;
                *shadow_opacity *= alpha;
            }
            NativePanelVisualPrimitive::MascotRoundRect {
                alpha: primitive_alpha,
                ..
            }
            | NativePanelVisualPrimitive::MascotEllipse {
                alpha: primitive_alpha,
                ..
            }
            | NativePanelVisualPrimitive::MascotText {
                alpha: primitive_alpha,
                ..
            }
            | NativePanelVisualPrimitive::MascotSprite {
                opacity: primitive_alpha,
                ..
            } => {
                *primitive_alpha *= alpha;
            }
            _ => {}
        }
    }
}

pub(super) fn push_mascot_primitives(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    spec: &MascotVisualSpec,
) {
    if spec.pose == SceneMascotPose::Hidden {
        return;
    }

    if mascot_sprite_enabled() {
        if let Some(sprite) = resolve_default_mascot_sprite(spec) {
            primitives.push(NativePanelVisualPrimitive::MascotSprite {
                sprite_path: DEFAULT_MASCOT_SPRITE_PATH.to_string(),
                source_rect: sprite.source_rect,
                frame: sprite.frame,
                opacity: sprite.opacity,
            });
            if let Some(badge) = &spec.completion_badge {
                push_mascot_completion_badge(primitives, badge);
            }
            return;
        }
    }

    primitives.push(NativePanelVisualPrimitive::MascotDot {
        center: spec.body.center,
        frame: spec.body.frame,
        radius: spec.body.radius,
        corner_radius: spec.body.corner_radius,
        scale_x: spec.body.scale_x,
        scale_y: spec.body.scale_y,
        pose: spec.pose,
        debug_mode_enabled: false,
        fill: spec.body.fill_color,
        stroke: spec.body.stroke_color,
        stroke_width: spec.body.stroke_width,
        shadow_opacity: spec.body.shadow_opacity,
        shadow_radius: spec.body.shadow_radius,
        alpha: spec.motion.shell_alpha,
    });
    if let Some(message_bubble) = &spec.message_bubble {
        push_mascot_message_bubble(primitives, message_bubble);
    }
    if let Some(sleep_label) = &spec.sleep_label {
        push_mascot_text(
            primitives,
            NativePanelVisualMascotTextRole::SleepLabel,
            sleep_label,
        );
    }
    for (index, eye) in spec.eyes.iter().enumerate() {
        primitives.push(NativePanelVisualPrimitive::MascotEllipse {
            role: if index == 0 {
                NativePanelVisualMascotEllipseRole::LeftEye
            } else {
                NativePanelVisualMascotEllipseRole::RightEye
            },
            frame: eye.frame,
            color: eye.color,
            alpha: 1.0,
        });
    }
    push_mascot_round_rect(
        primitives,
        NativePanelVisualMascotRoundRectRole::Mouth,
        spec.mouth,
        1.0,
    );
    if let Some(badge) = &spec.completion_badge {
        push_mascot_completion_badge(primitives, badge);
    }
}

fn mascot_sprite_enabled() -> bool {
    native_panel_enabled_from_env_value(!cfg!(test), std::env::var("ECHOISLAND_MASCOT_SPRITE").ok())
}

fn resolve_default_mascot_sprite(spec: &MascotVisualSpec) -> Option<MascotSpriteFrameSpec> {
    let manifest = default_mascot_sprite_manifest()?;
    let center = sprite_locked_mascot_center(spec);
    resolve_mascot_sprite_frame(MascotSpriteFrameInput {
        manifest,
        pose: spec.pose,
        center,
        elapsed_ms: spec.elapsed_ms,
        opacity: spec.motion.shell_alpha,
    })
}

fn sprite_locked_mascot_center(spec: &MascotVisualSpec) -> PanelPoint {
    PanelPoint {
        x: spec.body.center.x - spec.motion.offset_x,
        y: spec.body.center.y - spec.motion.offset_y,
    }
}

fn default_mascot_sprite_manifest() -> Option<&'static MascotSpriteManifest> {
    DEFAULT_MASCOT_SPRITE_MANIFEST_CACHE
        .get_or_init(|| parse_mascot_sprite_manifest(DEFAULT_MASCOT_SPRITE_MANIFEST).ok())
        .as_ref()
}

fn push_mascot_message_bubble(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    bubble: &MascotMessageBubbleVisualSpec,
) {
    push_mascot_round_rect(
        primitives,
        NativePanelVisualMascotRoundRectRole::MessageBubble,
        bubble.bubble,
        bubble.alpha,
    );
    for dot in &bubble.dots {
        primitives.push(NativePanelVisualPrimitive::MascotEllipse {
            role: NativePanelVisualMascotEllipseRole::MessageBubbleDot,
            frame: dot.frame,
            color: dot.color,
            alpha: bubble.alpha,
        });
    }
}

fn push_mascot_completion_badge(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    badge: &MascotCompletionBadgeVisualSpec,
) {
    push_mascot_round_rect(
        primitives,
        NativePanelVisualMascotRoundRectRole::CompletionBadgeOutline,
        badge.outline,
        1.0,
    );
    push_mascot_round_rect(
        primitives,
        NativePanelVisualMascotRoundRectRole::CompletionBadgeFill,
        badge.fill,
        1.0,
    );
    push_mascot_text(
        primitives,
        NativePanelVisualMascotTextRole::CompletionBadgeLabel,
        &badge.label,
    );
}

fn push_mascot_round_rect(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    role: NativePanelVisualMascotRoundRectRole,
    spec: MascotRoundRectVisualSpec,
    alpha: f64,
) {
    primitives.push(NativePanelVisualPrimitive::MascotRoundRect {
        role,
        frame: spec.frame,
        radius: spec.radius,
        color: spec.color,
        alpha,
    });
}

fn push_mascot_text(
    primitives: &mut Vec<NativePanelVisualPrimitive>,
    role: NativePanelVisualMascotTextRole,
    spec: &MascotTextVisualSpec,
) {
    primitives.push(NativePanelVisualPrimitive::MascotText {
        role,
        origin: spec.origin,
        max_width: spec.max_width,
        text: spec.text.clone(),
        color: spec.color,
        size: spec.size,
        weight: spec.weight,
        alignment: NativePanelVisualTextAlignment::Center,
        alpha: spec.alpha,
    });
}
