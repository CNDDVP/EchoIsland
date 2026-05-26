use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSColor, NSFont, NSTextAlignment, NSTextField, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::panel_refs::NativePanelRefs;
use super::panel_types::NativePanelLayout;
use crate::native_panel_core::{
    PanelRect, PanelRenderLayerStyleState, resolve_compact_action_button_layout,
};
use crate::native_panel_renderer::facade::{
    descriptor::{
        NativePanelEdgeAction, NativePanelEdgeActionFrames, NativePanelPointerRegionInput,
    },
    presentation::{
        ActionButtonVisibilitySpecInput, NativePanelActionButtonCommand,
        action_button_transition_progress_from_compact_width, action_button_visual_frame_for_phase,
        resolve_action_button_visibility_spec,
    },
};

pub(super) struct EdgeActionButtonViews {
    pub(super) button: Retained<NSView>,
    pub(super) label: Retained<NSTextField>,
}

pub(super) fn text_primary_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(0.96, 0.97, 0.99, 0.88)
}

pub(super) fn close_action_color() -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.32, 0.32, 0.95)
}

pub(super) fn create_edge_action_button(
    mtm: MainThreadMarker,
    label: &str,
    text_color: Retained<NSColor>,
    font_size: f64,
    label_y: f64,
) -> EdgeActionButtonViews {
    let button = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(26.0, 26.0)),
    );
    button.setWantsLayer(true);
    if let Some(layer) = button.layer() {
        layer.setCornerRadius(0.0);
        layer.setMasksToBounds(true);
        layer.setBackgroundColor(Some(&NSColor::clearColor().CGColor()));
        layer.setBorderWidth(0.0);
    }
    button.setHidden(true);
    button.setAlphaValue(0.0);

    let label = NSTextField::labelWithString(&NSString::from_str(label), mtm);
    label.setFrame(NSRect::new(
        NSPoint::new(0.0, label_y),
        NSSize::new(26.0, 20.0),
    ));
    label.setAlignment(NSTextAlignment::Center);
    label.setTextColor(Some(&text_color));
    label.setFont(Some(&NSFont::boldSystemFontOfSize(font_size)));
    label.setDrawsBackground(false);
    label.setBezeled(false);
    label.setBordered(false);
    label.setEditable(false);
    label.setSelectable(false);
    button.addSubview(&label);

    EdgeActionButtonViews { button, label }
}

pub(super) fn apply_edge_action_button_commands(
    refs: &NativePanelRefs,
    layout: &NativePanelLayout,
    commands: &[NativePanelActionButtonCommand],
) {
    for command in commands {
        let frame = edge_action_command_local_frame(layout, command.frame);
        match command.action {
            NativePanelEdgeAction::Settings => {
                refs.settings_button.setFrame(frame);
            }
            NativePanelEdgeAction::Quit => {
                refs.quit_button.setFrame(frame);
            }
        }
    }
}

pub(super) fn edge_action_pointer_region_input(
    layout: &NativePanelLayout,
    layer_style: PanelRenderLayerStyleState,
) -> NativePanelPointerRegionInput {
    let frames = edge_action_visual_frames(layout, layer_style);
    NativePanelPointerRegionInput {
        edge_action_frames: frames,
    }
}

pub(super) fn transition_edge_action_commands(
    layout: &NativePanelLayout,
    layer_style: PanelRenderLayerStyleState,
) -> Vec<NativePanelActionButtonCommand> {
    let visibility = edge_action_visibility(layout, layer_style);
    let frames = edge_action_visual_frames_with_visibility(layout, visibility);
    [
        (NativePanelEdgeAction::Settings, frames.settings_action),
        (NativePanelEdgeAction::Quit, frames.quit_action),
    ]
    .into_iter()
    .filter_map(|(action, frame)| {
        frame.map(|frame| NativePanelActionButtonCommand {
            action,
            frame,
            visible: visibility.visible,
        })
    })
    .collect()
}

fn edge_action_command_local_frame(layout: &NativePanelLayout, frame: PanelRect) -> NSRect {
    NSRect::new(
        NSPoint::new(
            frame.x - layout.panel_frame.origin.x - layout.pill_frame.origin.x,
            frame.y - layout.panel_frame.origin.y - layout.pill_frame.origin.y,
        ),
        NSSize::new(frame.width, frame.height),
    )
}

fn edge_action_visual_frames(
    layout: &NativePanelLayout,
    layer_style: PanelRenderLayerStyleState,
) -> NativePanelEdgeActionFrames {
    edge_action_visual_frames_with_visibility(layout, edge_action_visibility(layout, layer_style))
}

