#![allow(unused_imports)]

use super::super::*;
use super::common::*;
use chrono::Utc;
use echoisland_runtime::{
    PendingPermissionView, PendingQuestionView, RuntimeSnapshot, SessionSnapshotView,
};
use std::time::{Duration, Instant};

#[test]
fn pending_permission_card_clears_after_grace_window_expires() {
    let now = Instant::now();
    let previous = PendingPermissionCardState {
        request_id: "request-1".to_string(),
        payload: pending_permission("request-1", "session-1"),
        started_at: now - Duration::from_millis(PENDING_CARD_MIN_VISIBLE_MS),
        last_seen_at: now - Duration::from_millis(PENDING_CARD_RELEASE_GRACE_MS + 10),
        visible_until: now - Duration::from_millis(1),
    };

    assert!(resolve_pending_permission_card(None, Some(&previous), now).is_none());
}

#[test]
fn pending_card_grace_snapshot_does_not_readd_status_approval() {
    let mut state = PanelState::default();
    let live_snapshot = snapshot_with_permission("request-1", "session-1");
    let held_snapshot = sync_pending_card_visibility(&mut state, &live_snapshot);

    assert_eq!(held_snapshot.pending_permission_count, 1);
    state.last_raw_snapshot = Some(live_snapshot);

    let empty_snapshot = snapshot(0, 1);
    let held_after_resolve = sync_pending_card_visibility(&mut state, &empty_snapshot);

    assert_eq!(held_after_resolve.pending_permission_count, 1);
    assert_eq!(
        sync_status_queue(&mut state, &empty_snapshot).added_approvals,
        0
    );
    assert!(state.status_queue.is_empty());
}

#[test]
fn completion_badge_tracks_completed_session_until_new_dialogue() {
    let mut state = PanelState::default();
    let mut previous = snapshot(1, 1);
    previous.sessions = vec![session("Running")];

    let mut current = snapshot(0, 1);
    let mut completed = session("Idle");
    completed.last_assistant_message = Some("已完成".to_string());
    current.sessions = vec![completed.clone()];

    let completed_session_ids = detect_completed_sessions(&previous, &current, Utc::now());
    sync_completion_badge(&mut state, &current, &completed_session_ids);

    assert_eq!(state.completion_badge_items.len(), 1);
    assert_eq!(
        state.completion_badge_items[0].session_id,
        completed.session_id
    );

    let mut next = current.clone();
    let next_session = next.sessions.first_mut().unwrap();
    next_session.status = "Running".to_string();
    next_session.last_user_prompt = Some("continue".to_string());
    next_session.last_activity = completed.last_activity + chrono::Duration::seconds(1);

    sync_completion_badge(&mut state, &next, &[]);

    assert!(state.completion_badge_items.is_empty());
}

#[test]
fn completion_detection_allows_active_to_idle_without_assistant_message() {
    let mut previous = snapshot(1, 1);
    previous.sessions = vec![session("Running")];

    let mut current = snapshot(0, 1);
    let mut completed_without_message = session("Idle");
    completed_without_message.last_assistant_message = None;
    current.sessions = vec![completed_without_message.clone()];

    assert_eq!(
        detect_completed_sessions(&previous, &current, Utc::now()),
        vec![completed_without_message.session_id.clone()]
    );

    current.sessions[0].last_assistant_message = Some("   ".to_string());

    assert_eq!(
        detect_completed_sessions(&previous, &current, Utc::now()),
        vec![completed_without_message.session_id.clone()]
    );

    current.sessions[0].last_assistant_message = Some("已完成".to_string());

    assert_eq!(
        detect_completed_sessions(&previous, &current, Utc::now()),
        vec![completed_without_message.session_id]
    );
}

#[test]
fn completion_detection_treats_new_feishu_message_as_completed_notification() {
    let now = Utc::now();
    let previous = snapshot(0, 0);
    let mut current = snapshot(0, 1);
    let mut message = session("Idle");
    message.session_id = "feishu:oc_1".to_string();
    message.source = "feishu".to_string();
    message.last_assistant_message = Some("[feishu direct text] hello".to_string());
    message.last_activity = now;
    current.sessions = vec![message.clone()];

    assert_eq!(
        detect_completed_sessions(&previous, &current, now),
        vec![message.session_id]
    );
}

