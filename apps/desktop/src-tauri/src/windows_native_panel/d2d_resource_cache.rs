use super::dpi::{WindowsDpiScale, WindowsPhysicalRect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct WindowsDirect2DResourceKey {
    pub(super) physical_rect: WindowsPhysicalRect,
    dpi_scale_millis: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct WindowsDirect2DResourceCacheState {
    current_key: Option<WindowsDirect2DResourceKey>,
    rebuild_count: usize,
}

impl WindowsDirect2DResourceKey {
    pub(super) fn new(physical_rect: WindowsPhysicalRect, dpi_scale: WindowsDpiScale) -> Self {
        Self {
            physical_rect,
            dpi_scale_millis: (dpi_scale.scale * 1000.0).round() as i32,
        }
    }
}

impl WindowsDirect2DResourceCacheState {
    pub(super) fn sync(&mut self, key: WindowsDirect2DResourceKey) -> bool {
        if self.current_key == Some(key) {
            return false;
        }
        self.current_key = Some(key);
        self.rebuild_count += 1;
        true
    }

    pub(super) fn rebuild_count(&self) -> usize {
        self.rebuild_count
    }
}
