use crate::{
    native_panel_core::{PanelPoint, PanelRect},
    native_panel_renderer::facade::descriptor::NativePanelPointerRegion,
};

const WINDOWS_BASE_DPI: u32 = 96;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WindowsDpiScale {
    pub(super) scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WindowsPhysicalRect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl Default for WindowsDpiScale {
    fn default() -> Self {
        Self::from_scale(1.0)
    }
}

impl WindowsDpiScale {
    pub(super) fn from_scale(scale: f64) -> Self {
        if scale.is_finite() && scale > 0.0 {
            Self { scale }
        } else {
            Self { scale: 1.0 }
        }
    }

    pub(super) fn from_dpi(dpi: u32) -> Self {
        if dpi == 0 {
            return Self::default();
        }
        Self::from_scale(dpi as f64 / WINDOWS_BASE_DPI as f64)
    }

    pub(super) fn logical_to_physical(self, value: f64) -> i32 {
        (value * self.scale).round() as i32
    }

    pub(super) fn physical_to_logical(self, value: i32) -> f64 {
        value as f64 / self.scale
    }

    pub(super) fn point_to_logical(self, x: i32, y: i32) -> PanelPoint {
        PanelPoint {
            x: self.physical_to_logical(x),
            y: self.physical_to_logical(y),
        }
    }

    pub(super) fn rect_to_physical(self, rect: PanelRect) -> WindowsPhysicalRect {
        WindowsPhysicalRect {
            x: self.logical_to_physical(rect.x),
            y: self.logical_to_physical(rect.y),
            width: self.logical_to_physical(rect.width),
            height: self.logical_to_physical(rect.height),
        }
    }

    pub(super) fn pointer_region_to_physical(
        self,
        region: &NativePanelPointerRegion,
    ) -> WindowsPhysicalRect {
        self.rect_to_physical(region.frame)
    }
}

#[cfg(any(windows, test))]
fn panel_dpi_scales() -> &'static std::sync::Mutex<std::collections::HashMap<isize, WindowsDpiScale>>
{
    static SCALES: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<isize, WindowsDpiScale>>,
    > = std::sync::OnceLock::new();
    SCALES.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

#[cfg(any(windows, test))]
fn replace_windows_panel_dpi_scale(
    hwnd: isize,
    scale: Option<WindowsDpiScale>,
) -> Option<WindowsDpiScale> {
    let mut scales = panel_dpi_scales().lock().ok()?;
    if let Some(scale) = scale {
        scales.insert(hwnd, scale)
    } else {
        scales.remove(&hwnd)
    }
}

#[cfg(all(windows, not(test)))]
pub(super) fn set_windows_panel_dpi_scale(hwnd: isize, scale: Option<WindowsDpiScale>) {
    let _ = replace_windows_panel_dpi_scale(hwnd, scale);
}

/// Publish the target DPI before native positioning can dispatch pointer
/// messages, but restore the old cache if positioning fails. The cache lock is
/// released before calling native code, whose synchronous messages read it.
#[cfg(any(windows, test))]
pub(super) fn with_windows_panel_dpi_change<T, E>(
    hwnd: isize,
    target_scale: WindowsDpiScale,
    position_window: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let previous_scale = replace_windows_panel_dpi_scale(hwnd, Some(target_scale));
    let result = position_window();
    if result.is_err() {
        let _ = replace_windows_panel_dpi_scale(hwnd, previous_scale);
    }
    result
}

#[cfg(all(windows, not(test)))]
pub(super) fn resolve_windows_dpi_scale_for_window(
    raw_window_handle: Option<isize>,
) -> WindowsDpiScale {
    use windows::Win32::{Foundation::HWND, UI::HiDpi::GetDpiForWindow};

    let Some(hwnd) = raw_window_handle else {
        return WindowsDpiScale::default();
    };
    if let Some(scale) = panel_dpi_scales()
        .lock()
        .ok()
        .and_then(|scales| scales.get(&hwnd).copied())
    {
        return scale;
    }
    let dpi = unsafe { GetDpiForWindow(HWND(hwnd as _)) };
    WindowsDpiScale::from_dpi(dpi)
}

#[cfg(any(not(windows), test))]
pub(super) fn resolve_windows_dpi_scale_for_window(
    _raw_window_handle: Option<isize>,
) -> WindowsDpiScale {
    WindowsDpiScale::default()
}

#[cfg(test)]
mod tests {
    use super::{WindowsDpiScale, WindowsPhysicalRect, resolve_windows_dpi_scale_for_window};
    use crate::{
        native_panel_core::PanelRect,
        native_panel_renderer::facade::descriptor::{
            NativePanelPointerRegion, NativePanelPointerRegionKind,
        },
    };

    #[test]
    fn dpi_scale_maps_logical_rect_at_100_percent() {
        let scale = WindowsDpiScale::from_scale(1.0);

        assert_eq!(
            scale.rect_to_physical(PanelRect {
                x: 0.0,
                y: 0.0,
                width: 253.0,
                height: 80.0,
            }),
            WindowsPhysicalRect {
                x: 0,
                y: 0,
                width: 253,
                height: 80,
            }
        );
    }

    #[test]
    fn dpi_scale_rounds_logical_rect_at_125_percent() {
        let scale = WindowsDpiScale::from_scale(1.25);

        assert_eq!(
            scale.rect_to_physical(PanelRect {
                x: 10.0,
                y: 20.0,
                width: 253.0,
                height: 80.0,
            }),
            WindowsPhysicalRect {
                x: 13,
                y: 25,
                width: 316,
                height: 100,
            }
        );
    }

    #[test]
    fn dpi_scale_maps_physical_point_back_to_logical() {
        let scale = WindowsDpiScale::from_scale(1.25);

        assert_eq!(
            scale.point_to_logical(150, 75),
            crate::native_panel_core::PanelPoint { x: 120.0, y: 60.0 }
        );
    }

    #[test]
    fn dpi_scale_uses_same_conversion_for_window_and_hit_regions() {
        let scale = WindowsDpiScale::from_scale(1.5);
        let frame = PanelRect {
            x: 20.0,
            y: 8.0,
            width: 265.0,
            height: 80.0,
        };
        let region = NativePanelPointerRegion {
            frame,
            kind: NativePanelPointerRegionKind::CompactBar,
        };

        assert_eq!(
            scale.pointer_region_to_physical(&region),
            scale.rect_to_physical(frame)
        );
    }

    #[test]
    fn dpi_scale_preserves_negative_monitor_origins() {
        let scale = WindowsDpiScale::from_scale(1.25);

        assert_eq!(
            scale.rect_to_physical(PanelRect {
                x: -1280.0,
                y: -16.0,
                width: 253.0,
                height: 80.0,
            }),
            WindowsPhysicalRect {
                x: -1600,
                y: -20,
                width: 316,
                height: 100,
            }
        );
    }

    #[test]
    fn dpi_scale_from_window_defaults_to_100_percent_in_tests() {
        assert_eq!(
            resolve_windows_dpi_scale_for_window(Some(1)),
            WindowsDpiScale::from_scale(1.0)
        );
    }

    #[test]
    fn failed_positioning_restores_previous_dpi_after_synchronous_pointer_query() {
        let hwnd = -1201;
        let old = WindowsDpiScale::from_scale(1.5);
        let target = WindowsDpiScale::from_scale(1.0);
        super::replace_windows_panel_dpi_scale(hwnd, Some(old));
        let result = super::with_windows_panel_dpi_change(hwnd, target, || {
            // Model the synchronous pointer event sent from inside SetWindowPos.
            let during = super::panel_dpi_scales().lock().unwrap()[&hwnd];
            assert_eq!(
                during.point_to_logical(120, 24),
                crate::native_panel_core::PanelPoint { x: 120.0, y: 24.0 }
            );
            Err::<(), _>("injected SetWindowPos failure")
        });
        assert_eq!(result, Err("injected SetWindowPos failure"));
        let after = super::panel_dpi_scales().lock().unwrap()[&hwnd];
        assert_eq!(after, old);
        assert_eq!(
            after.point_to_logical(180, 36),
            crate::native_panel_core::PanelPoint { x: 120.0, y: 24.0 }
        );
        super::replace_windows_panel_dpi_scale(hwnd, None);
    }

    #[test]
    fn failed_first_positioning_removes_provisional_dpi_cache() {
        let hwnd = -1202;
        super::replace_windows_panel_dpi_scale(hwnd, None);
        let result =
            super::with_windows_panel_dpi_change(hwnd, WindowsDpiScale::from_scale(2.0), || {
                Err::<(), _>("failure")
            });
        assert!(result.is_err());
        assert!(
            !super::panel_dpi_scales()
                .lock()
                .unwrap()
                .contains_key(&hwnd)
        );
    }

    #[test]
    fn successful_positioning_keeps_target_dpi_cache() {
        let hwnd = -1203;
        super::replace_windows_panel_dpi_scale(hwnd, Some(WindowsDpiScale::from_scale(2.0)));
        let target = WindowsDpiScale::from_scale(1.25);
        let result =
            super::with_windows_panel_dpi_change(hwnd, target, || Ok::<_, &str>("positioned"));
        assert_eq!(result, Ok("positioned"));
        assert_eq!(super::panel_dpi_scales().lock().unwrap()[&hwnd], target);
        super::replace_windows_panel_dpi_scale(hwnd, None);
    }
}
