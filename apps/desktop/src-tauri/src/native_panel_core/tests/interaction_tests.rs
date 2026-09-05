#![allow(unused_imports)]

use super::super::*;
use super::common::*;
use chrono::Utc;
use echoisland_runtime::{
    PendingPermissionView, PendingQuestionView, RuntimeSnapshot, SessionSnapshotView,
};
use std::time::{Duration, Instant};

#[test]
fn settings_surface_toggle_cycles_between_default_and_settings() {
    let mut state = PanelState::default();

    assert!(toggle_settings_surface(&mut state));
    assert_eq!(state.surface_mode, ExpandedSurface::Settings);

    assert!(toggle_settings_surface(&mut state));
    assert_eq!(state.surface_mode, ExpandedSurface::Default);
}

#[test]
fn settings_surface_toggle_marks_completion_badge_as_viewed() {
    let mut state = PanelState {
        completion_badge_items: vec![CompletionBadgeItem {
            session_id: "session-1".to_string(),
            completed_at: Utc::now(),
            last_user_prompt: Some("ship it".to_string()),
            last_assistant_message: Some("已完成".to_string()),
        }],
        ..PanelState::default()
    };

    assert!(toggle_settings_surface(&mut state));

    assert!(state.completion_badge_items.is_empty());
}

#[test]
fn settings_row_actions_preserve_semantics() {
    assert_eq!(settings_row_action(0), Some(PanelHitAction::CycleDisplay));
    assert_eq!(
        settings_row_action(1),
        Some(PanelHitAction::CycleIslandWidth)
    );
    assert_eq!(
        settings_row_action(2),
        Some(PanelHitAction::ToggleCompletionSound)
    );
    assert_eq!(settings_row_action(3), Some(PanelHitAction::ToggleMascot));
    assert_eq!(
        settings_row_action(4),
        Some(PanelHitAction::OpenReleasePage)
    );
    assert_eq!(settings_row_action(5), None);
}

#[test]
fn island_width_presets_cycle_and_preserve_standard_defaults() {
    assert_eq!(
        next_island_width_preset(PanelIslandWidthPreset::Compact),
        PanelIslandWidthPreset::Standard
    );
    assert_eq!(
        next_island_width_preset(PanelIslandWidthPreset::Standard),
        PanelIslandWidthPreset::Wide
    );
    assert_eq!(
        next_island_width_preset(PanelIslandWidthPreset::Wide),
        PanelIslandWidthPreset::Compact
    );
    assert_eq!(
        next_island_width_preset_for_display(PanelIslandWidthPreset::Standard, false),
        PanelIslandWidthPreset::Compact
    );
    assert_eq!(
        next_island_width_preset_for_display(PanelIslandWidthPreset::Compact, false),
        PanelIslandWidthPreset::Standard
    );
    assert_eq!(
        effective_island_width_preset_for_display(PanelIslandWidthPreset::Wide, false),
        PanelIslandWidthPreset::Standard
    );

    let standard = island_width_spec(PanelIslandWidthPreset::Standard);
    assert_eq!(standard.compact_width, DEFAULT_COMPACT_PILL_WIDTH);
    assert_eq!(standard.expanded_width, DEFAULT_EXPANDED_PILL_WIDTH);
    assert_eq!(standard.canvas_width, DEFAULT_PANEL_CANVAS_WIDTH);
    assert!(standard.expanded_width > standard.compact_width);
}

#[test]
fn island_width_specs_are_shared_labels_and_safe_canvas_widths() {
    assert_eq!(
        island_width_preset_label(PanelIslandWidthPreset::Compact),
        "紧凑"
    );
    assert_eq!(
        island_width_preset_label(PanelIslandWidthPreset::Standard),
        "标准"
    );
    assert_eq!(
        island_width_preset_label(PanelIslandWidthPreset::Wide),
        "宽"
    );

    for preset in [
        PanelIslandWidthPreset::Compact,
        PanelIslandWidthPreset::Standard,
        PanelIslandWidthPreset::Wide,
    ] {
        let spec = island_width_spec(preset);
        assert!(spec.expanded_width > spec.compact_width);
        assert!(spec.canvas_width >= spec.expanded_width + 24.0);
    }
}

