use echoisland_runtime::RuntimeSnapshot;

use super::{
    panel_refs::native_panel_state, panel_runtime_input::native_panel_runtime_input_descriptor,
    panel_types::NativePanelState,
};
use crate::native_panel_renderer::facade::{
    descriptor::{NativePanelPointerRegionInput, NativePanelRuntimeInputDescriptor},
    presentation::{
        NativePanelPresentationModel, NativePanelResolvedPresentation,
        NativePanelSnapshotRenderPlan, resolve_native_panel_presentation,
        resolve_native_panel_presentation_model_for_scene,
        resolve_native_panel_snapshot_render_plan_for_scene,
    },
    renderer::{
        NativePanelRenderCommandBundle, build_native_panel_scene_for_state_bridge_with_input,
        cache_render_command_bundle_for_state_bridge_with_input,
        native_panel_runtime_render_state_from_preserved_scene,
        native_panel_status_close_preservation_active,
        resolve_and_cache_native_panel_presentation_for_state_bridge_with_input,
        resolve_current_native_panel_presentation_model_for_state_bridge_with_input,
        resolve_current_native_panel_render_command_bundle_for_state_bridge_with_input,
        resolve_native_panel_presentation_model_for_state_bridge_and_snapshot_with_input,
        resolve_native_panel_preserved_status_close_scene,
        resolve_native_panel_preserved_status_close_scene_for_snapshot,
        resolve_native_panel_render_command_bundle_for_state_bridge_and_snapshot_with_input,
        resolve_native_panel_runtime_render_state_for_state_bridge_with_input,
        resolve_native_panel_scene_for_state_bridge_and_snapshot_with_input,
        resolve_native_panel_scene_for_state_bridge_with_input,
        resolve_native_panel_snapshot_render_plan_for_state_bridge_snapshot_with_input,
    },
};

pub(super) fn build_native_panel_scene(
    snapshot: &RuntimeSnapshot,
) -> crate::native_panel_scene::PanelScene {
    let input = native_panel_runtime_input_descriptor();
    native_panel_state()
        .and_then(|state| state.lock().ok())
        .map(|guard| build_native_panel_scene_for_state_with_input(&guard, snapshot, &input))
        .unwrap_or_else(|| {
            build_native_panel_scene_for_core_state_with_input(
                snapshot,
                &crate::native_panel_core::PanelState::default(),
                &input,
            )
        })
}

pub(super) fn resolve_or_build_native_panel_scene(
    snapshot: &RuntimeSnapshot,
) -> crate::native_panel_scene::PanelScene {
    resolve_current_native_panel_scene_for_snapshot(snapshot)
        .unwrap_or_else(|| build_native_panel_scene(snapshot))
}

pub(super) fn build_native_panel_scene_for_state_with_input(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
    input: &NativePanelRuntimeInputDescriptor,
) -> crate::native_panel_scene::PanelScene {
    build_native_panel_scene_for_state_bridge_with_input(state, snapshot, input)
}

pub(super) fn build_native_panel_scene_for_core_state_with_input(
    snapshot: &RuntimeSnapshot,
    state: &crate::native_panel_core::PanelState,
    input: &NativePanelRuntimeInputDescriptor,
) -> crate::native_panel_scene::PanelScene {
    crate::native_panel_scene::build_panel_scene(state, snapshot, &input.scene_input)
}

pub(super) fn resolve_current_native_panel_runtime_render_state()
-> crate::native_panel_scene::PanelRuntimeRenderState {
    let input = native_panel_runtime_input_descriptor();
    native_panel_state()
        .and_then(|state| state.lock().ok())
        .map(|guard| build_native_panel_runtime_render_state_for_state_with_input(&guard, &input))
        .unwrap_or_default()
}

pub(super) fn resolve_current_native_panel_scene() -> Option<crate::native_panel_scene::PanelScene>
{
    let input = native_panel_runtime_input_descriptor();
    native_panel_state()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| resolve_native_panel_scene_for_state_with_input(&guard, &input))
}

