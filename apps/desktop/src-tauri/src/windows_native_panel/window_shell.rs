use super::{
    draw_presenter::WindowsNativePanelDrawPresenter, host_window::WindowsNativePanelDrawFrame,
};
use crate::{
    native_panel_core::{
        MASCOT_STATE_TRANSITION_SECONDS, MascotRuntimeFrameInput, MascotRuntimeState,
        MascotVisualFrame, PanelPoint, PanelRect,
    },
    native_panel_renderer::facade::{
        descriptor::{
            NativePanelHostWindowState, NativePanelInteractionPlan, NativePanelPointerInput,
            NativePanelPointerPointState, NativePanelPointerRegion,
        },
        interaction::{
            NativePanelHoverFallbackFrames, NativePanelPollingHostFacts,
            resolve_native_panel_hover_fallback_frames,
            resolve_native_panel_stable_compact_hover_frame,
        },
        presentation::{
            NativePanelPresentationModel, NativePanelVisualActionButtonInput,
            NativePanelVisualDisplayMode, NativePanelVisualPlanInput,
            native_panel_visual_display_mode_from_presentation,
            native_panel_visual_plan_input_from_presentation,
        },
        shell::{
            NativePanelHostShellCommand, NativePanelHostShellLifecycle, NativePanelHostShellState,
            NativePanelPlatformWindowHandleAdapter, native_panel_has_raw_window_handle,
            sync_native_panel_raw_window_handle,
        },
    },
    native_panel_scene::{
        SceneMascotPose, panel_mascot_state_from_scene_pose, scene_mascot_pose_from_panel_state,
    },
};

pub(super) const WINDOWS_WM_MOUSEMOVE: u32 = 0x0200;
pub(super) const WINDOWS_WM_LBUTTONUP: u32 = 0x0202;
pub(super) const WINDOWS_WM_MOUSELEAVE: u32 = 0x02A3;
pub(super) const WINDOWS_WM_PAINT: u32 = 0x000F;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct WindowsNativePanelWindowHandle {
    pub(super) hwnd: Option<isize>,
}

impl NativePanelPlatformWindowHandleAdapter for WindowsNativePanelWindowHandle {
    type RawHandle = isize;

    fn raw_window_handle(&self) -> Option<Self::RawHandle> {
        self.hwnd
    }

    fn set_raw_window_handle(&mut self, handle: Option<Self::RawHandle>) {
        self.hwnd = handle;
    }
}

pub(super) type WindowsNativePanelShellCommand = NativePanelHostShellCommand;

pub(super) type WindowsNativePanelShellPaintJob = NativePanelVisualPlanInput;