#[test]
fn preferred_panel_display_uses_key_before_index() {
    let displays = vec![
        panel_display_key(PanelDisplayGeometry {
            x: 0,
            y: 0,
            width: 1440,
            height: 900,
        }),
        panel_display_key(PanelDisplayGeometry {
            x: 1440,
            y: 0,
            width: 1512,
            height: 982,
        }),
    ];

    assert_eq!(
        resolve_preferred_panel_display_index(&displays, Some(&displays[1]), 0, Some(0)),
        Some(1)
    );
}

#[test]
fn preferred_panel_display_falls_back_to_index_then_main_then_first() {
    let displays = vec![
        "Display|0|0|1440|900".to_string(),
        "Display|1440|0|1512|982".to_string(),
    ];

    assert_eq!(
        resolve_preferred_panel_display_index(&displays, Some("missing"), 1, Some(0)),
        Some(1)
    );
    assert_eq!(
        resolve_preferred_panel_display_index(&displays, Some("missing"), 9, Some(1)),
        Some(1)
    );
    assert_eq!(
        resolve_preferred_panel_display_index(&displays, Some("missing"), 9, Some(9)),
        Some(0)
    );
    assert_eq!(
        resolve_preferred_panel_display_index(&[], Some("missing"), 0, Some(0)),
        None
    );
}