pub(super) fn resolve_current_native_panel_scene_for_snapshot(
    snapshot: &RuntimeSnapshot,
) -> Option<crate::native_panel_scene::PanelScene> {
    let input = native_panel_runtime_input_descriptor();
    native_panel_state()
        .and_then(|state| state.lock().ok())
        .and_then(|guard| {
            resolve_native_panel_scene_for_state_and_snapshot(&guard, snapshot, &input)
        })
}

pub(super) fn resolve_current_native_panel_render_command_bundle(
    state: &NativePanelState,
) -> Option<NativePanelRenderCommandBundle> {
    resolve_current_native_panel_render_command_bundle_for_state_bridge_with_input(
        state,
        &native_panel_runtime_input_descriptor(),
    )
}

pub(super) fn resolve_and_cache_native_panel_presentation(
    state: &mut NativePanelState,
    layout: crate::native_panel_core::PanelLayout,
    render_state: crate::native_panel_core::PanelRenderState,
    pointer_region_input: Option<NativePanelPointerRegionInput>,
) -> Option<NativePanelResolvedPresentation> {
    let input = native_panel_runtime_input_descriptor();
    if let Some(resolved) = resolve_and_cache_preserved_status_close_presentation(
        state,
        &input,
        layout,
        render_state,
        pointer_region_input,
    ) {
        return Some(resolved);
    }
    resolve_and_cache_native_panel_presentation_for_state_bridge_with_input(
        state,
        &input,
        layout,
        render_state,
        pointer_region_input,
    )
}

pub(super) fn resolve_snapshot_render_plan(
    snapshot: &RuntimeSnapshot,
) -> NativePanelSnapshotRenderPlan {
    let input = native_panel_runtime_input_descriptor();
    native_panel_state()
        .and_then(|state| state.lock().ok())
        .map(|guard| {
            resolve_snapshot_render_plan_for_state_snapshot_with_input(&guard, snapshot, &input)
        })
        .unwrap_or_else(|| {
            resolve_native_panel_snapshot_render_plan_for_scene(
                resolve_or_build_native_panel_scene(snapshot),
                None,
            )
        })
}

#[cfg(test)]
pub(super) fn resolve_snapshot_card_stack_command(
    snapshot: &RuntimeSnapshot,
    frame: crate::native_panel_core::PanelRect,
    visible: bool,
) -> crate::native_panel_renderer::facade::presentation::NativePanelCardStackCommand {
    resolve_snapshot_render_plan(snapshot).card_stack_command(frame, visible)
}

pub(super) fn build_native_panel_runtime_render_state_for_state_with_input(
    state: &NativePanelState,
    input: &NativePanelRuntimeInputDescriptor,
) -> crate::native_panel_scene::PanelRuntimeRenderState {
    if let Some(scene) = cached_status_close_scene(state) {
        return native_panel_runtime_render_state_from_preserved_scene(state.transitioning, &scene);
    }
    resolve_native_panel_runtime_render_state_for_state_bridge_with_input(state, input)
}

pub(super) fn resolve_native_panel_scene_for_state_with_input(
    state: &NativePanelState,
    input: &NativePanelRuntimeInputDescriptor,
) -> Option<crate::native_panel_scene::PanelScene> {
    if let Some(scene) = cached_status_close_scene(state) {
        return Some(scene);
    }
    resolve_native_panel_scene_for_state_bridge_with_input(state, input)
}

pub(super) fn resolve_native_panel_scene_for_state_and_snapshot(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
    input: &NativePanelRuntimeInputDescriptor,
) -> Option<crate::native_panel_scene::PanelScene> {
    if let Some(scene) = cached_status_close_scene_for_snapshot(state, snapshot) {
        return Some(scene);
    }
    resolve_native_panel_scene_for_state_bridge_and_snapshot_with_input(state, snapshot, input)
}

#[allow(dead_code)]
fn resolve_native_panel_render_command_bundle_for_state_and_snapshot_with_input(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
    input: &NativePanelRuntimeInputDescriptor,
) -> Option<NativePanelRenderCommandBundle> {
    resolve_native_panel_render_command_bundle_for_state_bridge_and_snapshot_with_input(
        state, snapshot, input,
    )
}

