use super::{
    WINDOWS_WM_PAINT, WindowsNativePanelDrawFrame, WindowsNativePanelHost,
    WindowsNativePanelRenderer, WindowsNativePanelRuntime,
    clear_windows_native_panel_window_messages, create_native_panel, dpi, facade, hit_region,
    host_window, native_ui_enabled, platform_loop, queue_windows_native_panel_window_message,
    runtime_entry, wait_windows_native_platform_loop_processed_at_least, window_shell,
    windows_native_platform_loop_generations,
};
use crate::{
    native_panel_core::{
        ACTIVE_COUNT_SCROLL_HOLD_MS, CompletionBadgeItem, ExpandedSurface, HoverTransition,
        PanelAnimationDescriptor, PanelAnimationKind, PanelHitAction, PanelHitTarget,
        PanelInteractionCommand, PanelPoint, PanelRect, PanelState,
    },
    native_panel_renderer::facade::{
        command::{
            NativePanelPlatformEvent, NativePanelPointerInput, NativePanelPointerInputOutcome,
            NativePanelRuntimeCommandCapability,
            dispatch_queued_native_panel_platform_events_with_handler,
        },
        descriptor::{
            NativePanelEdgeAction, NativePanelHostWindowState, NativePanelPointerRegion,
            NativePanelPointerRegionKind, NativePanelRuntimeInputDescriptor,
            NativePanelTimelineDescriptor,
        },
        host::{NativePanelHost, NativePanelRuntimeHostController, NativePanelSceneHost},
        interaction::{
            NativePanelClickStateBridge, NativePanelCoreStateBridge,
            NativePanelHostInteractionStateBridge, NativePanelPointerInputRuntimeBridge,
            NativePanelPrimaryPointerStateBridge, NativePanelQueuedPlatformEventBridge,
        },
        presentation::{
            NativePanelActionButtonsPresentation, NativePanelCardStackPresentation,
            NativePanelCompactBarPresentation, NativePanelMascotPresentation,
            NativePanelPresentationMetrics, NativePanelPresentationModel,
            NativePanelShellPresentation, NativePanelVisualDisplayMode,
            native_panel_visual_plan_input_from_presentation,
        },
        renderer::{
            NativePanelClosePresentationInput, NativePanelCloseTrigger, NativePanelRenderer,
            NativePanelRuntimeSceneMutableStateBridge, NativePanelRuntimeSceneStateBridge,
            NativePanelSceneRuntimeBridge, NativePanelStatusClosePreservationInput,
            cache_render_command_bundle_for_state_bridge_with_input,
            resolve_current_native_panel_render_command_bundle_for_state_bridge_with_input,
            resolve_native_panel_animation_plan, resolve_native_panel_close_presentation_plan,
            resolve_native_panel_status_close_preservation_plan,
        },
        runtime::sync_runtime_scene_bundle_from_input_descriptor,
        shell::{
            NativePanelHostShellLifecycle, NativePanelHostShellRuntimePump,
            NativePanelPlatformWindowMessagePump,
        },
        testing::{
            test_pending_permission, test_pending_question, test_runtime_snapshot_with_counts,
            test_session_snapshot,
        },
        transition::NativePanelTransitionRequest,
        visual::{
            NativePanelVisualPrimitive, NativePanelVisualTextWeight,
            resolve_native_panel_visual_plan,
        },
    },
    native_panel_scene::{
        PanelRuntimeRenderState, PanelRuntimeSceneBundle, PanelSceneBuildInput, SceneCard,
        SceneMascotPose, build_panel_scene,
    },
};
use chrono::Utc;
use echoisland_runtime::{RuntimeSnapshot, SessionSnapshotView};
use std::{
    sync::{Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant},
};

fn window_message_queue_test_guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn sync_test_pointer_regions(
    runtime: &mut super::WindowsNativePanelRuntime,
    regions: Vec<NativePanelPointerRegion>,
) {
    runtime.host.renderer.last_pointer_regions = regions;
    runtime
        .host
        .present_renderer_state()
        .expect("present test pointer regions");
}