pub(super) type WindowsNativePanelShellActionButtonPaintInput = NativePanelVisualActionButtonInput;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WindowsNativePanelShellDisplaySnapshot {
    pub(super) display_mode: NativePanelVisualDisplayMode,
    pub(super) visual_input: NativePanelVisualPlanInput,
    pub(super) shared_visible: bool,
    pub(super) pointer_region_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct WindowsNativePanelShellPresentResult {
    pub(super) display_updated: bool,
    pub(super) paint_queued: bool,
    pub(super) redraw_requested: bool,
}

#[derive(Clone, Debug, Default)]
pub(super) struct WindowsNativePanelWindowShell {
    shell_state: NativePanelHostShellState,
    handle: WindowsNativePanelWindowHandle,
    last_frame: Option<WindowsNativePanelDrawFrame>,
    pending_paint_job: Option<WindowsNativePanelShellPaintJob>,
    last_painted_job: Option<WindowsNativePanelShellPaintJob>,
    paint_pass_count: usize,
    display_snapshot: Option<WindowsNativePanelShellDisplaySnapshot>,
    last_pointer_input: Option<NativePanelPointerInput>,
    mascot_base_pose: Option<SceneMascotPose>,
    mascot_runtime: MascotRuntimeState,
}

impl WindowsNativePanelWindowShell {
    pub(super) fn raw_window_handle(&self) -> Option<isize> {
        self.handle.raw_window_handle()
    }

    pub(super) fn set_raw_window_handle(&mut self, hwnd: Option<isize>) {
        sync_native_panel_raw_window_handle(&mut self.handle, hwnd);
    }

    pub(super) fn has_raw_window_handle(&self) -> bool {
        native_panel_has_raw_window_handle(&self.handle)
    }

    pub(super) fn lifecycle(&self) -> NativePanelHostShellLifecycle {
        self.shell_state.lifecycle()
    }

    pub(super) fn redraw_requests(&self) -> usize {
        self.shell_state.redraw_requests()
    }

    pub(super) fn last_window_state(&self) -> Option<NativePanelHostWindowState> {
        self.shell_state.last_window_state()
    }

    pub(super) fn last_ignores_mouse_events(&self) -> Option<bool> {
        self.shell_state.last_ignores_mouse_events()
    }

    pub(super) fn last_frame(&self) -> Option<&WindowsNativePanelDrawFrame> {
        self.last_frame.as_ref()
    }

    pub(super) fn display_snapshot(&self) -> Option<&WindowsNativePanelShellDisplaySnapshot> {
        self.display_snapshot.as_ref()
    }

    pub(super) fn pending_paint_job(&self) -> Option<&WindowsNativePanelShellPaintJob> {
        self.pending_paint_job.as_ref()
    }

    pub(super) fn last_painted_job(&self) -> Option<&WindowsNativePanelShellPaintJob> {
        self.last_painted_job.as_ref()
    }

    pub(super) fn paint_pass_count(&self) -> usize {
        self.paint_pass_count
    }

    pub(super) fn last_pointer_input(&self) -> Option<NativePanelPointerInput> {
        self.last_pointer_input
    }

    pub(super) fn active_count_marquee_needs_refresh(&self) -> bool {
        self.lightweight_refresh_visible()
            && self
                .display_snapshot
                .as_ref()
                .is_some_and(|display| display.visual_input.active_count.chars().count() > 1)
    }

    pub(super) fn refresh_active_count_marquee(&mut self, elapsed_ms: u128) -> bool {
        let Some(display) = self.display_snapshot.as_mut() else {
            return false;
        };
        if display.visual_input.active_count.chars().count() <= 1 {
            return false;
        }
        display.visual_input.active_count_elapsed_ms = elapsed_ms;
        self.pending_paint_job = Some(display.visual_input.clone());
        self.request_redraw();
        true
    }

    pub(super) fn mascot_animation_needs_refresh(&self) -> bool {
        self.lightweight_refresh_visible()
            && self.display_snapshot.as_ref().is_some_and(|display| {
                display.visual_input.mascot_pose
                    != crate::native_panel_scene::SceneMascotPose::Hidden
            })
    }

    pub(super) fn refresh_mascot_animation(&mut self, elapsed_ms: u128) -> bool {
        let Some(display) = self.display_snapshot.as_mut() else {
            return false;
        };
        let base_pose = self
            .mascot_base_pose
            .unwrap_or(display.visual_input.mascot_pose);
        if display.visual_input.mascot_pose == crate::native_panel_scene::SceneMascotPose::Hidden {
            return false;
        }
        let pose = resolve_windows_shell_mascot_timed_pose(
            &mut self.mascot_runtime,
            display.display_mode,
            base_pose,
            elapsed_ms,
        );
        display.visual_input.mascot_pose = pose.pose;
        display.visual_input.mascot_elapsed_ms = pose.elapsed_ms;
        display.visual_input.mascot_motion_frame = pose.motion_frame;
        self.pending_paint_job = Some(display.visual_input.clone());
        self.request_redraw();
        true
    }

    pub(super) fn pointer_regions(&self) -> &[NativePanelPointerRegion] {
        self.last_frame
            .as_ref()
            .map(|frame| frame.pointer_regions.as_slice())
            .unwrap_or(&[])
    }

    fn lightweight_refresh_visible(&self) -> bool {
        if matches!(
            self.lifecycle(),
            NativePanelHostShellLifecycle::Detached | NativePanelHostShellLifecycle::Hidden
        ) {
            return false;
        }

        self.last_window_state()
            .map(|window_state| window_state.visible)
            .unwrap_or_else(|| {
                self.display_snapshot
                    .as_ref()
                    .is_some_and(|display| display.visual_input.window_state.visible)
            })
    }

    pub(super) fn pointer_state_at_point(&self, point: PanelPoint) -> NativePanelPointerPointState {
        NativePanelInteractionPlan::from_pointer_regions(self.pointer_regions())
            .pointer_state_at_point(point)
    }

    pub(super) fn hover_inside_for_input(&self, input: NativePanelPointerInput) -> Option<bool> {
        NativePanelInteractionPlan::from_pointer_regions(self.pointer_regions())
            .inside_for_input(input)
    }

    pub(super) fn platform_loop_started(&self) -> bool {
        self.shell_state.platform_loop_started()
    }

    pub(super) fn platform_loop_spawn_count(&self) -> usize {
        self.shell_state.platform_loop_spawn_count()
    }

    pub(super) fn take_pending_commands(&mut self) -> Vec<WindowsNativePanelShellCommand> {
        self.shell_state.take_pending_commands()
    }

    pub(super) fn has_pending_destroy_command(&self) -> bool {
        self.shell_state.has_pending_destroy_command()
    }

    pub(super) fn create(&mut self) {
        self.shell_state.create();
    }

    pub(super) fn show(&mut self) {
        self.shell_state.show();
    }

    pub(super) fn hide(&mut self) {
        self.shell_state.hide();
    }

    pub(super) fn destroy(&mut self) {
        if self.shell_state.destroy() {
            self.last_frame = None;
            self.set_raw_window_handle(None);
        }
    }

    pub(super) fn sync_window_state(&mut self, window_state: NativePanelHostWindowState) {
        self.shell_state.sync_window_state(window_state);
    }

    pub(super) fn request_redraw(&mut self) {
        self.shell_state.request_redraw();
    }

    pub(super) fn sync_mouse_event_passthrough(&mut self, ignores_mouse_events: bool) {
        self.shell_state
            .sync_mouse_event_passthrough(ignores_mouse_events);
    }

    pub(super) fn paint_next_frame(&mut self) -> Option<WindowsNativePanelShellPaintJob> {
        let job = self.pending_paint_job.take()?;
        self.paint_pass_count += 1;
        self.last_painted_job = Some(job.clone());
        Some(job)
    }

    pub(super) fn hover_frames(&self) -> Option<NativePanelHoverFallbackFrames> {
        let display = self.display_snapshot.as_ref()?;
        Some(windows_client_hover_fallback_frames(
            &display.visual_input,
            resolve_native_panel_hover_fallback_frames(&display.visual_input),
        ))
    }

    pub(super) fn polling_host_facts<'a>(
        &'a self,
        pointer: PanelPoint,
        primary_mouse_down: bool,
        snapshot: Option<echoisland_runtime::RuntimeSnapshot>,
    ) -> Option<NativePanelPollingHostFacts<'a>> {
        Some(NativePanelPollingHostFacts {
            pointer,
            pointer_regions: self.pointer_regions(),
            hover_frames: self.hover_frames()?,
            primary_mouse_down,
            cards_visible: self
                .display_snapshot
                .as_ref()
                .map(|display| display.visual_input.cards_visible)
                .unwrap_or(false),
            snapshot,
        })
    }

    pub(super) fn record_platform_loop_spawn(&mut self) {
        self.shell_state.record_platform_loop_spawn();
    }

    pub(super) fn decode_window_message(
        &self,
        message_id: u32,
        lparam: isize,
    ) -> Option<NativePanelPointerInput> {
        match message_id {
            WINDOWS_WM_MOUSEMOVE => Some(NativePanelPointerInput::Move(
                panel_point_from_window_lparam(lparam),
            )),
            WINDOWS_WM_LBUTTONUP => Some(NativePanelPointerInput::Click(
                panel_point_from_window_lparam(lparam),
            )),
            WINDOWS_WM_MOUSELEAVE => Some(NativePanelPointerInput::Leave),
            _ => None,
        }
    }

    pub(super) fn record_pointer_input(&mut self, input: NativePanelPointerInput) {
        self.last_pointer_input = Some(input);
    }

    pub(super) fn consume_presenter(
        &mut self,
        presenter: &mut WindowsNativePanelDrawPresenter,
    ) -> WindowsNativePanelShellPresentResult {
        let Some(frame) = presenter.take_redraw_frame() else {
            return WindowsNativePanelShellPresentResult::default();
        };
        self.sync_window_state(frame.window_state);
        let mut display_snapshot = build_display_snapshot(&frame);
        let previous_mascot_elapsed_ms = self
            .display_snapshot
            .as_ref()
            .map(|display| display.visual_input.mascot_elapsed_ms);
        let mascot_base_pose = display_snapshot.visual_input.mascot_pose;
        sync_presented_mascot_visual_state(
            &mut display_snapshot,
            &mut self.mascot_runtime,
            mascot_base_pose,
            previous_mascot_elapsed_ms,
        );
        self.mascot_base_pose =
            (mascot_base_pose != SceneMascotPose::Hidden).then_some(mascot_base_pose);
        let paint_job = build_paint_job(&display_snapshot);
        self.last_frame = Some(frame);
        self.display_snapshot = Some(display_snapshot);
        self.pending_paint_job = Some(paint_job);
        self.request_redraw();

        WindowsNativePanelShellPresentResult {
            display_updated: true,
            paint_queued: true,
            redraw_requested: true,
        }
    }
}

