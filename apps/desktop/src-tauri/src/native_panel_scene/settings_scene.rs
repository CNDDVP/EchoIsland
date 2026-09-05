use serde::Serialize;

use crate::{
    native_panel_core::{
        PanelHitAction, PanelSettingsState, effective_island_width_preset_for_display,
        island_width_preset_label, settings_row_action,
    },
    native_panel_scene::PanelDisplayOptionState,
    updater_service::{AppUpdatePhase, AppUpdateStatus},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSurfaceScene {
    pub(crate) title: String,
    pub(crate) version_text: String,
    pub(crate) rows: Vec<SettingsSurfaceRowScene>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsSurfaceRowScene {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) control_kind: SettingsSurfaceControlKind,
    pub(crate) value_text: String,
    pub(crate) checked: Option<bool>,
    pub(crate) enabled: bool,
    pub(crate) action_key: String,
    pub(crate) update_phase: Option<String>,
    pub(crate) can_install: bool,
    pub(crate) can_open_release_page: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SettingsSurfaceControlKind {
    Toggle,
    Action,
}

pub(crate) fn build_settings_surface_scene(
    display_options: &[PanelDisplayOptionState],
    settings: PanelSettingsState,
    app_version: &str,
    update_status: &AppUpdateStatus,
) -> SettingsSurfaceScene {
    let selected_display_position = display_options
        .iter()
        .position(|display| display.index == settings.selected_display_index)
        .or_else(|| {
            (settings.selected_display_index < display_options.len())
                .then_some(settings.selected_display_index)
        })
        .unwrap_or(0);
    let selected_display_label = if display_options.is_empty() {
        "0/0".to_string()
    } else {
        format!(
            "{}/{}",
            selected_display_position + 1,
            display_options.len()
        )
    };
    let selected_display_supports_wide = display_options
        .iter()
        .find(|display| display.index == settings.selected_display_index)
        .or_else(|| display_options.get(settings.selected_display_index))
        .or_else(|| display_options.first())
        .is_none_or(|display| display.supports_wide_island);
    let effective_width_preset = effective_island_width_preset_for_display(
        settings.island_width_preset,
        selected_display_supports_wide,
    );
    SettingsSurfaceScene {
        title: echoisland_i18n::t("app.settings").to_string(),
        version_text: format!("EchoIsland v{app_version}"),
        rows: vec![
            SettingsSurfaceRowScene {
                id: "island_display".to_string(),
                label: echoisland_i18n::t("settings.display").to_string(),
                control_kind: SettingsSurfaceControlKind::Action,
                value_text: selected_display_label,
                checked: None,
                enabled: !display_options.is_empty(),
                action_key: settings_action_key(0),
                update_phase: None,
                can_install: false,
                can_open_release_page: false,
            },
            SettingsSurfaceRowScene {
                id: "island_width".to_string(),
                label: echoisland_i18n::t("settings.width").to_string(),
                control_kind: SettingsSurfaceControlKind::Action,
                value_text: island_width_preset_label(effective_width_preset).to_string(),
                checked: None,
                enabled: true,
                action_key: settings_action_key(1),
                update_phase: None,
                can_install: false,
                can_open_release_page: false,
            },
            SettingsSurfaceRowScene {
                id: "completion_sound".to_string(),
                label: echoisland_i18n::t("settings.sound").to_string(),
                control_kind: SettingsSurfaceControlKind::Toggle,
                value_text: if settings.completion_sound_enabled {
                    echoisland_i18n::t("settings.on").to_string()
                } else {
                    echoisland_i18n::t("settings.off").to_string()
                },
                checked: Some(settings.completion_sound_enabled),
                enabled: true,
                action_key: settings_action_key(2),
                update_phase: None,
                can_install: false,
                can_open_release_page: false,
            },
            SettingsSurfaceRowScene {
                id: "mascot".to_string(),
                label: echoisland_i18n::t("settings.mascot").to_string(),
                control_kind: SettingsSurfaceControlKind::Toggle,
                value_text: if settings.mascot_enabled {
                    echoisland_i18n::t("settings.on").to_string()
                } else {
                    echoisland_i18n::t("settings.off").to_string()
                },
                checked: Some(settings.mascot_enabled),
                enabled: true,
                action_key: settings_action_key(3),
                update_phase: None,
                can_install: false,
                can_open_release_page: false,
            },
            SettingsSurfaceRowScene {
                id: "update".to_string(),
                label: update_status.label.clone(),
                control_kind: SettingsSurfaceControlKind::Action,
                value_text: update_status.value_text.clone(),
                checked: None,
                enabled: !matches!(
                    update_status.phase,
                    AppUpdatePhase::Checking
                        | AppUpdatePhase::Downloading
                        | AppUpdatePhase::Installing
                        | AppUpdatePhase::Installed
                ),
                action_key: settings_action_key(4),
                update_phase: Some(update_phase_key(update_status.phase).to_string()),
                can_install: update_status.can_install,
                can_open_release_page: update_status.can_open_release_page,
            },
        ],
    }
}

pub(crate) fn settings_surface_row_action(index: usize) -> Option<PanelHitAction> {
    settings_row_action(index)
}

fn settings_action_key(index: usize) -> String {
    match settings_row_action(index) {
        Some(PanelHitAction::CycleDisplay) => "cycle_display",
        Some(PanelHitAction::CycleIslandWidth) => "cycle_island_width",
        Some(PanelHitAction::ToggleCompletionSound) => "toggle_completion_sound",
        Some(PanelHitAction::ToggleMascot) => "toggle_mascot",
        Some(PanelHitAction::OpenSettingsLocation) => "open_settings_location",
        Some(PanelHitAction::OpenReleasePage) => "open_release_page",
        Some(PanelHitAction::FocusSession) => "focus_session",
        None => "unknown",
    }
    .to_string()
}

fn update_phase_key(phase: AppUpdatePhase) -> &'static str {
    match phase {
        AppUpdatePhase::Idle => "idle",
        AppUpdatePhase::Checking => "checking",
        AppUpdatePhase::UpToDate => "up_to_date",
        AppUpdatePhase::Available => "available",
        AppUpdatePhase::Downloading => "downloading",
        AppUpdatePhase::Installing => "installing",
        AppUpdatePhase::Installed => "installed",
        AppUpdatePhase::Failed => "failed",
        AppUpdatePhase::UnsupportedPortable => "unsupported_portable",
    }
}
