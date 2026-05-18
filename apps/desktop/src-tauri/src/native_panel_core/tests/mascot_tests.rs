#![allow(unused_imports)]

use super::super::*;
use super::common::*;
use chrono::Utc;
use echoisland_runtime::{
    PendingPermissionView, PendingQuestionView, RuntimeSnapshot, SessionSnapshotView,
};
use std::time::{Duration, Instant};

#[test]
fn compact_active_session_count_ignores_idle_sessions() {
    let mut snapshot = snapshot(2, 3);
    snapshot.sessions = vec![session("Running"), session("Idle"), session("Processing")];
    snapshot.sessions[1].session_id = "session-2".to_string();
    snapshot.sessions[2].session_id = "session-3".to_string();

    assert_eq!(compact_active_session_count(&snapshot), 2);
}

#[test]
fn mascot_base_state_preserves_priority_order() {
    let mut snapshot = snapshot(1, 1);
    snapshot.sessions = vec![session("Running")];

    assert_eq!(
        resolve_mascot_base_state(Some(&snapshot), false, false),
        PanelMascotBaseState::Running
    );

    assert_eq!(
        resolve_mascot_base_state(Some(&snapshot), false, true),
        PanelMascotBaseState::Complete
    );

    assert_eq!(
        resolve_mascot_base_state(Some(&snapshot), true, true),
        PanelMascotBaseState::MessageBubble
    );

    snapshot.pending_question_count = 1;
    assert_eq!(
        resolve_mascot_base_state(Some(&snapshot), true, true),
        PanelMascotBaseState::Question
    );

    snapshot.pending_permission_count = 1;
    assert_eq!(
        resolve_mascot_base_state(Some(&snapshot), true, true),
        PanelMascotBaseState::Approval
    );
}

#[test]
fn reminder_state_unifies_badge_glow_and_mascot_semantics() {
    let mut snapshot = snapshot(1, 1);
    snapshot.sessions = vec![session("Running")];
    let state = PanelState {
        completion_badge_items: vec![CompletionBadgeItem {
            session_id: "session-1".to_string(),
            completed_at: Utc::now(),
            last_user_prompt: None,
            last_assistant_message: Some("Done".to_string()),
        }],
        ..PanelState::default()
    };

    let reminder = resolve_panel_reminder_state(&state, Some(&snapshot));

    assert_eq!(reminder.completion_badge_count, 1);
    assert!(reminder.show_completion_glow);
    assert!(!reminder.has_status_completion);
    assert_eq!(reminder.mascot_base_state, PanelMascotBaseState::Complete);
}

#[test]
fn mascot_visual_frame_animates_idle_breath_and_blink() {
    let idle = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Idle,
        elapsed_ms: 0,
    });
    let breathing = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Idle,
        elapsed_ms: 900,
    });
    let blinking = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Idle,
        elapsed_ms: 4535,
    });

    assert_eq!(idle.offset_y, 0.0);
    assert!(breathing.scale_x > idle.scale_x);
    assert!(blinking.eye_open < idle.eye_open);
}

#[test]
fn mascot_visual_frame_gives_message_bubble_a_visible_bob() {
    let start = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::MessageBubble,
        elapsed_ms: 0,
    });
    let bobbing = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::MessageBubble,
        elapsed_ms: 480,
    });

    assert!(bobbing.offset_y > start.offset_y);
    assert!(bobbing.scale_x >= 1.0);
    assert_eq!(bobbing.eye_open, 1.0);
}

#[test]
fn mascot_visual_frame_matches_mac_motion_phase_and_horizontal_sway() {
    let running = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Running,
        elapsed_ms: 250,
    });
    let message = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::MessageBubble,
        elapsed_ms: 0,
    });

    assert!(running.offset_x.abs() > 0.01);
    assert!((message.offset_y - 0.8).abs() < 0.001);
}

#[test]
fn mascot_visual_frame_uses_mac_style_state_specific_blink_floor() {
    let elapsed_ms = 4535;
    let idle = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Idle,
        elapsed_ms,
    });
    let running = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Running,
        elapsed_ms,
    });
    let approval = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Approval,
        elapsed_ms,
    });
    let question = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Question,
        elapsed_ms,
    });
    let complete = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Complete,
        elapsed_ms,
    });

    assert!(idle.eye_open < 0.2);
    assert!(running.eye_open >= 0.72);
    assert!(approval.eye_open >= 0.34);
    assert!(question.eye_open >= 0.48);
    assert!(complete.eye_open >= 0.72);
}

#[test]
fn mascot_visual_frame_supports_mac_sleepy_and_wake_angry_states() {
    let sleepy_start = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Sleepy,
        elapsed_ms: 0,
    });
    let sleepy_nod = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Sleepy,
        elapsed_ms: 4550,
    });
    let wake_start = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::WakeAngry,
        elapsed_ms: 0,
    });
    let wake_faded = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::WakeAngry,
        elapsed_ms: 900,
    });

    assert!(sleepy_start.scale_y < 0.96);
    assert!(sleepy_start.eye_open < 1.0);
    assert!(sleepy_nod.offset_y < 0.0);
    assert!(wake_start.offset_x.abs() < 0.001);
    assert!(wake_start.scale_x > 1.04);
    assert!(wake_faded.scale_x < wake_start.scale_x);
}

#[test]
fn mascot_visual_frame_transition_smoothsteps_motion_fields() {
    let start = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Idle,
        elapsed_ms: 0,
    });
    let target = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Running,
        elapsed_ms: 0,
    });
    let halfway = resolve_mascot_visual_frame_transition(MascotVisualFrameTransitionInput {
        start,
        target,
        elapsed_ms: 120,
        duration_ms: 240,
    });
    let done = resolve_mascot_visual_frame_transition(MascotVisualFrameTransitionInput {
        start,
        target,
        elapsed_ms: 240,
        duration_ms: 240,
    });

    assert!((halfway.scale_x - ((start.scale_x + target.scale_x) / 2.0)).abs() < 0.001);
    assert!(halfway.shadow_opacity > start.shadow_opacity);
    assert!(halfway.shadow_opacity < target.shadow_opacity);
    assert_eq!(done, target);
}

#[test]
fn mascot_visual_frame_transition_zero_duration_jumps_to_target() {
    let start = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Idle,
        elapsed_ms: 0,
    });
    let target = resolve_mascot_visual_frame(MascotVisualFrameInput {
        state: PanelMascotBaseState::Complete,
        elapsed_ms: 300,
    });

    assert_eq!(
        resolve_mascot_visual_frame_transition(MascotVisualFrameTransitionInput {
            start,
            target,
            elapsed_ms: 0,
            duration_ms: 0,
        }),
        target
    );
}