fn edge_action_visibility(
    layout: &NativePanelLayout,
    layer_style: PanelRenderLayerStyleState,
) -> crate::native_panel_core::ActionButtonVisibilitySpec {
    let compact_frame = PanelRect {
        x: layout.panel_frame.origin.x + layout.pill_frame.origin.x,
        y: layout.panel_frame.origin.y + layout.pill_frame.origin.y,
        width: layout.pill_frame.size.width,
        height: layout.pill_frame.size.height,
    };
    resolve_action_button_visibility_spec(ActionButtonVisibilitySpecInput {
        semantic_visible: layer_style.edge_actions_visible,
        expanded_display_mode: layer_style.shell_visible,
        transition_visibility_progress: action_button_transition_progress_from_compact_width(
            compact_frame.width,
        ),
    })
}

fn edge_action_visual_frames_with_visibility(
    layout: &NativePanelLayout,
    visibility: crate::native_panel_core::ActionButtonVisibilitySpec,
) -> NativePanelEdgeActionFrames {
    let compact_frame = PanelRect {
        x: layout.panel_frame.origin.x + layout.pill_frame.origin.x,
        y: layout.panel_frame.origin.y + layout.pill_frame.origin.y,
        width: layout.pill_frame.size.width,
        height: layout.pill_frame.size.height,
    };
    let action_layout = resolve_compact_action_button_layout(compact_frame);
    NativePanelEdgeActionFrames {
        settings_action: Some(action_button_visual_frame_for_phase(
            action_layout.settings,
            visibility,
            -1.0,
        )),
        quit_action: Some(action_button_visual_frame_for_phase(
            action_layout.quit,
            visibility,
            1.0,
        )),
    }
}

#[cfg(test)]
mod tests {
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    use super::{edge_action_command_local_frame, edge_action_visual_frames};
    use crate::{
        macos_native_panel::panel_types::NativePanelLayout,
        native_panel_core::{
            PanelRect, PanelRenderLayerStyleState, resolve_compact_action_button_layout,
        },
    };

    fn layout() -> NativePanelLayout {
        NativePanelLayout {
            panel_frame: NSRect::new(NSPoint::new(500.0, 700.0), NSSize::new(420.0, 180.0)),
            content_frame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(420.0, 180.0)),
            pill_frame: NSRect::new(NSPoint::new(68.5, 118.0), NSSize::new(283.0, 38.0)),
            left_shoulder_frame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
            right_shoulder_frame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
            expanded_frame: NSRect::new(NSPoint::new(68.5, 0.0), NSSize::new(283.0, 180.0)),
            cards_frame: NSRect::new(NSPoint::new(14.0, 40.0), NSSize::new(255.0, 120.0)),
            separator_frame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
            shared_content_frame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
            shell_visible: true,
            separator_visibility: 0.88,
        }
    }

    #[test]
    fn edge_action_pointer_frames_use_shared_visual_frames() {
        let layout = layout();
        let layer_style = PanelRenderLayerStyleState {
            shell_visible: true,
            separator_visibility: 1.0,
            shared_visible: false,
            bar_progress: 1.0,
            height_progress: 1.0,
            chrome_transition_progress: 1.0,
            shoulder_progress: 0.0,
            headline_emphasized: false,
            edge_actions_visible: true,
        };
        let frames = edge_action_visual_frames(&layout, layer_style);
        let compact_frame = PanelRect {
            x: layout.panel_frame.origin.x + layout.pill_frame.origin.x,
            y: layout.panel_frame.origin.y + layout.pill_frame.origin.y,
            width: layout.pill_frame.size.width,
            height: layout.pill_frame.size.height,
        };
        let expected = resolve_compact_action_button_layout(compact_frame);

        assert_eq!(frames.settings_action, Some(expected.settings));
        assert_eq!(frames.quit_action, Some(expected.quit));
    }

    #[test]
    fn edge_action_command_local_frame_preserves_shared_visual_frame_position() {
        let layout = layout();
        let compact_frame = PanelRect {
            x: layout.panel_frame.origin.x + layout.pill_frame.origin.x,
            y: layout.panel_frame.origin.y + layout.pill_frame.origin.y,
            width: layout.pill_frame.size.width,
            height: layout.pill_frame.size.height,
        };
        let action_layout = resolve_compact_action_button_layout(compact_frame);

        for expected in [action_layout.settings, action_layout.quit] {
            let local = edge_action_command_local_frame(&layout, expected);

            assert_eq!(
                local.origin.x,
                expected.x - layout.panel_frame.origin.x - layout.pill_frame.origin.x
            );
            assert_eq!(
                local.origin.y,
                expected.y - layout.panel_frame.origin.y - layout.pill_frame.origin.y
            );
            assert_eq!(local.size.width, expected.width);
            assert_eq!(local.size.height, expected.height);
        }
    }
}
