#![allow(unused_imports)]

use super::super::*;
use super::common::*;
use chrono::Utc;
use echoisland_runtime::{
    PendingPermissionView, PendingQuestionView, RuntimeSnapshot, SessionSnapshotView,
};
use std::time::{Duration, Instant};

#[test]
fn lightweight_refresh_plan_allows_needed_channels_when_idle() {
    let plan = resolve_native_panel_lightweight_refresh_plan(NativePanelLightweightRefreshInput {
        transitioning: false,
        animation_active: false,
        active_count_marquee_needs_refresh: true,
        mascot_animation_needs_refresh: true,
    });

    assert!(plan.active_count_marquee.refresh_allowed);
    assert!(!plan.active_count_marquee.reset_timer);
    assert!(plan.mascot_animation.refresh_allowed);
    assert!(!plan.mascot_animation.reset_timer);
}

#[test]
fn lightweight_refresh_plan_resets_channels_that_do_not_need_refresh() {
    let plan = resolve_native_panel_lightweight_refresh_plan(NativePanelLightweightRefreshInput {
        transitioning: false,
        animation_active: false,
        active_count_marquee_needs_refresh: false,
        mascot_animation_needs_refresh: false,
    });

    assert!(!plan.active_count_marquee.refresh_allowed);
    assert!(plan.active_count_marquee.reset_timer);
    assert!(!plan.mascot_animation.refresh_allowed);
    assert!(plan.mascot_animation.reset_timer);
}

#[test]
fn lightweight_refresh_plan_suspends_channels_during_panel_animation() {
    let transitioning =
        resolve_native_panel_lightweight_refresh_plan(NativePanelLightweightRefreshInput {
            transitioning: true,
            animation_active: false,
            active_count_marquee_needs_refresh: true,
            mascot_animation_needs_refresh: true,
        });
    let scheduled_animation =
        resolve_native_panel_lightweight_refresh_plan(NativePanelLightweightRefreshInput {
            transitioning: false,
            animation_active: true,
            active_count_marquee_needs_refresh: true,
            mascot_animation_needs_refresh: true,
        });

    assert!(!transitioning.active_count_marquee.refresh_allowed);
    assert!(transitioning.active_count_marquee.reset_timer);
    assert!(!transitioning.mascot_animation.refresh_allowed);
    assert!(transitioning.mascot_animation.reset_timer);
    assert_eq!(scheduled_animation, transitioning);
}

#[test]
fn panel_render_layer_style_state_preserves_render_flags() {
    let state = resolve_panel_render_layer_style_state(PanelRenderLayerStyleInput {
        shell_visible: true,
        separator_visibility: 0.42,
        shared_visible: false,
        bar_progress: 0.7,
        height_progress: 0.8,
        chrome_transition_progress: 0.7,
        shoulder_progress: 0.25,
        headline_emphasized: true,
        edge_actions_visible: true,
    });

    assert_eq!(
        state,
        PanelRenderLayerStyleState {
            shell_visible: true,
            separator_visibility: 0.42,
            shared_visible: false,
            bar_progress: 0.7,
            height_progress: 0.8,
            chrome_transition_progress: 0.7,
            shoulder_progress: 0.25,
            headline_emphasized: true,
            edge_actions_visible: true,
        }
    );
}

#[test]
fn shared_body_height_decision_ignores_sub_threshold_updates() {
    let decision = resolve_shared_body_height_decision(SharedBodyHeightDecisionInput {
        current_height: Some(120.0),
        requested_height: 120.4,
        has_snapshot: true,
        update_threshold: 1.0,
    });

    assert_eq!(
        decision,
        SharedBodyHeightDecision {
            next_height: 120.4,
            should_update: false,
            should_rerender: false,
        }
    );
}

#[test]
fn shared_body_height_decision_clamps_and_rerenders_when_snapshot_exists() {
    let decision = resolve_shared_body_height_decision(SharedBodyHeightDecisionInput {
        current_height: Some(12.0),
        requested_height: -4.0,
        has_snapshot: true,
        update_threshold: 1.0,
    });

    assert_eq!(
        decision,
        SharedBodyHeightDecision {
            next_height: 0.0,
            should_update: true,
            should_rerender: true,
        }
    );
}

#[test]
fn status_queue_sorting_keeps_approvals_first_and_completions_after() {
    let now = Instant::now();
    let earlier = Utc::now() - chrono::Duration::seconds(10);
    let middle = Utc::now() - chrono::Duration::seconds(5);
    let later = Utc::now();
    let mut items = [
        StatusQueueItem {
            key: "completion:session-2".to_string(),
            session_id: "session-2".to_string(),
            sort_time: later,
            expires_at: now,
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Completion(session("Idle")),
        },
        StatusQueueItem {
            key: "approval:request-2".to_string(),
            session_id: "session-2".to_string(),
            sort_time: later,
            expires_at: now,
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Approval(pending_permission("request-2", "session-2")),
        },
        StatusQueueItem {
            key: "question:question-1".to_string(),
            session_id: "session-3".to_string(),
            sort_time: middle,
            expires_at: now,
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Question(pending_question("question-1", "session-3")),
        },
        StatusQueueItem {
            key: "approval:request-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: earlier,
            expires_at: now,
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Approval(pending_permission("request-1", "session-1")),
        },
    ];

    items.sort_by(compare_status_queue_items);

    assert_eq!(items[0].key, "approval:request-1");
    assert_eq!(items[1].key, "question:question-1");
    assert_eq!(items[2].key, "approval:request-2");
    assert!(matches!(
        items[3].payload,
        StatusQueuePayload::Completion(_)
    ));
}

