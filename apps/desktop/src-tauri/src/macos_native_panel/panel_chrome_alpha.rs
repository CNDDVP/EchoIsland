use crate::native_panel_core::{
    PanelChromeVisibilitySpecInput, resolve_panel_chrome_visibility_spec,
};

pub(super) fn resolve_collapsed_chrome_alpha(input: PanelChromeVisibilitySpecInput) -> f64 {
    let chrome = resolve_panel_chrome_visibility_spec(input);
    if chrome.collapsed_mascot_visible {
        1.0 - chrome.collapsed_exit_progress.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