#[test]
fn completion_detection_treats_new_scanned_host_app_message_as_completed_notification() {
    let now = Utc::now();
    let previous = snapshot(0, 0);
    let mut current = snapshot(0, 1);
    let mut message = session("Idle");
    message.source = "claude".to_string();
    message.host_app = Some("claude".to_string());
    message.last_assistant_message = Some("Done from scanner".to_string());
    message.last_activity = now;
    current.sessions = vec![message.clone()];

    assert_eq!(
        detect_completed_sessions(&previous, &current, now),
        vec![message.session_id]
    );
}

#[test]
fn completion_detection_ignores_first_seen_idle_codex_app_message() {
    let now = Utc::now();
    let previous = snapshot(0, 0);
    let mut current = snapshot(0, 1);
    let mut message = session("Idle");
    message.source = "codex".to_string();
    message.host_app = Some("com.openai.codex".to_string());
    message.last_assistant_message = Some("streaming answer".to_string());
    message.last_activity = now;
    current.sessions = vec![message];

    assert!(detect_completed_sessions(&previous, &current, now).is_empty());
}

#[test]
fn completion_detection_ignores_idle_codex_app_streaming_message_updates() {
    let now = Utc::now();
    let mut previous = snapshot(0, 1);
    let mut previous_session = session("Idle");
    previous_session.host_app = Some("com.openai.codex".to_string());
    previous_session.last_assistant_message = Some("partial answer".to_string());
    previous_session.last_activity = now - chrono::Duration::seconds(1);
    previous.sessions = vec![previous_session];

    let mut current = snapshot(0, 1);
    let mut current_session = session("Idle");
    current_session.host_app = Some("com.openai.codex".to_string());
    current_session.last_assistant_message = Some("partial answer continues".to_string());
    current_session.last_activity = now;
    current.sessions = vec![current_session];

    assert!(detect_completed_sessions(&previous, &current, now).is_empty());
}

#[test]
fn feishu_meta_line_omits_missing_short_session_placeholder() {
    let mut message = session("Idle");
    message.session_id = "feishu:oc_1".to_string();
    message.source = "feishu".to_string();

    let meta = session_meta_line(&message);

    assert!(!meta.contains("------"));
    assert!(!meta.contains("#------"));
}

#[test]
fn completion_detection_does_not_treat_new_regular_idle_session_as_completion() {
    let now = Utc::now();
    let previous = snapshot(0, 0);
    let mut current = snapshot(0, 1);
    let mut idle = session("Idle");
    idle.last_assistant_message = Some("Restored".to_string());
    idle.last_activity = now;
    current.sessions = vec![idle];

    assert!(detect_completed_sessions(&previous, &current, now).is_empty());
}

#[test]
fn snapshot_sync_emits_generic_completion_reminder_for_active_to_idle_without_message() {
    let mut state = PanelState::default();
    let mut previous = snapshot(1, 1);
    previous.sessions = vec![session("Running")];
    state.last_raw_snapshot = Some(previous);

    let mut current = snapshot(0, 1);
    current.sessions = vec![session("Idle")];

    let result = sync_panel_snapshot_state(&mut state, &current, Utc::now());

    assert!(result.reminder.play_sound);
    assert!(result.reminder.show_status_card);
    assert_eq!(result.panel_transition, Some(true));
    assert_eq!(state.completion_badge_items.len(), 1);
    assert!(
        state
            .status_queue
            .iter()
            .any(|item| matches!(item.payload, StatusQueuePayload::Completion(_)))
    );
}