fn panel_point_from_window_lparam(lparam: isize) -> PanelPoint {
    let x = (lparam as u32 & 0xFFFF) as u16 as i16 as f64;
    let y = ((lparam as u32 >> 16) & 0xFFFF) as u16 as i16 as f64;
    PanelPoint { x, y }
}

fn display_mode_for_presentation(
    window_state: NativePanelHostWindowState,
    presentation: Option<&NativePanelPresentationModel>,
) -> NativePanelVisualDisplayMode {
    native_panel_visual_display_mode_from_presentation(window_state, presentation)
}

fn build_display_snapshot(
    frame: &WindowsNativePanelDrawFrame,
) -> WindowsNativePanelShellDisplaySnapshot {
    let presentation = frame.presentation_model.as_ref();
    let display_mode = display_mode_for_presentation(frame.window_state, presentation);

    WindowsNativePanelShellDisplaySnapshot {
        display_mode,
        visual_input: native_panel_visual_plan_input_from_presentation(
            frame.window_state,
            display_mode,
            presentation,
        ),
        shared_visible: presentation
            .map(|presentation| presentation.shell.shared_visible)
            .unwrap_or(false),
        pointer_region_count: frame.pointer_regions.len(),
    }
}

fn build_paint_job(
    display: &WindowsNativePanelShellDisplaySnapshot,
) -> WindowsNativePanelShellPaintJob {
    display.visual_input.clone()
}