#[test]
fn click_action_prioritizes_settings_before_quit_and_cards() {
    let now = Instant::now();
    let resolution = resolve_panel_click_action(PanelClickInput {
        primary_click_started: true,
        expanded: true,
        transitioning: false,
        settings_button_hit: true,
        quit_button_hit: true,
        cards_visible: true,
        card_target: Some(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
        last_focus_click: None,
        now,
        focus_debounce_ms: 600,
    });

    assert_eq!(
        resolution.command,
        PanelInteractionCommand::ToggleSettingsSurface
    );
    assert_eq!(resolution.focus_click_to_record, None);
}

#[test]
fn click_action_allows_edge_actions_during_open_transition() {
    let now = Instant::now();
    let settings = resolve_panel_click_action(PanelClickInput {
        primary_click_started: true,
        expanded: true,
        transitioning: true,
        settings_button_hit: true,
        quit_button_hit: false,
        cards_visible: true,
        card_target: Some(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
        last_focus_click: None,
        now,
        focus_debounce_ms: 600,
    });
    let card = resolve_panel_click_action(PanelClickInput {
        primary_click_started: true,
        expanded: true,
        transitioning: true,
        settings_button_hit: false,
        quit_button_hit: false,
        cards_visible: true,
        card_target: Some(PanelHitTarget {
            action: PanelHitAction::FocusSession,
            value: "session-1".to_string(),
        }),
        last_focus_click: None,
        now,
        focus_debounce_ms: 600,
    });

    assert_eq!(
        settings.command,
        PanelInteractionCommand::ToggleSettingsSurface
    );
    assert_eq!(settings.focus_click_to_record, None);
    assert_eq!(card.command, PanelInteractionCommand::None);
    assert_eq!(card.focus_click_to_record, None);
}

#[test]
fn click_action_records_focus_session_and_suppresses_duplicates() {
    let now = Instant::now();
    let target = PanelHitTarget {
        action: PanelHitAction::FocusSession,
        value: "session-1".to_string(),
    };
    let first = resolve_panel_click_action(PanelClickInput {
        primary_click_started: true,
        expanded: true,
        transitioning: false,
        settings_button_hit: false,
        quit_button_hit: false,
        cards_visible: true,
        card_target: Some(target.clone()),
        last_focus_click: None,
        now,
        focus_debounce_ms: 600,
    });

    assert_eq!(
        first.command,
        PanelInteractionCommand::HitTarget(target.clone())
    );
    assert_eq!(first.focus_click_to_record, Some("session-1".to_string()));

    let duplicate = resolve_panel_click_action(PanelClickInput {
        primary_click_started: true,
        expanded: true,
        transitioning: false,
        settings_button_hit: false,
        quit_button_hit: false,
        cards_visible: true,
        card_target: Some(target),
        last_focus_click: Some(LastFocusClick {
            session_id: "session-1",
            clicked_at: now,
        }),
        now,
        focus_debounce_ms: 600,
    });

    assert_eq!(duplicate.command, PanelInteractionCommand::None);
    assert_eq!(duplicate.focus_click_to_record, None);
}

#[test]
fn hover_state_expands_after_inside_delay_and_clears_badges() {
    let now = Instant::now();
    let mut state = PanelState {
        pointer_inside_since: Some(now - Duration::from_millis(600)),
        completion_badge_items: vec![CompletionBadgeItem {
            session_id: "session-1".to_string(),
            completed_at: Utc::now(),
            last_user_prompt: None,
            last_assistant_message: Some("已完成".to_string()),
        }],
        status_auto_expanded: true,
        surface_mode: ExpandedSurface::Status,
        ..PanelState::default()
    };

    let transition = sync_hover_expansion_state(&mut state, true, now, 500);

    assert_eq!(transition, Some(HoverTransition::Expand));
    assert!(state.expanded);
    assert!(state.completion_badge_items.is_empty());
    assert!(!state.status_auto_expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Default);
}

#[test]
fn hover_state_collapses_after_outside_delay_when_not_transitioning() {
    let now = Instant::now();
    let mut state = PanelState {
        expanded: true,
        pointer_outside_since: Some(now - Duration::from_millis(600)),
        ..PanelState::default()
    };

    let transition = sync_hover_expansion_state(&mut state, false, now, 500);

    assert_eq!(transition, Some(HoverTransition::Collapse));
    assert!(!state.expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Default);
}

#[test]
fn hover_state_reopens_during_close_transition_after_inside_delay() {
    let now = Instant::now();
    let mut state = PanelState {
        expanded: false,
        transitioning: true,
        pointer_inside_since: Some(now - Duration::from_millis(600)),
        ..PanelState::default()
    };

    let transition = sync_hover_expansion_state(&mut state, true, now, 500);

    assert_eq!(transition, Some(HoverTransition::Expand));
    assert!(state.expanded);
}

#[test]
fn hover_state_recloses_during_open_transition_after_outside_delay() {
    let now = Instant::now();
    let mut state = PanelState {
        expanded: true,
        transitioning: true,
        pointer_outside_since: Some(now - Duration::from_millis(600)),
        ..PanelState::default()
    };

    let transition = sync_hover_expansion_state(&mut state, false, now, 500);

    assert_eq!(transition, Some(HoverTransition::Collapse));
    assert!(!state.expanded);
}

#[test]
fn hover_state_keeps_auto_status_surface_open_outside() {
    let now = Instant::now();
    let mut state = PanelState {
        expanded: true,
        status_auto_expanded: true,
        surface_mode: ExpandedSurface::Status,
        pointer_outside_since: Some(now - Duration::from_millis(600)),
        status_queue: vec![StatusQueueItem {
            key: "approval:request-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: Utc::now(),
            expires_at: now + Duration::from_secs(STATUS_APPROVAL_VISIBLE_SECONDS),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Approval(pending_permission("request-1", "session-1")),
        }],
        ..PanelState::default()
    };

    let transition = sync_hover_expansion_state(&mut state, false, now, 500);

    assert_eq!(transition, None);
    assert!(state.expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Status);
}

#[test]
fn status_surface_policy_switches_hover_expanded_panel_to_new_status_message() {
    let now = Instant::now();
    let mut state = PanelState {
        expanded: true,
        pointer_inside_since: Some(now - Duration::from_millis(HOVER_DELAY_MS + 100)),
        surface_mode: ExpandedSurface::Default,
        status_queue: vec![StatusQueueItem {
            key: "completion:session-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: Utc::now(),
            expires_at: now + Duration::from_secs(STATUS_COMPLETION_VISIBLE_SECONDS),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Completion(session("Idle")),
        }],
        ..PanelState::default()
    };

    let transition = sync_status_surface_policy(
        &mut state,
        StatusQueueSyncResult {
            added_approvals: 0,
            added_questions: 0,
            added_completions: 1,
        },
    );

    assert_eq!(transition.panel_transition, None);
    assert!(transition.surface_transition);
    assert!(state.expanded);
    assert!(state.status_auto_expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Status);
}