#[test]
fn snapshot_sync_reopens_completion_when_message_arrives_after_expired_generic_card() {
    let now = Utc::now();
    let mut previous = snapshot(0, 1);
    let mut previous_session = session("Idle");
    previous_session.last_activity = now - chrono::Duration::seconds(5);
    previous.sessions = vec![previous_session.clone()];

    let mut state = PanelState {
        last_raw_snapshot: Some(previous),
        status_queue: vec![StatusQueueItem {
            key: "completion:session-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: previous_session.last_activity,
            expires_at: Instant::now() - Duration::from_millis(1),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Completion(previous_session),
        }],
        ..PanelState::default()
    };

    let mut current = snapshot(0, 1);
    let mut completed_with_message = session("Idle");
    completed_with_message.last_activity = now;
    completed_with_message.last_assistant_message = Some("已完成".to_string());
    current.sessions = vec![completed_with_message];

    let result = sync_panel_snapshot_state(&mut state, &current, now);

    assert!(result.reminder.play_sound);
    assert!(result.reminder.show_status_card);
    assert_eq!(result.panel_transition, Some(true));
    assert_eq!(state.status_queue.len(), 1);
    assert!(
        state
            .status_queue
            .iter()
            .any(|item| matches!(item.payload, StatusQueuePayload::Completion(_)))
    );
}

#[test]
fn completion_badge_stays_unread_during_auto_status_expansion() {
    let mut state = PanelState {
        expanded: true,
        status_auto_expanded: true,
        surface_mode: ExpandedSurface::Status,
        completion_badge_items: vec![CompletionBadgeItem {
            session_id: "session-1".to_string(),
            completed_at: Utc::now(),
            last_user_prompt: Some("ship it".to_string()),
            last_assistant_message: Some("已完成".to_string()),
        }],
        ..PanelState::default()
    };
    let mut current = snapshot(0, 1);
    let mut completed = session("Idle");
    completed.last_user_prompt = Some("ship it".to_string());
    completed.last_assistant_message = Some("已完成".to_string());
    current.sessions = vec![completed];

    sync_completion_badge(&mut state, &current, &[]);

    assert_eq!(state.completion_badge_items.len(), 1);
}

#[test]
fn completion_reminder_events_distinguish_viewed_and_passive_status_card_lifecycle() {
    assert!(completion_reminder_event_clears_badge(
        CompletionReminderEvent::ViewedByManualExpansion
    ));
    assert!(completion_reminder_event_clears_badge(
        CompletionReminderEvent::ViewedBySettings
    ));
    assert!(completion_reminder_event_clears_badge(
        CompletionReminderEvent::ClearedByNewDialogue
    ));
    assert!(!completion_reminder_event_clears_badge(
        CompletionReminderEvent::Added
    ));
    assert!(!completion_reminder_event_clears_badge(
        CompletionReminderEvent::StatusCardExpired
    ));
}

#[test]
fn status_surface_transition_switches_expanded_panel_into_status_mode() {
    let mut state = PanelState {
        expanded: true,
        status_queue: vec![StatusQueueItem {
            key: "approval:request-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: Utc::now(),
            expires_at: Instant::now() + Duration::from_secs(STATUS_APPROVAL_VISIBLE_SECONDS),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Approval(pending_permission("request-1", "session-1")),
        }],
        ..PanelState::default()
    };

    let transition = sync_status_surface_policy(
        &mut state,
        StatusQueueSyncResult {
            added_approvals: 1,
            added_questions: 0,
            added_completions: 0,
        },
    );

    assert_eq!(transition.panel_transition, None);
    assert!(transition.surface_transition);
    assert!(state.status_auto_expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Status);
}

#[test]
fn status_surface_policy_queues_surface_switch_during_open_transition() {
    let mut state = PanelState {
        expanded: true,
        transitioning: true,
        status_queue: vec![StatusQueueItem {
            key: "approval:request-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: Utc::now(),
            expires_at: Instant::now() + Duration::from_secs(STATUS_APPROVAL_VISIBLE_SECONDS),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Approval(pending_permission("request-1", "session-1")),
        }],
        surface_mode: ExpandedSurface::Default,
        ..PanelState::default()
    };

    let transition = sync_status_surface_policy(
        &mut state,
        StatusQueueSyncResult {
            added_approvals: 1,
            added_questions: 0,
            added_completions: 0,
        },
    );

    assert_eq!(transition.panel_transition, None);
    assert!(transition.surface_transition);
    assert!(state.expanded);
    assert!(state.status_auto_expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Status);
}