#[test]
fn surface_switch_card_progress_starts_above_zero_for_continuity() {
    assert_eq!(
        surface_switch_card_progress(0, 220),
        PANEL_SURFACE_SWITCH_INITIAL_CARD_PROGRESS
    );
}

#[test]
fn open_transition_expands_width_before_dropping_downward() {
    let width_expanding = resolve_open_transition_frame(
        PANEL_MORPH_DELAY_MS + (PANEL_MORPH_MS / 2),
        164.0,
        164.0,
        220,
    );
    assert!(width_expanding.bar_progress > 0.0);
    assert!(width_expanding.bar_progress < 1.0);
    assert_eq!(width_expanding.height_progress, 0.0);
    assert_eq!(width_expanding.drop_progress, 0.0);

    let height_growing = resolve_open_transition_frame(
        PANEL_MORPH_DELAY_MS + PANEL_MORPH_MS + (PANEL_HEIGHT_MS / 2),
        164.0,
        164.0,
        220,
    );
    assert_eq!(height_growing.bar_progress, 1.0);
    assert!(height_growing.height_progress > 0.0);
    assert!(height_growing.drop_progress > 0.0);
}

#[test]
fn animation_timeline_samples_match_existing_transition_descriptors() {
    let open = PanelAnimationTimeline::open(80.0, 164.0, 3);
    assert_eq!(
        open.total_ms(),
        PANEL_OPEN_TOTAL_MS
            + card_transition_total_ms(3, PANEL_CARD_REVEAL_MS, PANEL_CARD_REVEAL_STAGGER_MS)
    );
    assert_eq!(
        open.sample(120),
        resolve_open_transition_descriptor(
            120,
            panel_transition_canvas_height(80.0, 164.0),
            164.0,
            card_transition_total_ms(3, PANEL_CARD_REVEAL_MS, PANEL_CARD_REVEAL_STAGGER_MS),
        )
    );

    let surface_switch = PanelAnimationTimeline::surface_switch(120.0, 164.0, 2);
    assert_eq!(
        surface_switch.sample(80),
        resolve_surface_switch_transition_descriptor(
            80,
            panel_transition_canvas_height(120.0, 164.0),
            120.0,
            164.0,
            card_transition_total_ms(
                2,
                PANEL_SURFACE_SWITCH_CARD_REVEAL_MS,
                PANEL_SURFACE_SWITCH_CARD_REVEAL_STAGGER_MS
            ),
        )
    );

    let close = PanelAnimationTimeline::close(164.0, 2);
    assert_eq!(
        close.total_ms(),
        card_transition_total_ms(2, PANEL_CARD_EXIT_MS, PANEL_CARD_EXIT_STAGGER_MS)
            + PANEL_CARD_EXIT_SETTLE_MS
            + PANEL_CLOSE_TOTAL_MS
    );
    assert_eq!(close.sample(0).kind, PanelAnimationKind::Close);
}

#[test]
fn animation_descriptor_clamps_transition_values_and_preserves_kind() {
    let descriptor = resolve_panel_animation_descriptor(
        PanelAnimationKind::Open,
        PanelTransitionFrame {
            canvas_height: 180.0,
            visible_height: 140.0,
            bar_progress: 1.4,
            height_progress: -0.2,
            shoulder_progress: 0.5,
            drop_progress: 2.0,
            cards_progress: -1.0,
        },
    );

    assert_eq!(descriptor.kind, PanelAnimationKind::Open);
    assert_eq!(descriptor.canvas_height, 180.0);
    assert_eq!(descriptor.visible_height, 140.0);
    assert_eq!(descriptor.width_progress, 1.0);
    assert_eq!(descriptor.height_progress, 0.0);
    assert_eq!(descriptor.shoulder_progress, 0.5);
    assert_eq!(descriptor.drop_progress, 1.0);
    assert_eq!(descriptor.cards_progress, 0.0);
}

#[test]
fn panel_cards_visibility_progress_treats_close_progress_as_exit_progress() {
    let close = resolve_panel_animation_descriptor(
        PanelAnimationKind::Close,
        PanelTransitionFrame {
            canvas_height: 180.0,
            visible_height: 140.0,
            bar_progress: 1.0,
            height_progress: 1.0,
            shoulder_progress: 1.0,
            drop_progress: 1.0,
            cards_progress: 0.25,
        },
    );
    let open = resolve_panel_animation_descriptor(
        PanelAnimationKind::Open,
        PanelTransitionFrame {
            cards_progress: 0.25,
            ..PanelTransitionFrame {
                canvas_height: 180.0,
                visible_height: 140.0,
                bar_progress: 1.0,
                height_progress: 1.0,
                shoulder_progress: 1.0,
                drop_progress: 1.0,
                cards_progress: 0.0,
            }
        },
    );

    assert_eq!(resolve_panel_cards_visibility_progress(close), 0.75);
    assert_eq!(resolve_panel_cards_visibility_progress(open), 0.25);
}