#[allow(dead_code)]
pub(super) fn resolve_native_panel_render_command_bundle_for_state_and_snapshot(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
) -> Option<NativePanelRenderCommandBundle> {
    resolve_native_panel_render_command_bundle_for_state_and_snapshot_with_input(
        state,
        snapshot,
        &native_panel_runtime_input_descriptor(),
    )
}

#[allow(dead_code)]
pub(super) fn resolve_native_panel_presentation_model_for_state_and_snapshot(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
) -> Option<NativePanelPresentationModel> {
    if let Some(scene) = cached_status_close_scene_for_snapshot(state, snapshot) {
        return Some(resolve_native_panel_presentation_model_for_scene(
            &scene, None,
        ));
    }
    let input = native_panel_runtime_input_descriptor();
    resolve_native_panel_presentation_model_for_state_bridge_and_snapshot_with_input(
        state, snapshot, &input,
    )
}

pub(super) fn resolve_current_native_panel_presentation_model(
    state: &NativePanelState,
) -> Option<NativePanelPresentationModel> {
    if let Some(scene) = cached_status_close_scene(state) {
        return Some(resolve_native_panel_presentation_model_for_scene(
            &scene, None,
        ));
    }
    resolve_current_native_panel_presentation_model_for_state_bridge_with_input(
        state,
        &native_panel_runtime_input_descriptor(),
    )
}

fn resolve_snapshot_render_plan_for_state_snapshot_with_input(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
    input: &NativePanelRuntimeInputDescriptor,
) -> NativePanelSnapshotRenderPlan {
    if let Some(scene) = cached_status_close_scene_for_snapshot(state, snapshot) {
        return resolve_native_panel_snapshot_render_plan_for_scene(scene, None);
    }
    resolve_native_panel_snapshot_render_plan_for_state_bridge_snapshot_with_input(
        state, snapshot, input,
    )
}

fn resolve_and_cache_preserved_status_close_presentation(
    state: &mut NativePanelState,
    input: &NativePanelRuntimeInputDescriptor,
    layout: crate::native_panel_core::PanelLayout,
    render_state: crate::native_panel_core::PanelRenderState,
    pointer_region_input: Option<NativePanelPointerRegionInput>,
) -> Option<NativePanelResolvedPresentation> {
    let scene = cached_status_close_scene(state)?;
    let runtime =
        native_panel_runtime_render_state_from_preserved_scene(state.transitioning, &scene);
    let resolved = resolve_native_panel_presentation(
        layout,
        &scene,
        runtime,
        render_state,
        pointer_region_input,
    );
    cache_render_command_bundle_for_state_bridge_with_input(state, input, &resolved.bundle);
    Some(resolved)
}

fn cached_status_close_scene_for_snapshot(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
) -> Option<crate::native_panel_scene::PanelScene> {
    resolve_native_panel_preserved_status_close_scene_for_snapshot(
        &state.scene_cache,
        state.last_snapshot.as_ref(),
        snapshot,
        status_close_scene_preservation_active(state),
    )
}

fn cached_status_close_scene(
    state: &NativePanelState,
) -> Option<crate::native_panel_scene::PanelScene> {
    resolve_native_panel_preserved_status_close_scene(
        &state.scene_cache,
        status_close_scene_preservation_active(state),
    )
}

fn status_close_scene_preservation_active(state: &NativePanelState) -> bool {
    native_panel_status_close_preservation_active(
        state.transitioning,
        state.expanded,
        state.skip_next_close_card_exit,
    )
}

#[cfg(test)]
fn current_snapshot_render_plan_for_state_and_snapshot(
    state: &NativePanelState,
    snapshot: &RuntimeSnapshot,
) -> NativePanelSnapshotRenderPlan {
    let input = native_panel_runtime_input_descriptor();
    resolve_snapshot_render_plan_for_state_snapshot_with_input(state, snapshot, &input)
}

#[cfg(test)]
pub(super) fn resolve_native_panel_runtime_render_state_for_state_with_input(
    state: &NativePanelState,
    input: &NativePanelRuntimeInputDescriptor,
) -> crate::native_panel_scene::PanelRuntimeRenderState {
    resolve_native_panel_runtime_render_state_for_state_bridge_with_input(state, input)
}

#[cfg(test)]
mod tests;
