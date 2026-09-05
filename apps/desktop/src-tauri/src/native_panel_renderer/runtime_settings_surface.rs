use echoisland_runtime::RuntimeSnapshot;

use crate::native_panel_core::PanelState;

use super::runtime_interaction::{
    NativePanelCoreStateBridge, NativePanelSettingsSurfaceSnapshotUpdate,
    NativePanelSettingsSurfaceToggleResult,
};

pub(crate) fn mutate_native_panel_core_state<S>(
    state: &mut S,
    mutate: impl FnOnce(&mut PanelState) -> bool,
) -> bool
where
    S: NativePanelCoreStateBridge,
{
    let mut core = state.snapshot_core_panel_state();
    let changed = mutate(&mut core);
    if changed {
        state.apply_core_panel_state(core);
    }
    changed
}

pub(crate) fn toggle_native_panel_settings_surface_for_state<S>(state: &mut S) -> bool
where
    S: NativePanelCoreStateBridge,
{
    mutate_native_panel_core_state(state, crate::native_panel_core::toggle_settings_surface)
}

pub(crate) fn toggle_native_panel_settings_surface_transition_request_for_state<S>(
    state: &mut S,
) -> NativePanelSettingsSurfaceToggleResult
where
    S: NativePanelCoreStateBridge,
{
    let changed = toggle_native_panel_settings_surface_for_state(state);
    let transition_request = if changed {
        let core = state.snapshot_core_panel_state();
        native_panel_settings_surface_transition_request(core.expanded)
    } else {
        None
    };
    NativePanelSettingsSurfaceToggleResult {
        changed,
        transition_request,
    }
}

pub(crate) fn resolve_native_panel_settings_surface_snapshot_update_for_state<S>(
    state: &mut S,
    snapshot: Option<RuntimeSnapshot>,
) -> Option<NativePanelSettingsSurfaceSnapshotUpdate>
where
    S: NativePanelCoreStateBridge,
{
    let result = toggle_native_panel_settings_surface_transition_request_for_state(state);
    result
        .changed
        .then_some(NativePanelSettingsSurfaceSnapshotUpdate {
            snapshot,
            transition_request: result.transition_request,
        })
}

fn native_panel_settings_surface_transition_request(
    expanded: bool,
) -> Option<super::transition_controller::NativePanelTransitionRequest> {
    // Settings clicks can arrive while the panel is already animating; keep this
    // as a surface transition so platform runtimes can queue it behind the
    // active open/close animation.
    expanded.then_some(super::transition_controller::NativePanelTransitionRequest::SurfaceSwitch)
}
