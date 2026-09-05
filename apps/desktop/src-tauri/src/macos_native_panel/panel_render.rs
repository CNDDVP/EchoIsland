use super::compact_bar_layout::relayout_compact_content;
use super::macos_visual_plan::{
    apply_macos_visual_plan_compact_primitives, resolve_macos_native_panel_compact_bar_visual_plan,
    resolve_macos_native_panel_visual_plan,
};
use super::panel_action_buttons::{
    apply_edge_action_button_commands, edge_action_pointer_region_input,
    transition_edge_action_commands,
};
use super::panel_chrome_alpha::resolve_collapsed_chrome_alpha;
use super::panel_geometry::{
    apply_panel_frame, native_panel_core_layout, native_panel_geometry_metrics,
    resolve_native_panel_layout,
};
use super::panel_helpers::native_panel_content_visibility;
use super::panel_host_adapter::ns_rect_to_panel_rect;
use super::panel_refs::{NativePanelRefs, native_panel_state, resolve_native_panel_refs};
use super::panel_runtime_input::native_panel_runtime_input_descriptor;
use super::panel_scene_adapter::{
    resolve_and_cache_native_panel_presentation, resolve_current_native_panel_runtime_render_state,
    resolve_current_native_panel_scene,
};
use super::panel_screen_geometry::resolve_screen_frame_for_panel;
use super::panel_shoulder::apply_shoulder_path_scale_x;
use super::panel_style::apply_panel_layer_styles;
use super::panel_types::{NativePanelHandles, NativePanelLayout, NativePanelTransitionFrame};
use crate::native_panel_core::{
    PanelChromeVisibilitySpecInput, PanelRenderProgress, PanelRenderState, PanelRenderStateInput,
    resolve_panel_render_progress, resolve_panel_render_state,
};
use crate::native_panel_renderer::facade::presentation::{
    NativePanelActionButtonPresentation, NativePanelActionButtonsPresentation,
    NativePanelPresentationModel, resolve_native_panel_presentation,
    resolve_native_panel_presentation_model_for_scene,
};
use crate::native_panel_renderer::facade::renderer::native_panel_runtime_scene_cache_key_for_state_bridge;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct NativePanelGeometryFrameMetrics {
    pub(super) total_ms: u128,
    pub(super) view_ms: u128,
    pub(super) layer_ms: u128,
    pub(super) alpha_ms: u128,
    pub(super) visuals_ms: u128,
    pub(super) sync_ms: u128,
    pub(super) invalidate_ms: u128,
    pub(super) sync_path: &'static str,
    pub(super) shell_visible: bool,
}

#[allow(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn apply_panel_geometry(
    handles: NativePanelHandles,
    frame: NativePanelTransitionFrame,
) -> NativePanelGeometryFrameMetrics {
    apply_panel_geometry_with_metrics(handles, frame, true)
}

