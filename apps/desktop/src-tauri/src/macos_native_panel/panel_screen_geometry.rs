use objc2::MainThreadMarker;
use objc2_app_kit::{NSPanel, NSScreen};
use objc2_foundation::NSRect;

use super::panel_constants::{
    DEFAULT_COMPACT_PILL_HEIGHT, DEFAULT_COMPACT_PILL_WIDTH, DEFAULT_EXPANDED_PILL_WIDTH,
    DEFAULT_PANEL_CANVAS_WIDTH, EXPANDED_PILL_WIDTH_DELTA,
};

pub(super) fn compact_pill_height_for_screen(screen: &NSScreen) -> f64 {
    let safe_top = screen.safeAreaInsets().top;
    if safe_top > 0.0 {
        return safe_top;
    }

    let frame = screen.frame();
    let visible = screen.visibleFrame();
    let menu_bar_height =
        (frame.origin.y + frame.size.height) - (visible.origin.y + visible.size.height);
    if menu_bar_height > 5.0 {
        return menu_bar_height;
    }

    if let Some(mtm) = MainThreadMarker::new() {
        if let Some(main_screen) = NSScreen::mainScreen(mtm) {
            let main_frame = main_screen.frame();
            let main_visible = main_screen.visibleFrame();
            let main_menu = (main_frame.origin.y + main_frame.size.height)
                - (main_visible.origin.y + main_visible.size.height);
            if main_menu > 5.0 {
                return main_menu;
            }
        }
    }

    DEFAULT_COMPACT_PILL_HEIGHT
}

pub(super) fn screen_has_camera_housing(screen: &NSScreen) -> bool {
    crate::native_panel_core::resolve_panel_screen_has_camera_housing(panel_screen_top_area(screen))
}

#[cfg(test)]
pub(super) fn shell_width_for_non_camera_housing_screen(
    _screen_width: f64,
    compact_height: f64,
) -> f64 {
    crate::native_panel_core::resolve_panel_shell_width_for_non_camera_housing(
        compact_height,
        DEFAULT_COMPACT_PILL_WIDTH,
    )
}

#[cfg(test)]
pub(super) fn expanded_width_for_non_camera_housing_screen() -> f64 {
    DEFAULT_EXPANDED_PILL_WIDTH
}

#[cfg(test)]
pub(super) fn expanded_width_for_camera_housing_screen(compact_width: f64) -> f64 {
    crate::native_panel_core::resolve_panel_expanded_width_for_camera_housing(
        compact_width,
        EXPANDED_PILL_WIDTH_DELTA,
        DEFAULT_PANEL_CANVAS_WIDTH,
    )
}

pub(super) fn shell_width_for_screen(screen: &NSScreen, compact_height: f64) -> f64 {
    crate::native_panel_core::resolve_panel_shell_width(panel_screen_width_input(
        screen,
        compact_height,
    ))
}

pub(super) fn compact_pill_width_for_screen(screen: &NSScreen, compact_height: f64) -> f64 {
    shell_width_for_screen(screen, compact_height)
}

pub(super) fn compact_pill_height_for_screen_rect(
    screen: Option<&NSScreen>,
    fallback_rect: NSRect,
) -> f64 {
    screen
        .map(compact_pill_height_for_screen)
        .unwrap_or_else(|| {
            if fallback_rect.size.height > 0.0 {
                DEFAULT_COMPACT_PILL_HEIGHT
            } else {
                25.0
            }
        })
}

pub(super) fn compact_pill_width_for_screen_rect(
    screen: Option<&NSScreen>,
    compact_height: f64,
) -> f64 {
    let width_spec = island_width_spec_for_screen(screen);
    screen
        .map(|screen| compact_pill_width_for_screen(screen, compact_height))
        .unwrap_or(width_spec.compact_width)
}

pub(super) fn expanded_panel_width_for_screen(screen: &NSScreen) -> f64 {
    let compact_height = compact_pill_height_for_screen(screen);
    crate::native_panel_core::resolve_panel_expanded_width(panel_screen_width_input(
        screen,
        compact_height,
    ))
}

pub(super) fn expanded_panel_width_for_screen_rect(
    screen: Option<&NSScreen>,
    fallback_rect: NSRect,
) -> f64 {
    let width_spec = island_width_spec_for_screen(screen);
    screen
        .map(expanded_panel_width_for_screen)
        .unwrap_or_else(|| {
            crate::native_panel_core::resolve_fallback_panel_expanded_width(
                fallback_rect.size.width,
                width_spec.compact_width,
            )
        })
}

pub(super) fn panel_canvas_width_for_screen(screen: &NSScreen, compact_height: f64) -> f64 {
    crate::native_panel_core::resolve_panel_canvas_width(panel_screen_width_input(
        screen,
        compact_height,
    ))
}

pub(super) fn panel_canvas_width_for_screen_rect(
    screen: Option<&NSScreen>,
    compact_height: f64,
    fallback_rect: NSRect,
) -> f64 {
    let width_spec = island_width_spec_for_screen(screen);
    screen
        .map(|screen| panel_canvas_width_for_screen(screen, compact_height))
        .unwrap_or_else(|| {
            crate::native_panel_core::resolve_fallback_panel_canvas_width(
                fallback_rect.size.width,
                width_spec.canvas_width,
            )
        })
}

pub(super) fn resolve_screen_frame_for_panel(panel: &NSPanel) -> Option<NSRect> {
    if let Some(screen) = panel.screen() {
        return Some(screen.frame());
    }
    let mtm = MainThreadMarker::new()?;
    NSScreen::mainScreen(mtm)
        .or_else(|| {
            let screens = NSScreen::screens(mtm);
            if screens.is_empty() {
                None
            } else {
                Some(screens.objectAtIndex(0))
            }
        })
        .map(|screen| screen.frame())
}

fn panel_screen_top_area(screen: &NSScreen) -> crate::native_panel_core::PanelScreenTopArea {
    crate::native_panel_core::PanelScreenTopArea {
        screen_width: screen.frame().size.width,
        auxiliary_left_width: screen.auxiliaryTopLeftArea().size.width,
        auxiliary_right_width: screen.auxiliaryTopRightArea().size.width,
    }
}

fn panel_screen_width_input(
    screen: &NSScreen,
    compact_height: f64,
) -> crate::native_panel_core::PanelScreenWidthInput {
    let width_spec = island_width_spec_for_screen(Some(screen));
    crate::native_panel_core::PanelScreenWidthInput {
        top_area: panel_screen_top_area(screen),
        compact_height,
        default_compact_width: width_spec.compact_width,
        expanded_width_delta: width_spec.expanded_width - width_spec.compact_width,
        default_expanded_width: width_spec.expanded_width,
        default_canvas_width: width_spec.canvas_width,
    }
}

fn island_width_spec_for_screen(
    screen: Option<&NSScreen>,
) -> crate::native_panel_core::PanelIslandWidthSpec {
    let preset = crate::native_panel_core::effective_island_width_preset_for_display(
        crate::app_settings::current_app_settings().island_width_preset,
        screen.is_none_or(|screen| !screen_has_camera_housing(screen)),
    );
    crate::native_panel_core::island_width_spec(preset)
}
