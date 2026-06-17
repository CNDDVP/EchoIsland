use crate::native_panel_renderer::facade::visual::{
    NativePanelVisualColor, NativePanelVisualPlan, NativePanelVisualPrimitive,
    NativePanelVisualTextAlignment, NativePanelVisualTextRole, NativePanelVisualTextWeight,
    resolve_native_panel_visual_plan,
};

use crate::native_panel_core::{PanelPoint, PanelRect};
use crate::native_panel_scene::SceneMascotPose;

use super::{
    d2d_painter::WindowsNativePanelPainter, window_shell::WindowsNativePanelShellPaintJob,
};

#[cfg(all(windows, not(test)))]
thread_local! {
    static DIRECT2D_WINDOWS_NATIVE_PANEL_PAINTER:
        std::cell::RefCell<Option<super::d2d_painter::Direct2DWindowsNativePanelPainter>> =
            const { std::cell::RefCell::new(None) };
}

pub(super) const WINDOWS_NATIVE_PANEL_TRANSPARENT_COLOR_KEY: u32 = 0x00FF00FF;

pub(super) type WindowsNativePanelPaintColor = NativePanelVisualColor;
pub(super) type WindowsNativePanelPaintPlan = NativePanelVisualPlan;
pub(super) type WindowsNativePanelPaintPrimitive = NativePanelVisualPrimitive;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WindowsNativePanelPainterBackend {
    Direct2D,
    GdiFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum WindowsNativePanelPaintOperation {
    PushClip {
        frame: PanelRect,
    },
    PopClip,
    DrawCompletionGlowImage {
        frame: PanelRect,
        opacity: f64,
    },
    DrawMascotSprite {
        sprite_path: String,
        source_rect: PanelRect,
        frame: PanelRect,
        opacity: f64,
    },
    FillHitTestBlocker {
        frame: PanelRect,
        alpha: u8,
    },
    FillRoundRect {
        frame: PanelRect,
        radius: f64,
        color: WindowsNativePanelPaintColor,
        alpha: f64,
    },
    FillRect {
        frame: PanelRect,
        color: WindowsNativePanelPaintColor,
        alpha: f64,
    },
    FillEllipse {
        frame: PanelRect,
        color: WindowsNativePanelPaintColor,
        alpha: f64,
    },
    StrokeLine {
        from: PanelPoint,
        to: PanelPoint,
        color: WindowsNativePanelPaintColor,
        width: i32,
        alpha: f64,
    },
    DrawText {
        role: NativePanelVisualTextRole,
        origin: PanelPoint,
        max_width: f64,
        text: String,
        color: WindowsNativePanelPaintColor,
        size: i32,
        weight: NativePanelVisualTextWeight,
        alignment: NativePanelVisualTextAlignment,
        alpha: f64,
    },
    FillMascotDot {
        frame: PanelRect,
        radius: f64,
        pose: SceneMascotPose,
        color: WindowsNativePanelPaintColor,
        stroke_color: WindowsNativePanelPaintColor,
        stroke_width: f64,
        shadow_opacity: f64,
        shadow_radius: f64,
        alpha: f64,
    },
    FillCompactShoulder {
        frame: PanelRect,
        side: crate::native_panel_renderer::facade::visual::NativePanelVisualShoulderSide,
        progress: f64,
        fill: WindowsNativePanelPaintColor,
        border: WindowsNativePanelPaintColor,
    },
}

const WINDOWS_NATIVE_PANEL_HIT_TEST_BLOCKER_ALPHA: u8 = 1;

pub(super) fn resolve_windows_native_panel_paint_plan(
    job: &WindowsNativePanelShellPaintJob,
) -> WindowsNativePanelPaintPlan {
    resolve_native_panel_visual_plan(job)
}

pub(super) fn windows_native_panel_preferred_painter_backend() -> WindowsNativePanelPainterBackend {
    WindowsNativePanelPainterBackend::Direct2D
}

pub(super) fn windows_native_panel_composition_mode_for_preferred_painter()
-> super::layered_window::WindowsLayeredWindowCompositionMode {
    match windows_native_panel_preferred_painter_backend() {
        WindowsNativePanelPainterBackend::Direct2D => {
            super::layered_window::WindowsLayeredWindowCompositionMode::PerPixelAlpha
        }
        WindowsNativePanelPainterBackend::GdiFallback => {
            super::layered_window::WindowsLayeredWindowCompositionMode::GdiColorKeyFallback
        }
    }
}

pub(super) fn resolve_windows_native_panel_paint_operations(
    plan: &WindowsNativePanelPaintPlan,
) -> Vec<WindowsNativePanelPaintOperation> {
    if plan.hidden {
        return Vec::new();
    }

    plan.primitives
        .iter()
        .map(windows_native_panel_paint_operation_from_primitive)
        .collect()
}

pub(super) fn resolve_windows_native_panel_hit_test_blocker_operations(
    job: &WindowsNativePanelShellPaintJob,
) -> Vec<WindowsNativePanelPaintOperation> {
    if job.display_mode
        != crate::native_panel_renderer::facade::presentation::NativePanelVisualDisplayMode::Expanded
    {
        return Vec::new();
    }

    let mut operations = Vec::new();
    push_hit_test_blocker_operation(&mut operations, job.shell_frame);

    let content_top = job.content_frame.y + job.content_frame.height;
    let gap_y = job.shell_frame.y + job.shell_frame.height;
    push_hit_test_blocker_operation(
        &mut operations,
        PanelRect {
            x: job.shell_frame.x,
            y: gap_y,
            width: job.shell_frame.width,
            height: (content_top - gap_y).max(0.0),
        },
    );

    operations
}

fn push_hit_test_blocker_operation(
    operations: &mut Vec<WindowsNativePanelPaintOperation>,
    frame: PanelRect,
) {
    if frame.width <= 0.0 || frame.height <= 0.0 {
        return;
    }
    operations.push(WindowsNativePanelPaintOperation::FillHitTestBlocker {
        frame,
        alpha: WINDOWS_NATIVE_PANEL_HIT_TEST_BLOCKER_ALPHA,
    });
}

fn windows_native_panel_paint_operation_from_primitive(
    primitive: &WindowsNativePanelPaintPrimitive,
) -> WindowsNativePanelPaintOperation {
    match primitive {
        WindowsNativePanelPaintPrimitive::ClipStart { frame } => {
            WindowsNativePanelPaintOperation::PushClip { frame: *frame }
        }
        WindowsNativePanelPaintPrimitive::ClipEnd => WindowsNativePanelPaintOperation::PopClip,
        WindowsNativePanelPaintPrimitive::CompletionGlow { frame, opacity } => {
            WindowsNativePanelPaintOperation::DrawCompletionGlowImage {
                frame: *frame,
                opacity: *opacity,
            }
        }
        WindowsNativePanelPaintPrimitive::RoundRect {
            frame,
            radius,
            color,
        } => WindowsNativePanelPaintOperation::FillRoundRect {
            frame: *frame,
            radius: *radius,
            color: *color,
            alpha: 1.0,
        },
        WindowsNativePanelPaintPrimitive::Rect { frame, color } => {
            WindowsNativePanelPaintOperation::FillRect {
                frame: *frame,
                color: *color,
                alpha: 1.0,
            }
        }
        WindowsNativePanelPaintPrimitive::Ellipse { frame, color } => {
            WindowsNativePanelPaintOperation::FillEllipse {
                frame: *frame,
                color: *color,
                alpha: 1.0,
            }
        }
        WindowsNativePanelPaintPrimitive::MascotRoundRect {
            frame,
            radius,
            color,
            alpha,
            ..
        } => WindowsNativePanelPaintOperation::FillRoundRect {
            frame: *frame,
            radius: *radius,
            color: *color,
            alpha: *alpha,
        },
        WindowsNativePanelPaintPrimitive::MascotEllipse {
            frame,
            color,
            alpha,
            ..
        } => WindowsNativePanelPaintOperation::FillEllipse {
            frame: *frame,
            color: *color,
            alpha: *alpha,
        },
        WindowsNativePanelPaintPrimitive::StrokeLine {
            from,
            to,
            color,
            width,
        } => WindowsNativePanelPaintOperation::StrokeLine {
            from: *from,
            to: *to,
            color: *color,
            width: *width,
            alpha: 1.0,
        },
        WindowsNativePanelPaintPrimitive::Text {
            role,
            origin,
            max_width,
            text,
            color,
            size,
            weight,
            alignment,
            alpha,
        } => WindowsNativePanelPaintOperation::DrawText {
            role: *role,
            origin: *origin,
            max_width: *max_width,
            text: text.clone(),
            color: *color,
            size: *size,
            weight: *weight,
            alignment: *alignment,
            alpha: *alpha,
        },
        WindowsNativePanelPaintPrimitive::MascotText {
            origin,
            max_width,
            text,
            color,
            size,
            weight,
            alignment,
            alpha,
            ..
        } => WindowsNativePanelPaintOperation::DrawText {
            role: NativePanelVisualTextRole::Unspecified,
            origin: *origin,
            max_width: *max_width,
            text: text.clone(),
            color: *color,
            size: *size,
            weight: *weight,
            alignment: *alignment,
            alpha: *alpha,
        },
        WindowsNativePanelPaintPrimitive::MascotDot {
            frame,
            corner_radius,
            pose,
            fill,
            stroke,
            stroke_width,
            shadow_opacity,
            shadow_radius,
            alpha,
            ..
        } => WindowsNativePanelPaintOperation::FillMascotDot {
            frame: *frame,
            radius: *corner_radius,
            pose: *pose,
            color: *fill,
            stroke_color: *stroke,
            stroke_width: *stroke_width,
            shadow_opacity: *shadow_opacity,
            shadow_radius: *shadow_radius,
            alpha: *alpha,
        },
        WindowsNativePanelPaintPrimitive::MascotSprite {
            sprite_path,
            source_rect,
            frame,
            opacity,
        } => WindowsNativePanelPaintOperation::DrawMascotSprite {
            sprite_path: sprite_path.clone(),
            source_rect: *source_rect,
            frame: *frame,
            opacity: *opacity,
        },
        WindowsNativePanelPaintPrimitive::CompactShoulder {
            frame,
            side,
            progress,
            fill,
            border,
        } => WindowsNativePanelPaintOperation::FillCompactShoulder {
            frame: *frame,
            side: *side,
            progress: *progress,
            fill: *fill,
            border: *border,
        },
    }
}

pub(super) fn paint_windows_native_panel_job(
    raw_window_handle: Option<isize>,
    job: &WindowsNativePanelShellPaintJob,
) -> Result<WindowsNativePanelPaintPlan, String> {
    #[cfg(all(windows, not(test)))]
    {
        match windows_native_panel_preferred_painter_backend() {
            WindowsNativePanelPainterBackend::Direct2D => {
                paint_windows_native_panel_job_with_direct2d(raw_window_handle, job)
            }
            WindowsNativePanelPainterBackend::GdiFallback => {
                let mut painter =
                    super::d2d_painter::GdiWindowsNativePanelPainter::new(raw_window_handle);
                painter.paint(job)
            }
        }
    }

    #[cfg(any(not(windows), test))]
    {
        let _ = raw_window_handle;
        let mut painter = super::d2d_painter::PlanOnlyWindowsNativePanelPainter;
        painter.paint(job)
    }
}

#[cfg(all(windows, not(test)))]
fn paint_windows_native_panel_job_with_direct2d(
    raw_window_handle: Option<isize>,
    job: &WindowsNativePanelShellPaintJob,
) -> Result<WindowsNativePanelPaintPlan, String> {
    DIRECT2D_WINDOWS_NATIVE_PANEL_PAINTER.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(super::d2d_painter::Direct2DWindowsNativePanelPainter::new(
                raw_window_handle,
            )?);
        }
        let painter = slot
            .as_mut()
            .expect("Direct2D painter initialized when slot is Some");
        painter.set_raw_window_handle(raw_window_handle);
        painter.paint(job)
    })
}