fn windows_client_hover_fallback_frames(
    input: &NativePanelVisualPlanInput,
    frames: NativePanelHoverFallbackFrames,
) -> NativePanelHoverFallbackFrames {
    let surface_height = input
        .window_state
        .frame
        .map(|frame| frame.height)
        .or_else(|| non_zero_rect(input.content_frame).map(|frame| frame.height))
        .unwrap_or_else(|| {
            frames
                .interactive_pill_frame
                .height
                .max(frames.hover_pill_frame.height)
        })
        .max(1.0);
    let interactive_pill_frame = windows_client_frame(
        surface_height,
        local_visual_frame(input, frames.interactive_pill_frame),
    );
    let hover_pill_frame = windows_client_frame(
        surface_height,
        resolve_native_panel_stable_compact_hover_frame(local_visual_frame(
            input,
            frames.interactive_pill_frame,
        )),
    );
    let interactive_expanded_frame = frames
        .interactive_expanded_frame
        .map(|frame| windows_client_frame(surface_height, local_visual_frame(input, frame)));

    NativePanelHoverFallbackFrames {
        interactive_pill_frame,
        hover_pill_frame,
        interactive_expanded_frame,
    }
}

fn local_visual_frame(input: &NativePanelVisualPlanInput, frame: PanelRect) -> PanelRect {
    let panel = input.panel_frame;
    let frame_is_absolute = frame.x >= panel.x
        && frame.x + frame.width <= panel.x + panel.width
        && frame.y >= panel.y
        && frame.y + frame.height <= panel.y + panel.height;

    if frame_is_absolute {
        return PanelRect {
            x: frame.x - panel.x,
            y: frame.y - panel.y,
            width: frame.width,
            height: frame.height,
        };
    }

    frame
}

