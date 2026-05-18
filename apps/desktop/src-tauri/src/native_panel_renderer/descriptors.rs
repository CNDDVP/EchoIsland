use crate::{
    native_panel_core::{
        HoverTransition, PanelAnimationDescriptor, PanelHitTarget, PanelInteractionCommand,
        PanelLayout, PanelPoint, PanelRect, compose_local_rect, point_in_rect,
        resolve_compact_action_button_layout, resolve_native_panel_host_frame,
        resolve_settings_surface_card_height, settings_surface_row_frame,
    },
    native_panel_renderer::card_visual_spec::{CardVisualRowSpec, card_visual_settings_row_layout},
    native_panel_scene::{
        PanelDisplayOptionState, PanelScene, PanelSceneBuildInput, SceneCard, SceneHitTarget,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelRuntimeInputDescriptor {
    pub(crate) scene_input: PanelSceneBuildInput,
    pub(crate) screen_frame: Option<PanelRect>,
}

impl NativePanelRuntimeInputDescriptor {
    pub(crate) fn selected_display_index(&self) -> usize {
        self.scene_input.settings.selected_display_index
    }
}

pub(crate) fn native_panel_runtime_input_descriptor_with_screen_frame(
    screen_frame: Option<PanelRect>,
) -> NativePanelRuntimeInputDescriptor {
    NativePanelRuntimeInputDescriptor {
        scene_input: PanelSceneBuildInput::default(),
        screen_frame,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NativePanelRuntimeInputContext {
    pub(crate) display_options: Vec<PanelDisplayOptionState>,
    pub(crate) selected_display_index: usize,
    pub(crate) screen_frame: Option<PanelRect>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NativePanelHostWindowState {
    pub(crate) frame: Option<PanelRect>,
    pub(crate) visible: bool,
    pub(crate) preferred_display_index: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NativePanelHostWindowDescriptor {
    pub(crate) visible: bool,
    pub(crate) preferred_display_index: usize,
    pub(crate) screen_frame: Option<PanelRect>,
    pub(crate) shared_body_height: Option<f64>,
    pub(crate) timeline: Option<NativePanelTimelineDescriptor>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NativePanelHostWindowDescriptorPatch {
    pub(crate) visible: Option<bool>,
    pub(crate) preferred_display_index: Option<usize>,
    pub(crate) screen_frame: Option<Option<PanelRect>>,
    pub(crate) shared_body_height: Option<Option<f64>>,
    pub(crate) timeline: Option<Option<NativePanelTimelineDescriptor>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NativePanelTimelineDescriptor {
    pub(crate) animation: PanelAnimationDescriptor,
    pub(crate) cards_entering: bool,
}

impl NativePanelHostWindowDescriptor {
    pub(crate) fn animation_descriptor(self) -> Option<PanelAnimationDescriptor> {
        self.timeline.map(|timeline| timeline.animation)
    }

    pub(crate) fn window_state(self, frame: Option<PanelRect>) -> NativePanelHostWindowState {
        NativePanelHostWindowState {
            frame,
            visible: self.visible,
            preferred_display_index: self.preferred_display_index,
        }
    }
}

pub(crate) fn native_panel_host_window_descriptor(
    visible: bool,
    preferred_display_index: usize,
    screen_frame: Option<PanelRect>,
    shared_body_height: Option<f64>,
    timeline: Option<NativePanelTimelineDescriptor>,
) -> NativePanelHostWindowDescriptor {
    NativePanelHostWindowDescriptor {
        visible,
        preferred_display_index,
        screen_frame,
        shared_body_height,
        timeline,
    }
}

pub(crate) fn native_panel_timeline_descriptor(
    animation: PanelAnimationDescriptor,
    cards_entering: bool,
) -> NativePanelTimelineDescriptor {
    NativePanelTimelineDescriptor {
        animation,
        cards_entering,
    }
}

pub(crate) fn native_panel_timeline_descriptor_for_animation(
    animation: PanelAnimationDescriptor,
) -> NativePanelTimelineDescriptor {
    native_panel_timeline_descriptor(
        animation,
        native_panel_cards_entering_for_animation(animation),
    )
}

pub(crate) fn native_panel_cards_entering_for_animation(
    animation: PanelAnimationDescriptor,
) -> bool {
    !matches!(
        animation.kind,
        crate::native_panel_core::PanelAnimationKind::Close
    )
}

pub(crate) fn sync_native_panel_host_window_visibility(
    descriptor: &mut NativePanelHostWindowDescriptor,
    visible: bool,
) {
    patch_native_panel_host_window_descriptor(
        descriptor,
        NativePanelHostWindowDescriptorPatch {
            visible: Some(visible),
            ..NativePanelHostWindowDescriptorPatch::default()
        },
    );
}

pub(crate) fn sync_native_panel_host_window_screen_frame(
    descriptor: &mut NativePanelHostWindowDescriptor,
    preferred_display_index: usize,
    screen_frame: Option<PanelRect>,
) {
    patch_native_panel_host_window_descriptor(
        descriptor,
        NativePanelHostWindowDescriptorPatch {
            preferred_display_index: Some(preferred_display_index),
            screen_frame: Some(screen_frame),
            ..NativePanelHostWindowDescriptorPatch::default()
        },
    );
}

pub(crate) fn sync_native_panel_host_window_shared_body_height(
    descriptor: &mut NativePanelHostWindowDescriptor,
    shared_body_height: Option<f64>,
) {
    patch_native_panel_host_window_descriptor(
        descriptor,
        NativePanelHostWindowDescriptorPatch {
            shared_body_height: Some(shared_body_height),
            ..NativePanelHostWindowDescriptorPatch::default()
        },
    );
}

pub(crate) fn sync_native_panel_host_window_timeline(
    descriptor: &mut NativePanelHostWindowDescriptor,
    timeline: Option<NativePanelTimelineDescriptor>,
) {
    patch_native_panel_host_window_descriptor(
        descriptor,
        NativePanelHostWindowDescriptorPatch {
            timeline: Some(timeline),
            ..NativePanelHostWindowDescriptorPatch::default()
        },
    );
}

pub(crate) fn patch_native_panel_host_window_descriptor(
    descriptor: &mut NativePanelHostWindowDescriptor,
    patch: NativePanelHostWindowDescriptorPatch,
) {
    if let Some(visible) = patch.visible {
        descriptor.visible = visible;
    }
    if let Some(preferred_display_index) = patch.preferred_display_index {
        descriptor.preferred_display_index = preferred_display_index;
    }
    if let Some(screen_frame) = patch.screen_frame {
        descriptor.screen_frame = screen_frame;
    }
    if let Some(shared_body_height) = patch.shared_body_height {
        descriptor.shared_body_height = shared_body_height;
    }
    if let Some(timeline) = patch.timeline {
        descriptor.timeline = timeline;
    }
}

pub(crate) fn native_panel_host_window_frame(
    descriptor: NativePanelHostWindowDescriptor,
    fallback_screen_frame: PanelRect,
    compact_width: f64,
    expanded_width: f64,
) -> Option<PanelRect> {
    Some(resolve_native_panel_host_frame(
        descriptor.animation_descriptor()?,
        descriptor.screen_frame.unwrap_or(fallback_screen_frame),
        compact_width,
        expanded_width,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelPointerRegionKind {
    Shell,
    CompactBar,
    CardsContainer,
    DebugModeTrigger,
    EdgeAction(NativePanelEdgeAction),
    HitTarget(PanelHitTarget),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelEdgeAction {
    Settings,
    Quit,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelPointerRegion {
    pub(crate) frame: PanelRect,
    pub(crate) kind: NativePanelPointerRegionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelInteractionPlan {
    pub(crate) pointer_regions: Vec<NativePanelPointerRegion>,
}

impl NativePanelInteractionPlan {
    pub(crate) fn from_pointer_regions(regions: &[NativePanelPointerRegion]) -> Self {
        Self {
            pointer_regions: regions.to_vec(),
        }
    }

    pub(crate) fn pointer_region_at_point(
        &self,
        point: PanelPoint,
    ) -> Option<&NativePanelPointerRegion> {
        native_panel_pointer_region_at_point(&self.pointer_regions, point)
    }

    pub(crate) fn inside_regions(&self, point: PanelPoint) -> bool {
        self.pointer_region_at_point(point).is_some()
    }

    pub(crate) fn pointer_state_at_point(&self, point: PanelPoint) -> NativePanelPointerPointState {
        native_panel_pointer_state_at_point(&self.pointer_regions, point)
    }

    pub(crate) fn platform_event_at_point(
        &self,
        point: PanelPoint,
    ) -> Option<NativePanelPlatformEvent> {
        native_panel_platform_event_at_point(&self.pointer_regions, point)
    }

    pub(crate) fn input_outcome(
        &self,
        input: NativePanelPointerInput,
    ) -> NativePanelPointerInputOutcome {
        native_panel_pointer_input_outcome(&self.pointer_regions, input)
    }

    pub(crate) fn inside_for_input(&self, input: NativePanelPointerInput) -> Option<bool> {
        native_panel_pointer_inside_for_input(&self.pointer_regions, input)
    }

    pub(crate) fn hit_target_at_point(&self, point: PanelPoint) -> Option<PanelHitTarget> {
        native_panel_hit_target_at_point(&self.pointer_regions, point)
    }

    pub(crate) fn queue_platform_event_at_point(
        &self,
        events: &mut Vec<NativePanelPlatformEvent>,
        point: PanelPoint,
    ) -> Option<NativePanelPlatformEvent> {
        queue_native_panel_platform_event(events, self.platform_event_at_point(point))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativePanelPointerPointState {
    pub(crate) inside: bool,
    pub(crate) platform_event: Option<NativePanelPlatformEvent>,
    pub(crate) hit_target: Option<PanelHitTarget>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NativePanelEdgeActionFrames {
    pub(crate) settings_action: Option<PanelRect>,
    pub(crate) quit_action: Option<PanelRect>,
}

impl NativePanelEdgeActionFrames {
    fn edge_action_frame(self, action: NativePanelEdgeAction) -> Option<PanelRect> {
        match action {
            NativePanelEdgeAction::Settings => self.settings_action,
            NativePanelEdgeAction::Quit => self.quit_action,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct NativePanelPointerRegionInput {
    pub(crate) edge_action_frames: NativePanelEdgeActionFrames,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NativePanelPointerInput {
    Move(PanelPoint),
    Click(PanelPoint),
    Leave,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelPlatformEvent {
    FocusSession(String),
    ToggleSettingsSurface,
    QuitApplication,
    CycleDisplay,
    CycleIslandWidth,
    ToggleCompletionSound,
    ToggleMascot,
    DebugModeTrigger,
    OpenSettingsLocation,
    OpenReleasePage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelRuntimeCommand {
    FocusSession(String),
    ToggleSettingsSurface,
    QuitApplication,
    CycleDisplay,
    CycleIslandWidth,
    ToggleCompletionSound,
    ToggleMascot,
    DebugModeTrigger,
    OpenSettingsLocation,
    OpenReleasePage,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelPointerInputOutcome {
    Hover(Option<HoverTransition>),
    Click(Option<NativePanelPlatformEvent>),
}

impl NativePanelPointerInputOutcome {
    pub(crate) fn into_hover_transition(self) -> Option<HoverTransition> {
        match self {
            NativePanelPointerInputOutcome::Hover(transition) => transition,
            NativePanelPointerInputOutcome::Click(_) => None,
        }
    }

    pub(crate) fn into_click_event(self) -> Option<NativePanelPlatformEvent> {
        match self {
            NativePanelPointerInputOutcome::Click(event) => event,
            NativePanelPointerInputOutcome::Hover(_) => None,
        }
    }
}

pub(crate) trait NativePanelRuntimeCommandCapability {
    type Error;

    fn focus_session(&mut self, session_id: String) -> Result<(), Self::Error>;

    fn toggle_settings_surface(&mut self) -> Result<(), Self::Error>;

    fn quit_application(&mut self) -> Result<(), Self::Error>;

    fn cycle_display(&mut self) -> Result<(), Self::Error>;

    fn cycle_island_width(&mut self) -> Result<(), Self::Error>;

    fn toggle_completion_sound(&mut self) -> Result<(), Self::Error>;

    fn toggle_mascot(&mut self) -> Result<(), Self::Error>;

    fn debug_mode_trigger(&mut self) -> Result<(), Self::Error>;

    fn open_settings_location(&mut self) -> Result<(), Self::Error>;

    fn open_release_page(&mut self) -> Result<(), Self::Error>;
}

pub(crate) trait NativePanelRuntimeCommandHandler:
    NativePanelRuntimeCommandCapability
{
    fn execute_runtime_command(
        &mut self,
        command: NativePanelRuntimeCommand,
    ) -> Result<(), Self::Error> {
        match command {
            NativePanelRuntimeCommand::FocusSession(session_id) => self.focus_session(session_id),
            NativePanelRuntimeCommand::ToggleSettingsSurface => self.toggle_settings_surface(),
            NativePanelRuntimeCommand::QuitApplication => self.quit_application(),
            NativePanelRuntimeCommand::CycleDisplay => self.cycle_display(),
            NativePanelRuntimeCommand::CycleIslandWidth => self.cycle_island_width(),
            NativePanelRuntimeCommand::ToggleCompletionSound => self.toggle_completion_sound(),
            NativePanelRuntimeCommand::ToggleMascot => self.toggle_mascot(),
            NativePanelRuntimeCommand::DebugModeTrigger => self.debug_mode_trigger(),
            NativePanelRuntimeCommand::OpenSettingsLocation => self.open_settings_location(),
            NativePanelRuntimeCommand::OpenReleasePage => self.open_release_page(),
        }
    }
}

impl<T> NativePanelRuntimeCommandHandler for T where T: NativePanelRuntimeCommandCapability {}

#[derive(Default)]
pub(crate) struct NativePanelQueuedRuntimeCommandHandler {
    events: Vec<NativePanelPlatformEvent>,
}

impl NativePanelQueuedRuntimeCommandHandler {
    pub(crate) fn take_events(self) -> Vec<NativePanelPlatformEvent> {
        self.events
    }
}

impl NativePanelRuntimeCommandCapability for NativePanelQueuedRuntimeCommandHandler {
    type Error = String;

    fn focus_session(&mut self, session_id: String) -> Result<(), Self::Error> {
        self.events
            .push(NativePanelPlatformEvent::FocusSession(session_id));
        Ok(())
    }

    fn toggle_settings_surface(&mut self) -> Result<(), Self::Error> {
        self.events
            .push(NativePanelPlatformEvent::ToggleSettingsSurface);
        Ok(())
    }

    fn quit_application(&mut self) -> Result<(), Self::Error> {
        self.events.push(NativePanelPlatformEvent::QuitApplication);
        Ok(())
    }

    fn cycle_display(&mut self) -> Result<(), Self::Error> {
        self.events.push(NativePanelPlatformEvent::CycleDisplay);
        Ok(())
    }

    fn cycle_island_width(&mut self) -> Result<(), Self::Error> {
        self.events.push(NativePanelPlatformEvent::CycleIslandWidth);
        Ok(())
    }

    fn toggle_completion_sound(&mut self) -> Result<(), Self::Error> {
        self.events
            .push(NativePanelPlatformEvent::ToggleCompletionSound);
        Ok(())
    }

    fn toggle_mascot(&mut self) -> Result<(), Self::Error> {
        self.events.push(NativePanelPlatformEvent::ToggleMascot);
        Ok(())
    }

    fn debug_mode_trigger(&mut self) -> Result<(), Self::Error> {
        self.events.push(NativePanelPlatformEvent::DebugModeTrigger);
        Ok(())
    }

    fn open_settings_location(&mut self) -> Result<(), Self::Error> {
        self.events
            .push(NativePanelPlatformEvent::OpenSettingsLocation);
        Ok(())
    }

    fn open_release_page(&mut self) -> Result<(), Self::Error> {
        self.events.push(NativePanelPlatformEvent::OpenReleasePage);
        Ok(())
    }
}

impl From<SceneHitTarget> for PanelHitTarget {
    fn from(value: SceneHitTarget) -> Self {
        Self {
            action: value.action,
            value: value.value,
        }
    }
}

pub(crate) fn native_panel_runtime_command_for_platform_event(
    event: NativePanelPlatformEvent,
) -> NativePanelRuntimeCommand {
    match event {
        NativePanelPlatformEvent::FocusSession(session_id) => {
            NativePanelRuntimeCommand::FocusSession(session_id)
        }
        NativePanelPlatformEvent::ToggleSettingsSurface => {
            NativePanelRuntimeCommand::ToggleSettingsSurface
        }
        NativePanelPlatformEvent::QuitApplication => NativePanelRuntimeCommand::QuitApplication,
        NativePanelPlatformEvent::CycleDisplay => NativePanelRuntimeCommand::CycleDisplay,
        NativePanelPlatformEvent::CycleIslandWidth => NativePanelRuntimeCommand::CycleIslandWidth,
        NativePanelPlatformEvent::ToggleCompletionSound => {
            NativePanelRuntimeCommand::ToggleCompletionSound
        }
        NativePanelPlatformEvent::ToggleMascot => NativePanelRuntimeCommand::ToggleMascot,
        NativePanelPlatformEvent::DebugModeTrigger => NativePanelRuntimeCommand::DebugModeTrigger,
        NativePanelPlatformEvent::OpenSettingsLocation => {
            NativePanelRuntimeCommand::OpenSettingsLocation
        }
        NativePanelPlatformEvent::OpenReleasePage => NativePanelRuntimeCommand::OpenReleasePage,
    }
}

pub(crate) fn dispatch_native_panel_runtime_command<H>(
    handler: &mut H,
    command: NativePanelRuntimeCommand,
) -> Result<(), H::Error>
where
    H: NativePanelRuntimeCommandHandler,
{
    handler.execute_runtime_command(command)
}

pub(crate) fn dispatch_native_panel_runtime_commands<H>(
    handler: &mut H,
    commands: impl IntoIterator<Item = NativePanelRuntimeCommand>,
) -> Result<(), H::Error>
where
    H: NativePanelRuntimeCommandHandler,
{
    for command in commands {
        dispatch_native_panel_runtime_command(handler, command)?;
    }
    Ok(())
}

pub(crate) fn dispatch_native_panel_platform_event<H>(
    handler: &mut H,
    event: NativePanelPlatformEvent,
) -> Result<(), H::Error>
where
    H: NativePanelRuntimeCommandHandler,
{
    dispatch_native_panel_runtime_command(
        handler,
        native_panel_runtime_command_for_platform_event(event),
    )
}

pub(crate) fn dispatch_native_panel_platform_events<H>(
    handler: &mut H,
    events: impl IntoIterator<Item = NativePanelPlatformEvent>,
) -> Result<(), H::Error>
where
    H: NativePanelRuntimeCommandHandler,
{
    dispatch_native_panel_runtime_commands(
        handler,
        events
            .into_iter()
            .map(native_panel_runtime_command_for_platform_event),
    )
}

pub(crate) fn native_panel_platform_event_for_hit_target(
    target: &PanelHitTarget,
) -> NativePanelPlatformEvent {
    match target.action {
        crate::native_panel_core::PanelHitAction::FocusSession => {
            NativePanelPlatformEvent::FocusSession(target.value.clone())
        }
        crate::native_panel_core::PanelHitAction::CycleDisplay => {
            NativePanelPlatformEvent::CycleDisplay
        }
        crate::native_panel_core::PanelHitAction::CycleIslandWidth => {
            NativePanelPlatformEvent::CycleIslandWidth
        }
        crate::native_panel_core::PanelHitAction::ToggleCompletionSound => {
            NativePanelPlatformEvent::ToggleCompletionSound
        }
        crate::native_panel_core::PanelHitAction::ToggleMascot => {
            NativePanelPlatformEvent::ToggleMascot
        }
        crate::native_panel_core::PanelHitAction::OpenSettingsLocation => {
            NativePanelPlatformEvent::OpenSettingsLocation
        }
        crate::native_panel_core::PanelHitAction::OpenReleasePage => {
            NativePanelPlatformEvent::OpenReleasePage
        }
    }
}

pub(crate) fn native_panel_platform_event_for_pointer_region(
    region: &NativePanelPointerRegion,
) -> Option<NativePanelPlatformEvent> {
    match &region.kind {
        NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Settings) => {
            Some(NativePanelPlatformEvent::ToggleSettingsSurface)
        }
        NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Quit) => {
            Some(NativePanelPlatformEvent::QuitApplication)
        }
        NativePanelPointerRegionKind::DebugModeTrigger => {
            Some(NativePanelPlatformEvent::DebugModeTrigger)
        }
        NativePanelPointerRegionKind::HitTarget(target) => {
            Some(native_panel_platform_event_for_hit_target(target))
        }
        NativePanelPointerRegionKind::Shell
        | NativePanelPointerRegionKind::CompactBar
        | NativePanelPointerRegionKind::CardsContainer => None,
    }
}

pub(crate) fn native_panel_pointer_region_at_point<'a>(
    regions: &'a [NativePanelPointerRegion],
    point: PanelPoint,
) -> Option<&'a NativePanelPointerRegion> {
    regions
        .iter()
        .rev()
        .find(|region| point_in_rect(point, region.frame))
}

pub(crate) fn native_panel_pointer_inside_regions(
    regions: &[NativePanelPointerRegion],
    point: PanelPoint,
) -> bool {
    native_panel_pointer_region_at_point(regions, point).is_some()
}

pub(crate) fn native_panel_platform_event_at_point(
    regions: &[NativePanelPointerRegion],
    point: PanelPoint,
) -> Option<NativePanelPlatformEvent> {
    native_panel_pointer_region_at_point(regions, point)
        .and_then(native_panel_platform_event_for_pointer_region)
}

pub(crate) fn queue_native_panel_platform_event(
    events: &mut Vec<NativePanelPlatformEvent>,
    event: Option<NativePanelPlatformEvent>,
) -> Option<NativePanelPlatformEvent> {
    if let Some(event) = event.clone() {
        events.push(event);
    }
    event
}

pub(crate) fn queue_native_panel_platform_event_for_pointer_region(
    events: &mut Vec<NativePanelPlatformEvent>,
    region: &NativePanelPointerRegion,
) -> Option<NativePanelPlatformEvent> {
    queue_native_panel_platform_event(
        events,
        native_panel_platform_event_for_pointer_region(region),
    )
}

pub(crate) fn queue_native_panel_platform_event_at_point(
    events: &mut Vec<NativePanelPlatformEvent>,
    regions: &[NativePanelPointerRegion],
    point: PanelPoint,
) -> Option<NativePanelPlatformEvent> {
    queue_native_panel_platform_event(events, native_panel_platform_event_at_point(regions, point))
}

pub(crate) fn native_panel_pointer_state_at_point(
    regions: &[NativePanelPointerRegion],
    point: PanelPoint,
) -> NativePanelPointerPointState {
    let region = native_panel_pointer_region_at_point(regions, point);
    NativePanelPointerPointState {
        inside: region.is_some(),
        platform_event: region.and_then(native_panel_platform_event_for_pointer_region),
        hit_target: match region.map(|region| &region.kind) {
            Some(NativePanelPointerRegionKind::HitTarget(target)) => Some(target.clone()),
            _ => None,
        },
    }
}

pub(crate) fn native_panel_pointer_inside_for_input(
    regions: &[NativePanelPointerRegion],
    input: NativePanelPointerInput,
) -> Option<bool> {
    match input {
        NativePanelPointerInput::Move(point) => {
            Some(native_panel_pointer_inside_regions(regions, point))
        }
        NativePanelPointerInput::Leave => Some(false),
        NativePanelPointerInput::Click(_) => None,
    }
}

pub(crate) fn native_panel_platform_event_for_pointer_input(
    regions: &[NativePanelPointerRegion],
    input: NativePanelPointerInput,
) -> Option<NativePanelPlatformEvent> {
    match input {
        NativePanelPointerInput::Click(point) => {
            native_panel_platform_event_at_point(regions, point)
        }
        NativePanelPointerInput::Move(_) | NativePanelPointerInput::Leave => None,
    }
}

pub(crate) fn native_panel_hit_target_at_point(
    regions: &[NativePanelPointerRegion],
    point: PanelPoint,
) -> Option<PanelHitTarget> {
    match &native_panel_pointer_region_at_point(regions, point)?.kind {
        NativePanelPointerRegionKind::HitTarget(target) => Some(target.clone()),
        NativePanelPointerRegionKind::Shell
        | NativePanelPointerRegionKind::CompactBar
        | NativePanelPointerRegionKind::CardsContainer
        | NativePanelPointerRegionKind::DebugModeTrigger
        | NativePanelPointerRegionKind::EdgeAction(_) => None,
    }
}

pub(crate) fn native_panel_pointer_input_outcome(
    regions: &[NativePanelPointerRegion],
    input: NativePanelPointerInput,
) -> NativePanelPointerInputOutcome {
    match input {
        NativePanelPointerInput::Move(point) => NativePanelPointerInputOutcome::Hover(
            native_panel_pointer_inside_regions(regions, point).then_some(HoverTransition::Expand),
        ),
        NativePanelPointerInput::Leave => {
            NativePanelPointerInputOutcome::Hover(Some(HoverTransition::Collapse))
        }
        NativePanelPointerInput::Click(point) => NativePanelPointerInputOutcome::Click(
            native_panel_platform_event_at_point(regions, point),
        ),
    }
}

pub(crate) fn native_panel_platform_event_for_interaction_command(
    command: &PanelInteractionCommand,
) -> Option<NativePanelPlatformEvent> {
    match command {
        PanelInteractionCommand::HitTarget(target) => {
            Some(native_panel_platform_event_for_hit_target(target))
        }
        PanelInteractionCommand::ToggleSettingsSurface => {
            Some(NativePanelPlatformEvent::ToggleSettingsSurface)
        }
        PanelInteractionCommand::QuitApplication => Some(NativePanelPlatformEvent::QuitApplication),
        PanelInteractionCommand::None => None,
    }
}

pub(crate) fn resolve_native_panel_pointer_regions(
    layout: PanelLayout,
    scene: &PanelScene,
    input: Option<NativePanelPointerRegionInput>,
) -> Vec<NativePanelPointerRegion> {
    resolve_native_panel_interaction_plan(layout, scene, input).pointer_regions
}

pub(crate) fn resolve_native_panel_interaction_plan(
    layout: PanelLayout,
    scene: &PanelScene,
    input: Option<NativePanelPointerRegionInput>,
) -> NativePanelInteractionPlan {
    let mut regions = Vec::new();

    push_region(
        &mut regions,
        absolute_panel_rect(layout, layout.pill_frame),
        NativePanelPointerRegionKind::CompactBar,
    );
    push_mascot_bubble_hover_region(&mut regions, layout, scene);

    if layout.shell_visible {
        push_region(
            &mut regions,
            absolute_panel_rect(layout, layout.expanded_frame),
            NativePanelPointerRegionKind::Shell,
        );
        push_expanded_debug_mode_trigger_region(
            &mut regions,
            layout,
            input.unwrap_or_default().edge_action_frames,
        );
        push_expanded_top_gap_region(&mut regions, layout);
        push_region(
            &mut regions,
            absolute_expanded_rect(layout, layout.cards_frame),
            NativePanelPointerRegionKind::CardsContainer,
        );
        if scene.compact_bar.actions_visible {
            push_edge_action_regions(
                &mut regions,
                layout,
                input.unwrap_or_default().edge_action_frames,
            );
        }
        push_scene_hit_target_regions(&mut regions, layout, scene);
    }

    NativePanelInteractionPlan {
        pointer_regions: regions,
    }
}

fn push_expanded_debug_mode_trigger_region(
    regions: &mut Vec<NativePanelPointerRegion>,
    layout: PanelLayout,
    edge_action_frames: NativePanelEdgeActionFrames,
) {
    let pill = absolute_panel_rect(layout, layout.pill_frame);
    let action_layout = edge_action_frames
        .edge_action_frame(NativePanelEdgeAction::Settings)
        .unwrap_or_else(|| resolve_compact_action_button_layout(pill).settings);
    let trigger_size = 36.0;
    let trigger_gap = 6.0;
    push_region(
        regions,
        PanelRect {
            x: action_layout.x + action_layout.width + trigger_gap,
            y: pill.y + (pill.height - trigger_size) / 2.0,
            width: trigger_size,
            height: trigger_size,
        },
        NativePanelPointerRegionKind::DebugModeTrigger,
    );
}

fn push_expanded_top_gap_region(regions: &mut Vec<NativePanelPointerRegion>, layout: PanelLayout) {
    let gap_y = layout.expanded_frame.y + layout.expanded_frame.height;
    let gap_height = (layout.content_frame.height - gap_y).max(0.0);
    if gap_height <= 0.0 {
        return;
    }
    push_region(
        regions,
        absolute_panel_rect(
            layout,
            PanelRect {
                x: layout.expanded_frame.x,
                y: gap_y,
                width: layout.expanded_frame.width,
                height: gap_height,
            },
        ),
        NativePanelPointerRegionKind::Shell,
    );
}

fn push_mascot_bubble_hover_region(
    regions: &mut Vec<NativePanelPointerRegion>,
    layout: PanelLayout,
    scene: &PanelScene,
) {
    let has_bubble = scene.compact_bar.completion_count > 0
        || scene.mascot_pose == crate::native_panel_scene::SceneMascotPose::MessageBubble;
    if !has_bubble {
        return;
    }

    let pill = absolute_panel_rect(layout, layout.pill_frame);
    push_region(
        regions,
        PanelRect {
            x: pill.x + 20.0,
            y: pill.y + pill.height - 3.0,
            width: 30.0,
            height: 18.0,
        },
        NativePanelPointerRegionKind::CompactBar,
    );
}

fn push_edge_action_regions(
    regions: &mut Vec<NativePanelPointerRegion>,
    layout: PanelLayout,
    edge_action_frames: NativePanelEdgeActionFrames,
) {
    let pill = absolute_panel_rect(layout, layout.pill_frame);
    let action_layout = resolve_compact_action_button_layout(pill);
    let settings_frame = edge_action_frames
        .edge_action_frame(NativePanelEdgeAction::Settings)
        .unwrap_or_else(|| edge_action_hit_frame(action_layout.settings, pill));
    let quit_frame = edge_action_frames
        .edge_action_frame(NativePanelEdgeAction::Quit)
        .unwrap_or_else(|| edge_action_hit_frame(action_layout.quit, pill));
    push_region(
        regions,
        settings_frame,
        NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Settings),
    );
    push_region(
        regions,
        quit_frame,
        NativePanelPointerRegionKind::EdgeAction(NativePanelEdgeAction::Quit),
    );
}

fn edge_action_hit_frame(icon_frame: PanelRect, pill: PanelRect) -> PanelRect {
    let horizontal_padding = 5.0;
    PanelRect {
        x: icon_frame.x - horizontal_padding,
        y: pill.y,
        width: icon_frame.width + horizontal_padding * 2.0,
        height: pill.height,
    }
}

fn push_scene_hit_target_regions(
    regions: &mut Vec<NativePanelPointerRegion>,
    layout: PanelLayout,
    scene: &PanelScene,
) {
    if scene.hit_targets.is_empty() {
        return;
    }

    let cards = absolute_expanded_rect(layout, layout.cards_frame);
    if push_settings_hit_target_regions(regions, cards, scene) {
        return;
    }

    let target_count = scene.hit_targets.len();
    let row_height = cards.height / target_count as f64;
    for (index, target) in scene.hit_targets.iter().cloned().enumerate() {
        push_region(
            regions,
            PanelRect {
                x: cards.x,
                y: cards.y + cards.height - row_height * (index + 1) as f64,
                width: cards.width,
                height: row_height,
            },
            NativePanelPointerRegionKind::HitTarget(target.into()),
        );
    }
}

fn push_settings_hit_target_regions(
    regions: &mut Vec<NativePanelPointerRegion>,
    cards: PanelRect,
    scene: &PanelScene,
) -> bool {
    let Some(SceneCard::Settings { rows, .. }) = scene.cards.first() else {
        return false;
    };
    let card_height = resolve_settings_surface_card_height(rows.len());
    let card_frame = PanelRect {
        x: cards.x,
        y: cards.y - (card_height - cards.height).max(0.0),
        width: cards.width,
        height: card_height,
    };
    for (index, target) in scene.hit_targets.iter().cloned().enumerate() {
        push_settings_row_hit_target_regions(regions, card_frame, index, rows.get(index), target);
    }
    true
}

fn push_settings_row_hit_target_regions(
    regions: &mut Vec<NativePanelPointerRegion>,
    card_frame: PanelRect,
    index: usize,
    row: Option<&crate::native_panel_scene::SettingsRowScene>,
    target: SceneHitTarget,
) {
    let row_frame = settings_surface_row_frame(card_frame, index);
    push_region(
        regions,
        row_frame,
        NativePanelPointerRegionKind::HitTarget(target.clone().into()),
    );
    if let Some(row) = row {
        if let Some(layout) = card_visual_settings_row_layout(
            card_frame,
            index,
            &CardVisualRowSpec {
                title: row.title.clone(),
                value: row.value.text.clone(),
                active: row.value.emphasized,
            },
        ) {
            push_region(
                regions,
                layout.value_badge_frame,
                NativePanelPointerRegionKind::HitTarget(target.into()),
            );
        }
    }
}

fn absolute_panel_rect(layout: PanelLayout, local_frame: PanelRect) -> PanelRect {
    crate::native_panel_core::absolute_rect(layout.panel_frame, local_frame)
}

fn absolute_expanded_rect(layout: PanelLayout, local_frame: PanelRect) -> PanelRect {
    absolute_panel_rect(
        layout,
        compose_local_rect(layout.expanded_frame, local_frame),
    )
}

fn push_region(
    regions: &mut Vec<NativePanelPointerRegion>,
    frame: PanelRect,
    kind: NativePanelPointerRegionKind,
) {
    if frame.width <= 0.0 || frame.height <= 0.0 {
        return;
    }
    regions.push(NativePanelPointerRegion { frame, kind });
}

#[cfg(test)]
mod tests;