fn snapshot() -> RuntimeSnapshot {
    test_runtime_snapshot_with_counts("idle", "codex", 1, 1)
}

fn runtime_input_descriptor() -> NativePanelRuntimeInputDescriptor {
    NativePanelRuntimeInputDescriptor {
        scene_input: PanelSceneBuildInput::default(),
        screen_frame: Some(PanelRect {
            x: 0.0,
            y: 0.0,
            width: 1440.0,
            height: 900.0,
        }),
    }
}

fn test_runtime_scene_bundle(
    panel_state: &mut PanelState,
    raw_snapshot: &RuntimeSnapshot,
    input: &PanelSceneBuildInput,
) -> PanelRuntimeSceneBundle {
    sync_runtime_scene_bundle_from_input_descriptor(
        panel_state,
        raw_snapshot,
        &NativePanelRuntimeInputDescriptor {
            scene_input: input.clone(),
            screen_frame: None,
        },
        Utc::now(),
    )
    .bundle
}

fn shell_draw_frame(
    pointer_regions: Vec<NativePanelPointerRegion>,
    expanded_cards_visible: bool,
) -> WindowsNativePanelDrawFrame {
    let panel_frame = PanelRect {
        x: 100.0,
        y: 50.0,
        width: 320.0,
        height: 120.0,
    };
    WindowsNativePanelDrawFrame {
        window_state: NativePanelHostWindowState {
            frame: Some(panel_frame),
            visible: true,
            preferred_display_index: 0,
        },
        pointer_regions,
        presentation_model: expanded_cards_visible.then(|| NativePanelPresentationModel {
            panel_frame,
            content_frame: PanelRect {
                x: 110.0,
                y: 90.0,
                width: 300.0,
                height: 70.0,
            },
            shell: NativePanelShellPresentation {
                surface: ExpandedSurface::Default,
                frame: PanelRect {
                    x: 100.0,
                    y: 70.0,
                    width: 320.0,
                    height: 100.0,
                },
                visible: true,
                separator_visibility: 1.0,
                shared_visible: true,
                chrome_transition_progress: 1.0,
            },
            compact_bar: NativePanelCompactBarPresentation {
                frame: PanelRect {
                    x: 110.0,
                    y: 60.0,
                    width: 300.0,
                    height: 24.0,
                },
                left_shoulder_frame: PanelRect {
                    x: 104.0,
                    y: 78.0,
                    width: 6.0,
                    height: 6.0,
                },
                right_shoulder_frame: PanelRect {
                    x: 410.0,
                    y: 78.0,
                    width: 6.0,
                    height: 6.0,
                },
                shoulder_progress: 0.0,
                headline: crate::native_panel_scene::SceneText {
                    text: "Approval waiting".to_string(),
                    emphasized: false,
                },
                active_count: "1".to_string(),
                total_count: "1".to_string(),
                completion_count: 0,
                headline_emphasized: false,
                actions_visible: false,
            },
            card_stack: NativePanelCardStackPresentation {
                frame: PanelRect {
                    x: 110.0,
                    y: 90.0,
                    width: 300.0,
                    height: 70.0,
                },
                surface: ExpandedSurface::Default,
                cards: Vec::new(),
                content_height: 70.0,
                body_height: 70.0,
                visible: true,
            },
            mascot: NativePanelMascotPresentation {
                pose: SceneMascotPose::Idle,
                debug_mode_enabled: false,
            },
            glow: None,
            action_buttons: NativePanelActionButtonsPresentation {
                visible: false,
                buttons: Vec::new(),
            },
            metrics: NativePanelPresentationMetrics {
                expanded_content_height: 70.0,
                expanded_body_height: 70.0,
            },
        }),
    }
}

