use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{
    native_panel_core::{PanelPoint, PanelRect},
    native_panel_scene::SceneMascotPose,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MascotSpriteAnimationKey {
    Idle,
    Running,
    Approval,
    Question,
    Complete,
    Sleepy,
    WakeAngry,
}

impl MascotSpriteAnimationKey {
    pub(crate) fn manifest_key(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Approval => "approval",
            Self::Question => "question",
            Self::Complete => "complete",
            Self::Sleepy => "sleepy",
            Self::WakeAngry => "wake_angry",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MascotSpriteManifest {
    pub(crate) version: u32,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) sprite: MascotSpriteSheetSpec,
    pub(crate) animations: BTreeMap<String, MascotSpriteAnimationManifest>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MascotSpriteSheetSpec {
    pub(crate) file: String,
    pub(crate) columns: usize,
    pub(crate) rows: usize,
    #[serde(alias = "cell_width")]
    pub(crate) cell_width: f64,
    #[serde(alias = "cell_height")]
    pub(crate) cell_height: f64,
    #[serde(default = "default_sprite_pixel_ratio", alias = "pixel_ratio")]
    pub(crate) pixel_ratio: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MascotSpriteAnimationManifest {
    pub(crate) row: usize,
    pub(crate) frames: usize,
    #[serde(alias = "frame_ms")]
    pub(crate) frame_ms: u128,
    #[serde(rename = "loop")]
    pub(crate) looped: bool,
    #[serde(alias = "logical_width")]
    pub(crate) logical_width: f64,
    #[serde(alias = "logical_height")]
    pub(crate) logical_height: f64,
    #[serde(default = "default_sprite_anchor", alias = "anchor_x")]
    pub(crate) anchor_x: f64,
    #[serde(default = "default_sprite_anchor", alias = "anchor_y")]
    pub(crate) anchor_y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MascotSpriteFrameInput<'a> {
    pub(crate) manifest: &'a MascotSpriteManifest,
    pub(crate) pose: SceneMascotPose,
    pub(crate) center: PanelPoint,
    pub(crate) elapsed_ms: u128,
    pub(crate) opacity: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MascotSpriteFrameSpec {
    pub(crate) sprite_path: String,
    pub(crate) animation: MascotSpriteAnimationKey,
    pub(crate) frame_index: usize,
    pub(crate) source_rect: PanelRect,
    pub(crate) frame: PanelRect,
    pub(crate) opacity: f64,
}

pub(crate) fn parse_mascot_sprite_manifest(
    raw_manifest: &str,
) -> Result<MascotSpriteManifest, String> {
    let manifest = serde_json::from_str::<MascotSpriteManifest>(raw_manifest)
        .map_err(|error| format!("failed to parse mascot sprite manifest: {error}"))?;
    validate_mascot_sprite_manifest(&manifest)?;
    Ok(manifest)
}

pub(crate) fn validate_mascot_sprite_manifest(
    manifest: &MascotSpriteManifest,
) -> Result<(), String> {
    if manifest.version == 0 {
        return Err("mascot sprite manifest version must be greater than zero".to_string());
    }
    if manifest.id.trim().is_empty() {
        return Err("mascot sprite manifest id is required".to_string());
    }
    if manifest.sprite.file.trim().is_empty() {
        return Err("mascot sprite file is required".to_string());
    }
    if manifest.sprite.columns == 0 || manifest.sprite.rows == 0 {
        return Err("mascot sprite atlas must have at least one row and column".to_string());
    }
    if manifest.sprite.cell_width <= 0.0 || manifest.sprite.cell_height <= 0.0 {
        return Err("mascot sprite cell size must be positive".to_string());
    }
    if manifest.sprite.pixel_ratio <= 0.0 {
        return Err("mascot sprite pixel ratio must be positive".to_string());
    }
    if manifest.animations.is_empty() {
        return Err("mascot sprite manifest must declare animations".to_string());
    }
    for (name, animation) in &manifest.animations {
        if animation.row >= manifest.sprite.rows {
            return Err(format!(
                "mascot sprite animation '{name}' row is outside atlas"
            ));
        }
        if animation.frames == 0 || animation.frames > manifest.sprite.columns {
            return Err(format!(
                "mascot sprite animation '{name}' frames must be between 1 and atlas columns"
            ));
        }
        if animation.frame_ms == 0 {
            return Err(format!(
                "mascot sprite animation '{name}' frame_ms must be greater than zero"
            ));
        }
        if animation.logical_width <= 0.0 || animation.logical_height <= 0.0 {
            return Err(format!(
                "mascot sprite animation '{name}' logical size must be positive"
            ));
        }
        if !(0.0..=1.0).contains(&animation.anchor_x) || !(0.0..=1.0).contains(&animation.anchor_y)
        {
            return Err(format!(
                "mascot sprite animation '{name}' anchors must be between 0 and 1"
            ));
        }
    }
    Ok(())
}

pub(crate) fn resolve_mascot_sprite_animation_key(
    pose: SceneMascotPose,
) -> Option<MascotSpriteAnimationKey> {
    match pose {
        SceneMascotPose::Idle => Some(MascotSpriteAnimationKey::Idle),
        SceneMascotPose::Running => Some(MascotSpriteAnimationKey::Running),
        SceneMascotPose::Approval => Some(MascotSpriteAnimationKey::Approval),
        SceneMascotPose::Question => Some(MascotSpriteAnimationKey::Question),
        SceneMascotPose::Complete => Some(MascotSpriteAnimationKey::Complete),
        SceneMascotPose::Sleepy => Some(MascotSpriteAnimationKey::Sleepy),
        SceneMascotPose::WakeAngry => Some(MascotSpriteAnimationKey::WakeAngry),
        SceneMascotPose::MessageBubble => Some(MascotSpriteAnimationKey::Idle),
        SceneMascotPose::Hidden => None,
    }
}

pub(crate) fn resolve_mascot_sprite_frame(
    input: MascotSpriteFrameInput<'_>,
) -> Option<MascotSpriteFrameSpec> {
    let requested_animation = resolve_mascot_sprite_animation_key(input.pose)?;
    let (animation, spec) =
        mascot_sprite_animation_with_idle_fallback(input.manifest, requested_animation)?;
    let frame_index = resolve_mascot_sprite_frame_index(spec, input.elapsed_ms);
    let source_rect = PanelRect {
        x: input.manifest.sprite.cell_width * frame_index as f64,
        y: input.manifest.sprite.cell_height * spec.row as f64,
        width: input.manifest.sprite.cell_width,
        height: input.manifest.sprite.cell_height,
    };
    let frame = PanelRect {
        x: input.center.x - spec.logical_width * spec.anchor_x,
        y: input.center.y - spec.logical_height * spec.anchor_y,
        width: spec.logical_width,
        height: spec.logical_height,
    };

    Some(MascotSpriteFrameSpec {
        sprite_path: input.manifest.sprite.file.clone(),
        animation,
        frame_index,
        source_rect,
        frame,
        opacity: input.opacity.clamp(0.0, 1.0),
    })
}

fn mascot_sprite_animation_with_idle_fallback(
    manifest: &MascotSpriteManifest,
    animation: MascotSpriteAnimationKey,
) -> Option<(MascotSpriteAnimationKey, &MascotSpriteAnimationManifest)> {
    manifest
        .animations
        .get(animation.manifest_key())
        .map(|spec| (animation, spec))
        .or_else(|| {
            manifest
                .animations
                .get(MascotSpriteAnimationKey::Idle.manifest_key())
                .map(|spec| (MascotSpriteAnimationKey::Idle, spec))
        })
}

fn resolve_mascot_sprite_frame_index(
    animation: &MascotSpriteAnimationManifest,
    elapsed_ms: u128,
) -> usize {
    let raw_frame = (elapsed_ms / animation.frame_ms) as usize;
    if animation.looped {
        raw_frame % animation.frames
    } else {
        raw_frame.min(animation.frames - 1)
    }
}

fn default_sprite_pixel_ratio() -> f64 {
    1.0
}

fn default_sprite_anchor() -> f64 {
    0.5
}

#[cfg(test)]
mod tests {
    use super::{
        MascotSpriteAnimationKey, MascotSpriteFrameInput, parse_mascot_sprite_manifest,
        resolve_mascot_sprite_animation_key, resolve_mascot_sprite_frame,
    };
    use crate::{native_panel_core::PanelPoint, native_panel_scene::SceneMascotPose};

    const MANIFEST: &str = r#"{
      "version": 1,
      "id": "echoisland-default",
      "name": "EchoIsland Assistant",
      "sprite": {
        "file": "spritesheet.webp",
        "columns": 8,
        "rows": 9,
        "cell_width": 192,
        "cell_height": 208,
        "pixel_ratio": 4
      },
      "animations": {
        "idle": {
          "row": 0,
          "frames": 8,
          "frame_ms": 160,
          "loop": true,
          "logical_width": 54,
          "logical_height": 44,
          "anchor_x": 0.5,
          "anchor_y": 0.5
        },
        "running": {
          "row": 1,
          "frames": 8,
          "frame_ms": 120,
          "loop": true,
          "logical_width": 54,
          "logical_height": 44,
          "anchor_x": 0.5,
          "anchor_y": 0.5
        },
        "complete": {
          "row": 4,
          "frames": 6,
          "frame_ms": 100,
          "loop": false,
          "logical_width": 58,
          "logical_height": 46,
          "anchor_x": 0.5,
          "anchor_y": 0.5
        }
      }
    }"#;

    #[test]
    fn bundled_mascot_sprite_manifest_matches_generated_atlas_contract() {
        let manifest = parse_mascot_sprite_manifest(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/mascot/default/pet.json"
        )))
        .expect("bundled manifest");

        assert_eq!(manifest.sprite.file, "spritesheet.png");
        assert_eq!(manifest.sprite.columns, 10);
        assert_eq!(manifest.sprite.rows, 8);
        assert_eq!(manifest.animations.get("idle").expect("idle").frames, 10);
    }

    #[test]
    fn mascot_sprite_manifest_parses_codex_like_pet_pack_contract() {
        let manifest = parse_mascot_sprite_manifest(MANIFEST).expect("manifest");

        assert_eq!(manifest.id, "echoisland-default");
        assert_eq!(manifest.sprite.file, "spritesheet.webp");
        assert_eq!(manifest.sprite.columns, 8);
        assert_eq!(manifest.sprite.cell_width, 192.0);
        assert_eq!(manifest.animations.get("running").expect("running").row, 1);
    }

    #[test]
    fn mascot_sprite_animation_key_maps_scene_poses() {
        assert_eq!(
            resolve_mascot_sprite_animation_key(SceneMascotPose::Running),
            Some(MascotSpriteAnimationKey::Running)
        );
        assert_eq!(
            resolve_mascot_sprite_animation_key(SceneMascotPose::MessageBubble),
            Some(MascotSpriteAnimationKey::Idle)
        );
        assert_eq!(
            resolve_mascot_sprite_animation_key(SceneMascotPose::Hidden),
            None
        );
    }

    #[test]
    fn mascot_sprite_frame_resolves_source_and_destination_rects() {
        let manifest = parse_mascot_sprite_manifest(MANIFEST).expect("manifest");
        let frame = resolve_mascot_sprite_frame(MascotSpriteFrameInput {
            manifest: &manifest,
            pose: SceneMascotPose::Running,
            center: PanelPoint { x: 32.0, y: 18.0 },
            elapsed_ms: 260,
            opacity: 1.4,
        })
        .expect("sprite frame");

        assert_eq!(frame.animation, MascotSpriteAnimationKey::Running);
        assert_eq!(frame.frame_index, 2);
        assert_eq!(frame.source_rect.x, 384.0);
        assert_eq!(frame.source_rect.y, 208.0);
        assert_eq!(frame.frame.x, 5.0);
        assert_eq!(frame.frame.y, -4.0);
        assert_eq!(frame.opacity, 1.0);
    }

    #[test]
    fn mascot_sprite_frame_clamps_non_looping_animation_to_last_frame() {
        let manifest = parse_mascot_sprite_manifest(MANIFEST).expect("manifest");
        let frame = resolve_mascot_sprite_frame(MascotSpriteFrameInput {
            manifest: &manifest,
            pose: SceneMascotPose::Complete,
            center: PanelPoint { x: 32.0, y: 18.0 },
            elapsed_ms: 900,
            opacity: 0.8,
        })
        .expect("sprite frame");

        assert_eq!(frame.animation, MascotSpriteAnimationKey::Complete);
        assert_eq!(frame.frame_index, 5);
        assert_eq!(frame.source_rect.x, 960.0);
        assert_eq!(frame.source_rect.y, 832.0);
    }

    #[test]
    fn mascot_sprite_frame_falls_back_to_idle_when_pose_animation_is_missing() {
        let manifest = parse_mascot_sprite_manifest(MANIFEST).expect("manifest");
        let frame = resolve_mascot_sprite_frame(MascotSpriteFrameInput {
            manifest: &manifest,
            pose: SceneMascotPose::Question,
            center: PanelPoint { x: 32.0, y: 18.0 },
            elapsed_ms: 320,
            opacity: 1.0,
        })
        .expect("sprite frame");

        assert_eq!(frame.animation, MascotSpriteAnimationKey::Idle);
        assert_eq!(frame.frame_index, 2);
        assert_eq!(frame.source_rect.y, 0.0);
    }
}