#[allow(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn apply_panel_geometry_without_metrics(
    handles: NativePanelHandles,
    frame: NativePanelTransitionFrame,
) {
    let _ = apply_panel_geometry_with_metrics(handles, frame, false);
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn apply_panel_geometry_with_metrics(
    handles: NativePanelHandles,
    frame: NativePanelTransitionFrame,
    metrics_enabled: bool,
) -> NativePanelGeometryFrameMetrics {
    let total_started_at = metrics_enabled.then(Instant::now);
    let refs = resolve_native_panel_refs(handles);
    let panel = refs.panel;
    let screen_frame = resolve_screen_frame_for_panel(panel).unwrap_or(panel.frame());
    let progress = resolve_panel_render_progress(frame);
    let runtime_state = resolve_current_native_panel_runtime_render_state();
    let content_visibility = native_panel_content_visibility();
    let layout = resolve_native_panel_layout(
        screen_frame,
        native_panel_geometry_metrics(panel.screen().as_deref(), screen_frame),
        frame.canvas_height,
        frame.visible_height,
        progress.bar,
        progress.height,
        progress.drop,
        content_visibility,
    );

    let view_started_at = metrics_enabled.then(Instant::now);
    apply_panel_view_frames(&refs, &layout, progress);
    let view_ms = elapsed_ms(view_started_at);
    let render_state = native_panel_render_state(
        &layout,
        progress,
        content_visibility,
        runtime_state.transitioning,
        runtime_state.shell_scene,
    );
    let layer_started_at = metrics_enabled.then(Instant::now);
    apply_panel_layer_styles(&refs, render_state.layer_style);
    let layer_ms = elapsed_ms(layer_started_at);
    if runtime_state.transitioning {
        let alpha_started_at = metrics_enabled.then(Instant::now);
        let collapsed_chrome_alpha =
            sync_transition_collapsed_chrome_alpha(render_state, runtime_state.shell_scene.surface);
        refs.mascot_shell
            .setAlphaValue(collapsed_chrome_alpha.clamp(0.0, 1.0));
        let alpha_ms = elapsed_ms(alpha_started_at);
        let visuals_started_at = metrics_enabled.then(Instant::now);
        sync_native_panel_transition_visuals(&layout, &refs, runtime_state, render_state);
        let visuals_ms = elapsed_ms(visuals_started_at);
        let invalidate_started_at = metrics_enabled.then(Instant::now);
        invalidate_panel_transition_views(&refs);
        let invalidate_ms = elapsed_ms(invalidate_started_at);
        let total_ms = elapsed_ms(total_started_at);
        return NativePanelGeometryFrameMetrics {
            total_ms,
            view_ms,
            layer_ms,
            alpha_ms,
            visuals_ms,
            invalidate_ms,
            sync_path: "transition",
            shell_visible: layout.shell_visible,
            ..NativePanelGeometryFrameMetrics::default()
        };
    }
    clear_transition_collapsed_chrome_alpha();
    let sync_started_at = metrics_enabled.then(Instant::now);
    let sync_path = if !layout.shell_visible
        && sync_native_panel_cached_compact_visuals(&layout, &refs, render_state)
    {
        "cached_collapsed"
    } else {
        sync_native_panel_pointer_regions(&layout, &refs, runtime_state, render_state);
        "full"
    };
    let sync_ms = elapsed_ms(sync_started_at);
    let invalidate_started_at = metrics_enabled.then(Instant::now);
    invalidate_panel_render_views(&refs);
    let invalidate_ms = elapsed_ms(invalidate_started_at);
    let total_ms = elapsed_ms(total_started_at);
    NativePanelGeometryFrameMetrics {
        total_ms,
        view_ms,
        layer_ms,
        sync_ms,
        invalidate_ms,
        sync_path,
        shell_visible: layout.shell_visible,
        ..NativePanelGeometryFrameMetrics::default()
    }
}

fn elapsed_ms(started_at: Option<Instant>) -> u128 {
    started_at.map_or(0, |started_at| started_at.elapsed().as_millis())
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn apply_panel_view_frames(
    refs: &NativePanelRefs,
    layout: &NativePanelLayout,
    progress: PanelRenderProgress,
) {
    let panel = refs.panel;
    let content_view = refs.content_view;
    let left_shoulder = refs.left_shoulder;
    let right_shoulder = refs.right_shoulder;
    let pill_view = refs.pill_view;
    let expanded_container = refs.expanded_container;
    let cards_container = refs.cards_container;
    let body_separator = refs.body_separator;

    apply_panel_frame(panel, layout.panel_frame);
    content_view.setFrame(layout.content_frame);
    pill_view.setFrame(layout.pill_frame);
    apply_shoulder_path_scale_x(
        left_shoulder,
        layout.left_shoulder_frame,
        progress.shoulder,
        true,
    );
    apply_shoulder_path_scale_x(
        right_shoulder,
        layout.right_shoulder_frame,
        progress.shoulder,
        false,
    );
    relayout_compact_content(refs, layout.pill_frame.size, false);
    expanded_container.setFrame(layout.expanded_frame);
    cards_container.setFrame(layout.cards_frame);
    body_separator.setFrame(layout.separator_frame);
}

fn sync_native_panel_transition_visuals(
    layout: &NativePanelLayout,
    refs: &NativePanelRefs,
    _runtime_state: crate::native_panel_scene::PanelRuntimeRenderState,
    render_state: PanelRenderState,
) {
    let mut presentation = if let Some(presentation) = cached_current_native_panel_presentation() {
        presentation
    } else {
        let Some(scene) = resolve_current_native_panel_scene() else {
            return;
        };
        resolve_native_panel_presentation_model_for_scene(&scene, None)
    };
    apply_transition_layout_to_presentation(layout, render_state, &mut presentation);
    apply_transition_presentation_visuals(layout, refs, render_state, &presentation);
}

fn sync_native_panel_cached_compact_visuals(
    layout: &NativePanelLayout,
    refs: &NativePanelRefs,
    render_state: PanelRenderState,
) -> bool {
    let Some(mut presentation) = cached_current_native_panel_presentation() else {
        return false;
    };
    apply_transition_layout_to_presentation(layout, render_state, &mut presentation);
    apply_transition_presentation_visuals(layout, refs, render_state, &presentation);
    true
}

fn apply_transition_presentation_visuals(
    layout: &NativePanelLayout,
    refs: &NativePanelRefs,
    render_state: PanelRenderState,
    presentation: &NativePanelPresentationModel,
) {
    let mut presentation = presentation.clone();
    let action_commands = transition_edge_action_commands(layout, render_state.layer_style);
    presentation.action_buttons = NativePanelActionButtonsPresentation {
        visible: action_commands.iter().any(|command| command.visible),
        buttons: action_commands
            .iter()
            .map(|command| NativePanelActionButtonPresentation {
                action: command.action,
                frame: command.frame,
            })
            .collect(),
    };
    presentation.compact_bar.actions_visible = presentation.action_buttons.visible;
    // macOS draws native cards through the card stack views. This visual plan is only
    // consumed for compact-bar primitives, so avoid rebuilding expanded card primitives
    // on every transition frame.
    presentation.card_stack.visible = false;
    presentation.card_stack.cards.clear();
    let visual_plan = resolve_macos_native_panel_compact_bar_visual_plan(layout, &presentation);
    apply_edge_action_button_commands(refs, layout, &action_commands);
    apply_macos_visual_plan_compact_primitives(refs, layout, &presentation, &visual_plan);
}

fn cached_current_native_panel_presentation() -> Option<NativePanelPresentationModel> {
    let input = native_panel_runtime_input_descriptor();
    native_panel_state()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| {
            let cache_key = native_panel_runtime_scene_cache_key_for_state_bridge(&*guard, &input);
            (guard.scene_cache.last_cache_key.as_ref() == Some(&cache_key)).then(|| {
                guard.scene_cache.last_scene.as_ref().map(|scene| {
                    resolve_native_panel_presentation_model_for_scene(
                        scene,
                        guard.scene_cache.last_render_command_bundle.as_ref(),
                    )
                })
            })?
        })
}

fn sync_transition_collapsed_chrome_alpha(
    render_state: PanelRenderState,
    surface: crate::native_panel_core::ExpandedSurface,
) -> f64 {
    let alpha = collapsed_chrome_alpha_for_render_state(render_state, surface);
    let _ = native_panel_state().and_then(|state| {
        state.lock().ok().map(|mut guard| {
            guard.transition_collapsed_chrome_alpha = Some(alpha);
        })
    });
    alpha
}

fn clear_transition_collapsed_chrome_alpha() {
    let _ = native_panel_state().and_then(|state| {
        state.lock().ok().map(|mut guard| {
            guard.transition_collapsed_chrome_alpha = None;
        })
    });
}

fn collapsed_chrome_alpha_for_render_state(
    render_state: PanelRenderState,
    surface: crate::native_panel_core::ExpandedSurface,
) -> f64 {
    resolve_collapsed_chrome_alpha(PanelChromeVisibilitySpecInput {
        expanded_display_mode: render_state.layer_style.shell_visible,
        surface,
        edge_actions_visible: render_state.layer_style.edge_actions_visible,
        transition_visibility_progress: render_state.layer_style.chrome_transition_progress,
    })
}

fn apply_transition_layout_to_presentation(
    layout: &NativePanelLayout,
    render_state: PanelRenderState,
    presentation: &mut NativePanelPresentationModel,
) {
    presentation.panel_frame = ns_rect_to_panel_rect(layout.panel_frame);
    presentation.content_frame = ns_rect_to_panel_rect(layout.content_frame);
    presentation.shell.frame = ns_rect_to_panel_rect(layout.expanded_frame);
    presentation.shell.visible = layout.shell_visible;
    presentation.shell.separator_visibility = layout.separator_visibility;
    presentation.shell.shared_visible = render_state.shared.visible;
    presentation.shell.chrome_transition_progress =
        render_state.layer_style.chrome_transition_progress;
    presentation.compact_bar.frame = ns_rect_to_panel_rect(layout.pill_frame);
    presentation.compact_bar.left_shoulder_frame =
        ns_rect_to_panel_rect(layout.left_shoulder_frame);
    presentation.compact_bar.right_shoulder_frame =
        ns_rect_to_panel_rect(layout.right_shoulder_frame);
    presentation.compact_bar.shoulder_progress = render_state.layer_style.shoulder_progress;
    presentation.compact_bar.headline_emphasized = render_state.layer_style.headline_emphasized;
    presentation.card_stack.frame = ns_rect_to_panel_rect(layout.cards_frame);
    presentation.card_stack.visible =
        layout.shell_visible && !presentation.card_stack.cards.is_empty();
}

fn sync_native_panel_pointer_regions(
    layout: &NativePanelLayout,
    refs: &NativePanelRefs,
    runtime_state: crate::native_panel_scene::PanelRuntimeRenderState,
    render_state: PanelRenderState,
) {
    let pointer_region_input = Some(edge_action_pointer_region_input(
        layout,
        render_state.layer_style,
    ));
    let resolved = native_panel_state()
        .and_then(|state| state.lock().ok())
        .and_then(|mut guard| {
            resolve_and_cache_native_panel_presentation(
                &mut guard,
                native_panel_core_layout(layout),
                render_state,
                pointer_region_input,
            )
        })
        .or_else(|| {
            let scene = resolve_current_native_panel_scene()?;
            Some(resolve_native_panel_presentation(
                native_panel_core_layout(layout),
                &scene,
                runtime_state,
                render_state,
                pointer_region_input,
            ))
        });
    let Some(resolved) = resolved else {
        return;
    };
    let visual_plan = resolve_macos_native_panel_visual_plan(layout, &resolved.presentation);
    apply_edge_action_button_commands(
        refs,
        layout,
        &resolved.presentation.action_button_commands(),
    );
    apply_macos_visual_plan_compact_primitives(refs, layout, &resolved.presentation, &visual_plan);
}

fn native_panel_render_state(
    layout: &NativePanelLayout,
    progress: PanelRenderProgress,
    content_visibility: f64,
    transitioning: bool,
    shell_scene: crate::native_panel_scene::PanelShellSceneState,
) -> PanelRenderState {
    let status_surface_active =
        shell_scene.surface == crate::native_panel_core::ExpandedSurface::Status;
    resolve_panel_render_state(PanelRenderStateInput {
        shared_expanded_enabled: false,
        shell_visible: layout.shell_visible,
        separator_visibility: layout.separator_visibility,
        bar_progress: progress.bar,
        height_progress: progress.height,
        chrome_transition_progress: progress.bar,
        shoulder_progress: progress.shoulder,
        cards_height: layout.cards_frame.size.height,
        status_surface_active,
        content_visibility,
        transitioning,
        headline_emphasized: shell_scene.headline_emphasized,
        edge_actions_visible: shell_scene.edge_actions_visible,
    })
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn invalidate_panel_render_views(refs: &NativePanelRefs) {
    refs.pill_view.displayIfNeeded();
    refs.expanded_container.displayIfNeeded();
    refs.left_shoulder.setNeedsDisplay(true);
    refs.right_shoulder.setNeedsDisplay(true);
    refs.pill_view.setNeedsDisplay(true);
    refs.expanded_container.setNeedsDisplay(true);
    refs.content_view.setNeedsDisplay(true);
    refs.content_view.layoutSubtreeIfNeeded();
    refs.content_view.displayIfNeededIgnoringOpacity();
    refs.panel.displayIfNeeded();
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn invalidate_panel_transition_views(refs: &NativePanelRefs) {
    refs.left_shoulder.setNeedsDisplay(true);
    refs.right_shoulder.setNeedsDisplay(true);
    refs.pill_view.setNeedsDisplay(true);
    refs.expanded_container.setNeedsDisplay(true);
    refs.content_view.setNeedsDisplay(true);
}