#[cfg(all(windows, not(test)))]
pub(super) fn paint_windows_native_panel_job_with_gdi(
    raw_window_handle: Option<isize>,
    job: &WindowsNativePanelShellPaintJob,
) -> Result<WindowsNativePanelPaintPlan, String> {
    use std::iter;
    use windows_sys::Win32::{
        Foundation::RECT,
        Graphics::Gdi::{
            CreatePen, CreateSolidBrush, DT_END_ELLIPSIS, DT_SINGLELINE, DT_VCENTER, DeleteObject,
            DrawTextW, Ellipse, FillRect, GetDC, GetStockObject, IntersectClipRect, LineTo,
            MoveToEx, NULL_PEN, PS_SOLID, ReleaseDC, RestoreDC, RoundRect, SaveDC, SelectObject,
            SetBkMode, SetTextColor, TRANSPARENT,
        },
        UI::WindowsAndMessaging::GetClientRect,
    };

    let plan = resolve_windows_native_panel_paint_plan(job);
    let operations = resolve_windows_native_panel_paint_operations(&plan);
    let Some(hwnd) = raw_window_handle else {
        return Ok(plan);
    };

    unsafe {
        let hdc = GetDC(hwnd as _);
        if hdc.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }

        let clear_brush = CreateSolidBrush(WINDOWS_NATIVE_PANEL_TRANSPARENT_COLOR_KEY);
        let mut client_rect = std::mem::zeroed::<RECT>();
        if GetClientRect(hwnd as _, &mut client_rect) != 0 {
            let _ = FillRect(hdc, &client_rect, clear_brush);
        }
        let _ = DeleteObject(clear_brush as _);

        for operation in &operations {
            match operation {
                WindowsNativePanelPaintOperation::PushClip { frame } => {
                    let _ = SaveDC(hdc);
                    let _ = IntersectClipRect(
                        hdc,
                        frame.x.round() as i32,
                        frame.y.round() as i32,
                        (frame.x + frame.width).round() as i32,
                        (frame.y + frame.height).round() as i32,
                    );
                }
                WindowsNativePanelPaintOperation::PopClip => {
                    let _ = RestoreDC(hdc, -1);
                }
                WindowsNativePanelPaintOperation::DrawCompletionGlowImage { .. } => {}
                WindowsNativePanelPaintOperation::DrawMascotSprite { .. } => {}
                WindowsNativePanelPaintOperation::FillHitTestBlocker { .. } => {}
                WindowsNativePanelPaintOperation::FillRoundRect {
                    frame,
                    radius,
                    color,
                    ..
                } => {
                    let brush = CreateSolidBrush(color_ref(*color));
                    let pen = GetStockObject(NULL_PEN);
                    let previous = SelectObject(hdc, brush as _);
                    let previous_pen = SelectObject(hdc, pen);
                    let _ = RoundRect(
                        hdc,
                        frame.x.round() as i32,
                        frame.y.round() as i32,
                        (frame.x + frame.width).round() as i32,
                        (frame.y + frame.height).round() as i32,
                        (radius * 2.0).round() as i32,
                        (radius * 2.0).round() as i32,
                    );
                    let _ = SelectObject(hdc, previous_pen);
                    let _ = SelectObject(hdc, previous);
                    let _ = DeleteObject(brush as _);
                }
                WindowsNativePanelPaintOperation::FillRect { frame, color, .. } => {
                    let brush = CreateSolidBrush(color_ref(*color));
                    let rect = rect_from_panel_rect(*frame);
                    let _ = FillRect(hdc, &rect, brush);
                    let _ = DeleteObject(brush as _);
                }
                WindowsNativePanelPaintOperation::FillEllipse { frame, color, .. } => {
                    let brush = CreateSolidBrush(color_ref(*color));
                    let pen = GetStockObject(NULL_PEN);
                    let previous = SelectObject(hdc, brush as _);
                    let previous_pen = SelectObject(hdc, pen);
                    let _ = Ellipse(
                        hdc,
                        frame.x.round() as i32,
                        frame.y.round() as i32,
                        (frame.x + frame.width).round() as i32,
                        (frame.y + frame.height).round() as i32,
                    );
                    let _ = SelectObject(hdc, previous_pen);
                    let _ = SelectObject(hdc, previous);
                    let _ = DeleteObject(brush as _);
                }
                WindowsNativePanelPaintOperation::StrokeLine {
                    from,
                    to,
                    color,
                    width,
                    ..
                } => {
                    let pen = CreatePen(PS_SOLID, *width, color_ref(*color));
                    let previous = SelectObject(hdc, pen as _);
                    let _ = MoveToEx(
                        hdc,
                        from.x.round() as i32,
                        from.y.round() as i32,
                        std::ptr::null_mut(),
                    );
                    let _ = LineTo(hdc, to.x.round() as i32, to.y.round() as i32);
                    let _ = SelectObject(hdc, previous);
                    let _ = DeleteObject(pen as _);
                }
                WindowsNativePanelPaintOperation::DrawText {
                    role,
                    origin,
                    max_width,
                    text,
                    color,
                    size,
                    alignment,
                    ..
                } => {
                    let _ = SetBkMode(hdc, TRANSPARENT as i32);
                    let _ = SetTextColor(hdc, color_ref(*color));
                    let mut wide: Vec<u16> = text.encode_utf16().chain(iter::once(0)).collect();
                    let mut rect = RECT {
                        left: origin.x.round() as i32,
                        top: origin.y.round() as i32,
                        right: (origin.x + max_width).round() as i32,
                        bottom: (origin.y
                            + crate::native_panel_renderer::facade::visual::native_panel_visual_text_box_height_for_role(
                                *role, text, *size,
                            ))
                        .round() as i32,
                    };
                    let _ = DrawTextW(
                        hdc,
                        wide.as_mut_ptr(),
                        -1,
                        &mut rect,
                        DT_SINGLELINE
                            | DT_VCENTER
                            | gdi_text_alignment(*alignment)
                            | DT_END_ELLIPSIS,
                    );
                }
                WindowsNativePanelPaintOperation::FillMascotDot {
                    frame,
                    radius,
                    color,
                    stroke_color,
                    stroke_width,
                    ..
                } => {
                    let brush = CreateSolidBrush(color_ref(*color));
                    let pen = CreatePen(
                        PS_SOLID,
                        stroke_width.round().max(1.0) as i32,
                        color_ref(*stroke_color),
                    );
                    let previous = SelectObject(hdc, brush as _);
                    let previous_pen = SelectObject(hdc, pen as _);
                    let _ = RoundRect(
                        hdc,
                        frame.x.round() as i32,
                        frame.y.round() as i32,
                        (frame.x + frame.width).round() as i32,
                        (frame.y + frame.height).round() as i32,
                        (radius * 2.0).round() as i32,
                        (radius * 2.0).round() as i32,
                    );
                    let _ = SelectObject(hdc, previous_pen);
                    let _ = SelectObject(hdc, previous);
                    let _ = DeleteObject(pen as _);
                    let _ = DeleteObject(brush as _);
                }
                WindowsNativePanelPaintOperation::FillCompactShoulder { frame, fill, .. } => {
                    let brush = CreateSolidBrush(color_ref(*fill));
                    let rect = rect_from_panel_rect(*frame);
                    let _ = FillRect(hdc, &rect, brush);
                    let _ = DeleteObject(brush as _);
                }
            }
        }

        let _ = ReleaseDC(hwnd as _, hdc);
    }

    Ok(plan)
}

