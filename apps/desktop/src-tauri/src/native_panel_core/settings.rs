use serde::{Deserialize, Serialize};

use super::{
    CompletionReminderEvent, DEFAULT_COMPACT_PILL_WIDTH, DEFAULT_EXPANDED_PILL_WIDTH,
    DEFAULT_PANEL_CANVAS_WIDTH, ExpandedSurface, PanelHitAction, PanelRect, PanelState,
    mark_completion_reminders_viewed,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PanelIslandWidthPreset {
    Compact,
    #[default]
    Standard,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PanelIslandWidthSpec {
    pub(crate) compact_width: f64,
    pub(crate) expanded_width: f64,
    pub(crate) canvas_width: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PanelSettingsState {
    pub(crate) selected_display_index: usize,
    pub(crate) island_width_preset: PanelIslandWidthPreset,
    pub(crate) completion_sound_enabled: bool,
    pub(crate) mascot_enabled: bool,
    pub(crate) debug_mode_enabled: bool,
}

impl Default for PanelSettingsState {
    fn default() -> Self {
        Self {
            selected_display_index: 0,
            island_width_preset: PanelIslandWidthPreset::Standard,
            completion_sound_enabled: true,
            mascot_enabled: true,
            debug_mode_enabled: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PanelDisplayGeometry {
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
}

pub(crate) const SETTINGS_ROW_ACTIONS: [PanelHitAction; 5] = [
    PanelHitAction::CycleDisplay,
    PanelHitAction::CycleIslandWidth,
    PanelHitAction::ToggleCompletionSound,
    PanelHitAction::ToggleMascot,
    PanelHitAction::OpenReleasePage,
];

pub(crate) const SETTINGS_CARD_SIDE_INSET: f64 = 14.0;
pub(crate) const SETTINGS_ROWS_TOP_INSET: f64 = 46.0;
pub(crate) const SETTINGS_ROW_HEIGHT: f64 = 30.0;
pub(crate) const SETTINGS_ROW_GAP: f64 = 8.0;

pub(crate) fn panel_display_key(geometry: PanelDisplayGeometry) -> String {
    format!(
        "Display|{}|{}|{}|{}",
        geometry.x, geometry.y, geometry.width, geometry.height
    )
}

pub(crate) fn resolve_preferred_panel_display_index(
    display_keys: &[String],
    preferred_key: Option<&str>,
    preferred_index: usize,
    fallback_index: Option<usize>,
) -> Option<usize> {
    if display_keys.is_empty() {
        return None;
    }

    if let Some(index) = preferred_key.and_then(|key| {
        display_keys
            .iter()
            .position(|display_key| display_key == key)
    }) {
        return Some(index);
    }

    if preferred_index < display_keys.len() {
        return Some(preferred_index);
    }

    fallback_index
        .filter(|index| *index < display_keys.len())
        .or(Some(0))
}

pub(crate) fn settings_row_action(index: usize) -> Option<PanelHitAction> {
    SETTINGS_ROW_ACTIONS.get(index).copied()
}

pub(crate) fn next_island_width_preset(preset: PanelIslandWidthPreset) -> PanelIslandWidthPreset {
    match preset {
        PanelIslandWidthPreset::Compact => PanelIslandWidthPreset::Standard,
        PanelIslandWidthPreset::Standard => PanelIslandWidthPreset::Wide,
        PanelIslandWidthPreset::Wide => PanelIslandWidthPreset::Compact,
    }
}

pub(crate) fn effective_island_width_preset_for_display(
    preset: PanelIslandWidthPreset,
    supports_wide: bool,
) -> PanelIslandWidthPreset {
    if supports_wide || preset != PanelIslandWidthPreset::Wide {
        preset
    } else {
        PanelIslandWidthPreset::Standard
    }
}

pub(crate) fn next_island_width_preset_for_display(
    preset: PanelIslandWidthPreset,
    supports_wide: bool,
) -> PanelIslandWidthPreset {
    let current = effective_island_width_preset_for_display(preset, supports_wide);
    if supports_wide {
        return next_island_width_preset(current);
    }
    match current {
        PanelIslandWidthPreset::Compact => PanelIslandWidthPreset::Standard,
        PanelIslandWidthPreset::Standard | PanelIslandWidthPreset::Wide => {
            PanelIslandWidthPreset::Compact
        }
    }
}

pub(crate) fn island_width_preset_label(preset: PanelIslandWidthPreset) -> &'static str {
    match preset {
        PanelIslandWidthPreset::Compact => echoisland_i18n::t("settings.width.compact"),
        PanelIslandWidthPreset::Standard => echoisland_i18n::t("settings.width.standard"),
        PanelIslandWidthPreset::Wide => echoisland_i18n::t("settings.width.wide"),
    }
}

pub(crate) fn island_width_spec(preset: PanelIslandWidthPreset) -> PanelIslandWidthSpec {
    match preset {
        PanelIslandWidthPreset::Compact => PanelIslandWidthSpec {
            compact_width: 233.0,
            expanded_width: 263.0,
            canvas_width: DEFAULT_PANEL_CANVAS_WIDTH,
        },
        PanelIslandWidthPreset::Standard => PanelIslandWidthSpec {
            compact_width: DEFAULT_COMPACT_PILL_WIDTH,
            expanded_width: DEFAULT_EXPANDED_PILL_WIDTH,
            canvas_width: DEFAULT_PANEL_CANVAS_WIDTH,
        },
        PanelIslandWidthPreset::Wide => PanelIslandWidthSpec {
            compact_width: 283.0,
            expanded_width: 323.0,
            canvas_width: DEFAULT_PANEL_CANVAS_WIDTH.max(323.0 + 140.0),
        },
    }
}

pub(crate) fn settings_surface_row_frame(card_frame: PanelRect, index: usize) -> PanelRect {
    PanelRect {
        x: card_frame.x + SETTINGS_CARD_SIDE_INSET,
        y: card_frame.y + card_frame.height
            - SETTINGS_ROWS_TOP_INSET
            - SETTINGS_ROW_HEIGHT
            - ((SETTINGS_ROW_HEIGHT + SETTINGS_ROW_GAP) * index as f64),
        width: (card_frame.width - SETTINGS_CARD_SIDE_INSET * 2.0).max(0.0),
        height: SETTINGS_ROW_HEIGHT,
    }
}

pub(crate) fn toggle_settings_surface(state: &mut PanelState) -> bool {
    state.status_auto_expanded = false;
    mark_completion_reminders_viewed(state, CompletionReminderEvent::ViewedBySettings);
    let next_surface = if state.surface_mode == ExpandedSurface::Settings {
        ExpandedSurface::Default
    } else {
        ExpandedSurface::Settings
    };
    let changed = state.surface_mode != next_surface;
    state.surface_mode = next_surface;
    changed
}