fn pending_permission_snapshot(session_id: &str) -> RuntimeSnapshot {
    pending_permission_snapshot_with_request("req-1", session_id)
}

fn pending_permission_snapshot_with_request(request_id: &str, session_id: &str) -> RuntimeSnapshot {
    let pending = test_pending_permission("claude", request_id, session_id);
    let mut snapshot = snapshot();
    snapshot.pending_permission_count = 1;
    snapshot.pending_permission = Some(pending.clone());
    snapshot.pending_permissions = vec![pending];
    snapshot
}

fn pending_question_snapshot_with_request(request_id: &str, session_id: &str) -> RuntimeSnapshot {
    let pending = test_pending_question("claude", request_id, session_id);
    let mut snapshot = snapshot();
    snapshot.pending_question_count = 1;
    snapshot.pending_question = Some(pending.clone());
    snapshot.pending_questions = vec![pending];
    snapshot
}

fn session_snapshot_view(session_id: &str) -> SessionSnapshotView {
    let mut session = test_session_snapshot("codex", session_id, "thinking");
    session.project_name = Some("Blender Addon".to_string());
    session.model = Some("gpt-5.5".to_string());
    session.last_user_prompt = Some("Review the addon panel layout".to_string());
    session.last_assistant_message = Some("Checking the current implementation".to_string());
    session
}

fn sessions_snapshot(count: usize) -> RuntimeSnapshot {
    let mut snapshot = snapshot();
    snapshot.status = "active".to_string();
    snapshot.active_session_count = count;
    snapshot.total_session_count = count;
    snapshot.sessions = (0..count)
        .map(|index| session_snapshot_view(&format!("session-{}", index + 1)))
        .collect();
    snapshot
}

#[derive(Default)]
struct RecordingEventHandler {
    handled: Vec<NativePanelPlatformEvent>,
}

impl NativePanelRuntimeCommandCapability for RecordingEventHandler {
    type Error = String;

    fn focus_session(&mut self, session_id: String) -> Result<(), Self::Error> {
        self.handled
            .push(NativePanelPlatformEvent::FocusSession(session_id));
        Ok(())
    }

    fn toggle_settings_surface(&mut self) -> Result<(), Self::Error> {
        self.handled
            .push(NativePanelPlatformEvent::ToggleSettingsSurface);
        Ok(())
    }

    fn quit_application(&mut self) -> Result<(), Self::Error> {
        self.handled.push(NativePanelPlatformEvent::QuitApplication);
        Ok(())
    }

    fn cycle_display(&mut self) -> Result<(), Self::Error> {
        self.handled.push(NativePanelPlatformEvent::CycleDisplay);
        Ok(())
    }

    fn cycle_island_width(&mut self) -> Result<(), Self::Error> {
        self.handled
            .push(NativePanelPlatformEvent::CycleIslandWidth);
        Ok(())
    }

    fn toggle_completion_sound(&mut self) -> Result<(), Self::Error> {
        self.handled
            .push(NativePanelPlatformEvent::ToggleCompletionSound);
        Ok(())
    }

    fn toggle_mascot(&mut self) -> Result<(), Self::Error> {
        self.handled.push(NativePanelPlatformEvent::ToggleMascot);
        Ok(())
    }

    fn debug_mode_trigger(&mut self) -> Result<(), Self::Error> {
        self.handled
            .push(NativePanelPlatformEvent::DebugModeTrigger);
        Ok(())
    }

    fn open_settings_location(&mut self) -> Result<(), Self::Error> {
        self.handled
            .push(NativePanelPlatformEvent::OpenSettingsLocation);
        Ok(())
    }

    fn open_release_page(&mut self) -> Result<(), Self::Error> {
        self.handled.push(NativePanelPlatformEvent::OpenReleasePage);
        Ok(())
    }
}

mod platform_loop_tests;
mod renderer_tests;
mod runtime_tests;
mod status_close_tests;
mod window_shell_tests;