fn windows_client_frame(surface_height: f64, frame: PanelRect) -> PanelRect {
    PanelRect {
        x: frame.x,
        y: surface_height - frame.y - frame.height,
        width: frame.width,
        height: frame.height,
    }
}

fn non_zero_rect(rect: PanelRect) -> Option<PanelRect> {
    (rect.width > 0.0 && rect.height > 0.0).then_some(rect)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WindowsShellTimedMascotPose {
    pose: SceneMascotPose,
    elapsed_ms: u128,
    motion_frame: Option<MascotVisualFrame>,
}

fn sync_presented_mascot_visual_state(
    display: &mut WindowsNativePanelShellDisplaySnapshot,
    runtime: &mut MascotRuntimeState,
    base_pose: SceneMascotPose,
    previous_elapsed_ms: Option<u128>,
) {
    let elapsed_ms = previous_elapsed_ms.unwrap_or(display.visual_input.mascot_elapsed_ms);
    let pose = resolve_windows_shell_mascot_timed_pose(
        runtime,
        display.display_mode,
        base_pose,
        elapsed_ms,
    );
    display.visual_input.mascot_pose = pose.pose;
    display.visual_input.mascot_elapsed_ms = pose.elapsed_ms;
    display.visual_input.mascot_motion_frame = pose.motion_frame;
}

fn resolve_windows_shell_mascot_timed_pose(
    runtime: &mut MascotRuntimeState,
    display_mode: NativePanelVisualDisplayMode,
    current_pose: SceneMascotPose,
    elapsed_ms: u128,
) -> WindowsShellTimedMascotPose {
    if current_pose == SceneMascotPose::Hidden {
        runtime.reset(elapsed_ms);
        return WindowsShellTimedMascotPose {
            pose: SceneMascotPose::Hidden,
            elapsed_ms,
            motion_frame: None,
        };
    }

    let frame = runtime.next_frame(MascotRuntimeFrameInput {
        base_state: panel_mascot_state_from_scene_pose(current_pose),
        expanded: display_mode == NativePanelVisualDisplayMode::Expanded,
        elapsed_ms,
        transition_duration_ms: mascot_transition_duration_ms(),
    });
    WindowsShellTimedMascotPose {
        pose: scene_mascot_pose_from_panel_state(frame.state),
        elapsed_ms: frame.elapsed_ms,
        motion_frame: Some(frame.motion),
    }
}

fn mascot_transition_duration_ms() -> u128 {
    (MASCOT_STATE_TRANSITION_SECONDS * 1000.0).round() as u128
}

#[cfg(test)]
mod tests;
