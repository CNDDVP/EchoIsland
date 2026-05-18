use crate::{
    native_panel_core::{ExpandedSurface, MascotVisualFrame, PanelRect},
    native_panel_scene::SceneMascotPose,
};

use super::super::descriptors::{NativePanelEdgeAction, NativePanelHostWindowState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualDisplayMode {
    Hidden,
    Compact,
    Expanded,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelVisualPlanInput {
    pub(crate) window_state: NativePanelHostWindowState,
    pub(crate) display_mode: NativePanelVisualDisplayMode,
    pub(crate) surface: ExpandedSurface,
    pub(crate) panel_frame: PanelRect,
    pub(crate) compact_bar_frame: PanelRect,
    pub(crate) left_shoulder_frame: PanelRect,
    pub(crate) right_shoulder_frame: PanelRect,
    pub(crate) shoulder_progress: f64,
    pub(crate) content_frame: PanelRect,
    pub(crate) card_stack_frame: PanelRect,
    pub(crate) card_stack_content_height: f64,
    pub(crate) shell_frame: PanelRect,
    pub(crate) headline_text: String,
    pub(crate) headline_emphasized: bool,
    pub(crate) active_count: String,
    pub(crate) active_count_elapsed_ms: u128,
    pub(crate) total_count: String,
    pub(crate) separator_visibility: f64,
    pub(crate) chrome_transition_progress: f64,
    pub(crate) cards_visible: bool,
    pub(crate) card_count: usize,
    pub(crate) cards: Vec<NativePanelVisualCardInput>,
    pub(crate) glow_visible: bool,
    pub(crate) glow_opacity: f64,
    pub(crate) action_buttons_visible: bool,
    pub(crate) action_buttons: Vec<NativePanelVisualActionButtonInput>,
    pub(crate) completion_count: usize,
    pub(crate) mascot_elapsed_ms: u128,
    pub(crate) mascot_motion_frame: Option<MascotVisualFrame>,
    pub(crate) mascot_pose: SceneMascotPose,
    pub(crate) mascot_debug_mode_enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelVisualCardInput {
    pub(crate) style: NativePanelVisualCardStyle,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) body: Option<String>,
    pub(crate) badge: Option<NativePanelVisualCardBadgeInput>,
    pub(crate) source_badge: Option<NativePanelVisualCardBadgeInput>,
    pub(crate) body_prefix: Option<String>,
    pub(crate) body_lines: Vec<NativePanelVisualCardBodyLineInput>,
    pub(crate) action_hint: Option<String>,
    pub(crate) rows: Vec<NativePanelVisualCardRowInput>,
    pub(crate) height: f64,
    pub(crate) collapsed_height: f64,
    pub(crate) compact: bool,
    pub(crate) removing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualCardStyle {
    Default,
    Pending,
    PendingApproval,
    PendingQuestion,
    PromptAssist,
    Completion,
    Settings,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePanelVisualCardBodyRole {
    Assistant,
    User,
    Tool,
    Plain,
    ActionHint,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelVisualCardBodyLineInput {
    pub(crate) role: NativePanelVisualCardBodyRole,
    pub(crate) prefix: Option<String>,
    pub(crate) text: String,
    pub(crate) max_lines: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelVisualCardBadgeInput {
    pub(crate) text: String,
    pub(crate) emphasized: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativePanelVisualCardRowInput {
    pub(crate) title: String,
    pub(crate) value: String,
    pub(crate) active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NativePanelVisualActionButtonInput {
    pub(crate) action: NativePanelEdgeAction,
    pub(crate) frame: PanelRect,
    pub(crate) debug_mode_enabled: bool,
}