#[test]
fn status_surface_policy_marks_new_status_for_reopen_during_close_transition() {
    let mut state = PanelState {
        expanded: false,
        transitioning: true,
        status_queue: vec![StatusQueueItem {
            key: "approval:request-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: Utc::now(),
            expires_at: Instant::now() + Duration::from_secs(STATUS_APPROVAL_VISIBLE_SECONDS),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Approval(pending_permission("request-1", "session-1")),
        }],
        surface_mode: ExpandedSurface::Default,
        ..PanelState::default()
    };

    let transition = sync_status_surface_policy(
        &mut state,
        StatusQueueSyncResult {
            added_approvals: 1,
            added_questions: 0,
            added_completions: 0,
        },
    );

    assert_eq!(transition.panel_transition, None);
    assert!(!transition.surface_transition);
    assert!(!state.expanded);
    assert!(state.status_auto_expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Status);
}

#[test]
fn pending_status_reopen_after_transition_expands_auto_status_surface_once() {
    let mut state = PanelState {
        expanded: false,
        transitioning: false,
        status_auto_expanded: true,
        surface_mode: ExpandedSurface::Status,
        status_queue: vec![StatusQueueItem {
            key: "question:question-1".to_string(),
            session_id: "session-1".to_string(),
            sort_time: Utc::now(),
            expires_at: Instant::now() + Duration::from_secs(STATUS_APPROVAL_VISIBLE_SECONDS),
            is_live: true,
            is_removing: false,
            remove_after: None,
            payload: StatusQueuePayload::Question(pending_question("question-1", "session-1")),
        }],
        ..PanelState::default()
    };

    assert!(take_pending_status_reopen_after_transition(&mut state));
    assert!(state.expanded);
    assert!(!take_pending_status_reopen_after_transition(&mut state));
}

#[test]
fn snapshot_sync_emits_message_sound_and_panel_transition_for_new_status() {
    let mut state = PanelState::default();
    let raw_snapshot = snapshot_with_permission("request-1", "session-1");

    let result = sync_panel_snapshot_state(&mut state, &raw_snapshot, Utc::now());

    assert!(result.reminder.play_sound);
    assert!(result.reminder.show_status_card);
    assert_eq!(result.panel_transition, Some(true));
    assert!(!result.surface_transition);
    assert_eq!(result.displayed_snapshot.pending_permission_count, 1);
    assert!(state.last_raw_snapshot.is_some());
    assert_eq!(state.surface_mode, ExpandedSurface::Status);
}

#[test]
fn snapshot_sync_auto_expands_status_surface_for_new_question() {
    let mut state = PanelState::default();
    let raw_snapshot = snapshot_with_question("question-1", "session-1");

    let result = sync_panel_snapshot_state(&mut state, &raw_snapshot, Utc::now());

    assert!(result.reminder.play_sound);
    assert!(result.reminder.show_status_card);
    assert_eq!(result.panel_transition, Some(true));
    assert_eq!(state.surface_mode, ExpandedSurface::Status);
    assert!(state.status_queue.iter().any(|item| {
        matches!(
            &item.payload,
            StatusQueuePayload::Question(question) if question.request_id == "question-1"
        )
    }));
}

#[test]
fn status_surface_reverts_to_default_when_queue_drains() {
    let mut state = PanelState {
        expanded: true,
        status_auto_expanded: true,
        surface_mode: ExpandedSurface::Status,
        ..PanelState::default()
    };

    let transition = sync_status_surface_policy(&mut state, StatusQueueSyncResult::default());

    assert_eq!(transition.panel_transition, Some(false));
    assert!(!transition.surface_transition);
    assert!(!state.expanded);
    assert_eq!(state.surface_mode, ExpandedSurface::Default);
    assert!(state.skip_next_close_card_exit);
}