#[cfg(all(windows, not(test)))]
fn color_ref(color: WindowsNativePanelPaintColor) -> u32 {
    color.r as u32 | ((color.g as u32) << 8) | ((color.b as u32) << 16)
}

#[cfg(all(windows, not(test)))]
fn rect_from_panel_rect(rect: PanelRect) -> windows_sys::Win32::Foundation::RECT {
    windows_sys::Win32::Foundation::RECT {
        left: rect.x.round() as i32,
        top: rect.y.round() as i32,
        right: (rect.x + rect.width).round() as i32,
        bottom: (rect.y + rect.height).round() as i32,
    }
}

#[cfg(all(windows, not(test)))]
fn gdi_text_alignment(alignment: NativePanelVisualTextAlignment) -> u32 {
    use windows_sys::Win32::Graphics::Gdi::{DT_CENTER, DT_LEFT, DT_RIGHT};

    match alignment {
        NativePanelVisualTextAlignment::Left => DT_LEFT,
        NativePanelVisualTextAlignment::Center => DT_CENTER,
        NativePanelVisualTextAlignment::Right => DT_RIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        WindowsNativePanelPaintColor, WindowsNativePanelPaintOperation,
        WindowsNativePanelPaintPrimitive, resolve_windows_native_panel_hit_test_blocker_operations,
        resolve_windows_native_panel_paint_operations, resolve_windows_native_panel_paint_plan,
        windows_native_panel_paint_operation_from_primitive,
    };
    use crate::{
        native_panel_core::{ExpandedSurface, PanelRect},
        native_panel_renderer::facade::{
            descriptor::{NativePanelEdgeAction, NativePanelHostWindowState},
            presentation::{
                NativePanelVisualCardBadgeInput, NativePanelVisualCardInput,
                NativePanelVisualCardRowInput, NativePanelVisualDisplayMode,
            },
            visual::{NativePanelVisualTextRole, native_panel_visual_text_primitive_by_text},
        },
        native_panel_scene::SceneMascotPose,
        windows_native_panel::window_shell::{
            WindowsNativePanelShellActionButtonPaintInput, WindowsNativePanelShellPaintJob,
        },
    };

    fn paint_job(display_mode: NativePanelVisualDisplayMode) -> WindowsNativePanelShellPaintJob {
        let compact_bar_width = if display_mode == NativePanelVisualDisplayMode::Expanded {
            crate::native_panel_core::DEFAULT_EXPANDED_PILL_WIDTH
        } else {
            240.0
        };
        WindowsNativePanelShellPaintJob {
            window_state: NativePanelHostWindowState {
                frame: Some(PanelRect {
                    x: 100.0,
                    y: 20.0,
                    width: 320.0,
                    height: 160.0,
                }),
                visible: display_mode != NativePanelVisualDisplayMode::Hidden,
                preferred_display_index: 0,
            },
            display_mode,
            surface: ExpandedSurface::Default,
            panel_frame: PanelRect {
                x: 100.0,
                y: 20.0,
                width: 320.0,
                height: 160.0,
            },
            compact_bar_frame: PanelRect {
                x: (320.0 - compact_bar_width) / 2.0,
                y: 12.0,
                width: compact_bar_width,
                height: 36.0,
            },
            left_shoulder_frame: PanelRect {
                x: 34.0,
                y: 42.0,
                width: 6.0,
                height: 6.0,
            },
            right_shoulder_frame: PanelRect {
                x: 280.0,
                y: 42.0,
                width: 6.0,
                height: 6.0,
            },
            shoulder_progress: 0.0,
            content_frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 160.0,
            },
            card_stack_frame: PanelRect {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 160.0,
            },
            card_stack_content_height: 160.0,
            shell_frame: PanelRect {
                x: 20.0,
                y: 0.0,
                width: 280.0,
                height: 150.0,
            },
            headline_text: "Codex ready".to_string(),
            headline_emphasized: false,
            active_count: "1".to_string(),
            active_count_elapsed_ms: 0,
            total_count: "3".to_string(),
            separator_visibility: 0.5,
            chrome_transition_progress: if display_mode == NativePanelVisualDisplayMode::Expanded {
                1.0
            } else {
                0.0
            },
            cards_visible: true,
            card_count: 2,
            cards: vec![
                NativePanelVisualCardInput {
                    style: crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Settings,
                    title: "Settings".to_string(),
                    subtitle: Some("EchoIsland v0.6.1".to_string()),
                    body: None,
                    badge: None,
                    source_badge: None,
                    body_prefix: None,
                    body_lines: Vec::new(),
                    action_hint: None,
                    rows: vec![NativePanelVisualCardRowInput {
                        title: "Mute Sound".to_string(),
                        value: "Off".to_string(),
                        active: true,
                    }],
                    height: 92.0,
                    collapsed_height: 64.0,
                    compact: false,
                    removing: false,
                },
                NativePanelVisualCardInput {
                    style: crate::native_panel_renderer::facade::presentation::NativePanelVisualCardStyle::Completion,
                    title: "Done".to_string(),
                    subtitle: Some("#abcdef 璺?now".to_string()),
                    body: Some("Task complete".to_string()),
                    badge: Some(NativePanelVisualCardBadgeInput {
                        text: "Done".to_string(),
                        emphasized: true,
                    }),
                    source_badge: Some(NativePanelVisualCardBadgeInput {
                        text: "Codex".to_string(),
                        emphasized: false,
                    }),
                    body_prefix: Some("$".to_string()),
                    body_lines: Vec::new(),
                    action_hint: None,
                    rows: Vec::new(),
                    height: 76.0,
                    collapsed_height: 52.0,
                    compact: false,
                    removing: false,
                },
            ],
            glow_visible: true,
            glow_opacity: 0.78,
            action_buttons_visible: true,
            action_buttons: vec![
                WindowsNativePanelShellActionButtonPaintInput {
                    action: NativePanelEdgeAction::Settings,
                    frame: PanelRect {
                        x: 250.0,
                        y: 20.0,
                        width: 18.0,
                        height: 18.0,
                    },
                    debug_mode_enabled: false,
                },
                WindowsNativePanelShellActionButtonPaintInput {
                    action: NativePanelEdgeAction::Quit,
                    frame: PanelRect {
                        x: 280.0,
                        y: 20.0,
                        width: 18.0,
                        height: 18.0,
                    },
                    debug_mode_enabled: false,
                },
            ],
            completion_count: 2,
            mascot_elapsed_ms: 0,
            mascot_motion_frame: None,
            mascot_pose: SceneMascotPose::Complete,
            mascot_debug_mode_enabled: false,
        }
    }

    #[test]
    fn paint_plan_is_empty_when_hidden() {
        let plan = resolve_windows_native_panel_paint_plan(&paint_job(
            NativePanelVisualDisplayMode::Hidden,
        ));

        assert!(plan.hidden);
        assert!(plan.primitives.is_empty());
    }

    #[test]
    fn paint_plan_contains_visible_panel_primitives() {
        let plan = resolve_windows_native_panel_paint_plan(&paint_job(
            NativePanelVisualDisplayMode::Expanded,
        ));

        assert!(!plan.hidden);
        assert!(native_panel_visual_text_primitive_by_text(&plan, "Codex ready").is_some());
        assert!(plan.primitives.iter().any(|primitive| matches!(
            primitive,
            WindowsNativePanelPaintPrimitive::RoundRect { .. }
        )));
        assert!(native_panel_visual_text_primitive_by_text(&plan, "\u{E713}").is_some());
        assert!(native_panel_visual_text_primitive_by_text(&plan, "\u{E7E8}").is_some());
        assert!(!plan.primitives.iter().any(|primitive| matches!(
            primitive,
            WindowsNativePanelPaintPrimitive::MascotDot { .. }
                | WindowsNativePanelPaintPrimitive::MascotEllipse { .. }
        )));
        assert!(native_panel_visual_text_primitive_by_text(&plan, "Mute Sound").is_some());
    }

    #[test]
    fn paint_operations_cover_visual_primitives_without_rebuilding_scene_semantics() {
        let plan = resolve_windows_native_panel_paint_plan(&paint_job(
            NativePanelVisualDisplayMode::Expanded,
        ));

        let operations = resolve_windows_native_panel_paint_operations(&plan);

        assert_eq!(operations.len(), plan.primitives.len());
        assert!(!operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::DrawCompletionGlowImage { .. }
        )));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::DrawText { text, .. } if text == "Codex ready"
        )));
        assert!(!operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::FillMascotDot { .. }
        )));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::FillRoundRect { .. }
        )));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::DrawText { text, .. } if text == "\u{E713}"
        )));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::DrawText { text, .. } if text == "\u{E7E8}"
        )));
    }

    #[test]
    fn text_paint_operation_preserves_card_text_role_for_platform_text_metrics() {
        let operation = windows_native_panel_paint_operation_from_primitive(
            &WindowsNativePanelPaintPrimitive::Text {
                role: NativePanelVisualTextRole::CardStatusBadge,
                origin: crate::native_panel_core::PanelPoint { x: 12.0, y: 34.0 },
                max_width: 48.0,
                text: "Idle".to_string(),
                color: crate::native_panel_renderer::facade::visual::NativePanelVisualColor::rgb(
                    230, 235, 245,
                ),
                size: 10,
                weight: crate::native_panel_renderer::facade::visual::NativePanelVisualTextWeight::Semibold,
                alignment: crate::native_panel_renderer::facade::visual::NativePanelVisualTextAlignment::Center,
                alpha: 1.0,
            },
        );

        assert!(matches!(
            operation,
            WindowsNativePanelPaintOperation::DrawText {
                role: NativePanelVisualTextRole::CardStatusBadge,
                text,
                ..
            } if text == "Idle"
        ));
    }

    #[test]
    fn expanded_paint_adds_nearly_transparent_hit_test_blockers_for_alpha_window_gaps() {
        let job = paint_job(NativePanelVisualDisplayMode::Expanded);

        let operations = resolve_windows_native_panel_hit_test_blocker_operations(&job);

        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::FillHitTestBlocker {
                frame,
                alpha: 1,
            } if *frame == job.shell_frame
        )));
        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::FillHitTestBlocker {
                frame,
                alpha: 1,
            } if *frame == PanelRect {
                x: job.shell_frame.x,
                y: job.shell_frame.y + job.shell_frame.height,
                width: job.shell_frame.width,
                height: job.content_frame.height - job.shell_frame.height,
            }
        )));
    }

    #[test]
    fn compact_paint_does_not_add_alpha_window_gap_blockers() {
        let operations = resolve_windows_native_panel_hit_test_blocker_operations(&paint_job(
            NativePanelVisualDisplayMode::Compact,
        ));

        assert!(operations.is_empty());
    }

    #[test]
    fn mascot_paint_operation_uses_mac_body_fill_color() {
        let plan = resolve_windows_native_panel_paint_plan(&paint_job(
            NativePanelVisualDisplayMode::Compact,
        ));
        let operations = resolve_windows_native_panel_paint_operations(&plan);

        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::FillMascotDot {
                color: WindowsNativePanelPaintColor { r: 5, g: 5, b: 5 },
                ..
            }
        )));
    }

    #[test]
    fn mascot_paint_operation_uses_mac_body_shape() {
        let plan = resolve_windows_native_panel_paint_plan(&paint_job(
            NativePanelVisualDisplayMode::Compact,
        ));
        let operations = resolve_windows_native_panel_paint_operations(&plan);

        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::FillMascotDot {
                frame,
                radius,
                ..
            } if (frame.width - 24.0).abs() < 0.5
                && (frame.height - 20.0).abs() < 0.5
                && (*radius - 6.0).abs() < 0.5
        )));
    }

    #[test]
    fn mascot_paint_operation_consumes_shared_body_style() {
        let plan = resolve_windows_native_panel_paint_plan(&paint_job(
            NativePanelVisualDisplayMode::Compact,
        ));
        let operations = resolve_windows_native_panel_paint_operations(&plan);

        assert!(operations.iter().any(|operation| matches!(
            operation,
            WindowsNativePanelPaintOperation::FillMascotDot {
                color: WindowsNativePanelPaintColor { r: 5, g: 5, b: 5 },
                stroke_color: WindowsNativePanelPaintColor { r: 255, g: 122, b: 36 },
                stroke_width,
                ..
            } if (*stroke_width - 2.2).abs() < 0.001
        )));
    }

    #[test]
    fn mascot_child_paint_operations_preserve_primitive_alpha() {
        let alpha = 0.42;
        let frame = PanelRect {
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        };
        let color = WindowsNativePanelPaintColor::rgb(255, 255, 255);

        let round_rect = windows_native_panel_paint_operation_from_primitive(
            &WindowsNativePanelPaintPrimitive::MascotRoundRect {
                role: crate::native_panel_renderer::facade::visual::NativePanelVisualMascotRoundRectRole::Mouth,
                frame,
                radius: 2.0,
                color,
                alpha,
            },
        );
        let ellipse = windows_native_panel_paint_operation_from_primitive(
            &WindowsNativePanelPaintPrimitive::MascotEllipse {
                role: crate::native_panel_renderer::facade::visual::NativePanelVisualMascotEllipseRole::LeftEye,
                frame,
                color,
                alpha,
            },
        );
        let text = windows_native_panel_paint_operation_from_primitive(
            &WindowsNativePanelPaintPrimitive::MascotText {
                role: crate::native_panel_renderer::facade::visual::NativePanelVisualMascotTextRole::SleepLabel,
                origin: crate::native_panel_core::PanelPoint { x: 1.0, y: 2.0 },
                max_width: 20.0,
                text: "z".to_string(),
                color,
                size: 10,
                weight: crate::native_panel_renderer::facade::visual::NativePanelVisualTextWeight::Semibold,
                alignment: crate::native_panel_renderer::facade::visual::NativePanelVisualTextAlignment::Center,
                alpha,
            },
        );

        assert!(matches!(
            round_rect,
            WindowsNativePanelPaintOperation::FillRoundRect {
                alpha: preserved,
                ..
            } if (preserved - alpha).abs() < 0.001
        ));
        assert!(matches!(
            ellipse,
            WindowsNativePanelPaintOperation::FillEllipse {
                alpha: preserved,
                ..
            } if (preserved - alpha).abs() < 0.001
        ));
        assert!(matches!(
            text,
            WindowsNativePanelPaintOperation::DrawText {
                alpha: preserved,
                ..
            } if (preserved - alpha).abs() < 0.001
        ));
    }

    #[test]
    fn hidden_paint_plan_has_no_backend_operations() {
        let plan = resolve_windows_native_panel_paint_plan(&paint_job(
            NativePanelVisualDisplayMode::Hidden,
        ));

        let operations = resolve_windows_native_panel_paint_operations(&plan);

        assert!(operations.is_empty());
    }

    #[test]
    fn production_painter_prefers_direct2d_per_pixel_alpha_backend() {
        assert_eq!(
            super::windows_native_panel_preferred_painter_backend(),
            super::WindowsNativePanelPainterBackend::Direct2D
        );
        assert_eq!(
            super::windows_native_panel_composition_mode_for_preferred_painter(),
            crate::windows_native_panel::layered_window::WindowsLayeredWindowCompositionMode::PerPixelAlpha
        );
    }
}
