#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativePanelLightweightRefreshInput {
    pub(crate) transitioning: bool,
    pub(crate) animation_active: bool,
    pub(crate) active_count_marquee_needs_refresh: bool,
    pub(crate) mascot_animation_needs_refresh: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativePanelLightweightRefreshPlan {
    pub(crate) active_count_marquee: NativePanelLightweightRefreshChannelPlan,
    pub(crate) mascot_animation: NativePanelLightweightRefreshChannelPlan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativePanelLightweightRefreshChannelPlan {
    pub(crate) refresh_allowed: bool,
    pub(crate) reset_timer: bool,
}

pub(crate) fn resolve_native_panel_lightweight_refresh_plan(
    input: NativePanelLightweightRefreshInput,
) -> NativePanelLightweightRefreshPlan {
    let suspended = input.transitioning || input.animation_active;
    NativePanelLightweightRefreshPlan {
        active_count_marquee: resolve_native_panel_lightweight_refresh_channel(
            suspended,
            input.active_count_marquee_needs_refresh,
        ),
        mascot_animation: resolve_native_panel_lightweight_refresh_channel(
            suspended,
            input.mascot_animation_needs_refresh,
        ),
    }
}

fn resolve_native_panel_lightweight_refresh_channel(
    suspended: bool,
    needs_refresh: bool,
) -> NativePanelLightweightRefreshChannelPlan {
    NativePanelLightweightRefreshChannelPlan {
        refresh_allowed: !suspended && needs_refresh,
        reset_timer: suspended || !needs_refresh,
    }
}
