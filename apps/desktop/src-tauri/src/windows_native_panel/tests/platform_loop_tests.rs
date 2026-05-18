use super::*;

#[test]
fn windows_runtime_pump_platform_loop_consumes_passthrough_command() {
    let now = Instant::now();
    let mut runtime = super::WindowsNativePanelRuntime {
        ignores_mouse_events: true,
        ..Default::default()
    };
    runtime.host.presenter.present(shell_draw_frame(
        vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 110.0,
                y: 60.0,
                width: 100.0,
                height: 30.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        false,
    ));
    let _ = runtime.host.consume_presenter_into_shell_result();
    let _ = runtime
        .sync_host_polling_interaction(PanelPoint { x: 120.0, y: 70.0 }, false, now)
        .expect("polling interaction");

    assert_eq!(runtime.host.shell.last_ignores_mouse_events(), Some(false));

    runtime.pump_platform_loop().expect("pump platform loop");

    assert_eq!(runtime.platform_loop.last_ignores_mouse_events, Some(false));
    assert_eq!(runtime.platform_loop.redraw_request_count, 1);
    assert!(runtime.host.take_pending_shell_commands().is_empty());
}

#[test]
fn windows_runtime_pump_platform_loop_auto_consumes_presenter_frame() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime
        .host
        .presenter
        .present(shell_draw_frame(Vec::new(), false));
    runtime.create_panel().expect("create panel");

    runtime.pump_platform_loop().expect("pump platform loop");

    assert_eq!(runtime.host.shell.redraw_requests(), 1);
    assert_eq!(runtime.platform_loop.redraw_request_count, 1);
    assert!(
        runtime.host.shell.pending_paint_job().is_some()
            || runtime.platform_loop.last_painted_job.is_some()
    );
}

#[test]
fn windows_platform_hit_region_cache_uses_shared_pointer_regions() {
    let hwnd = 4242;
    let regions = vec![NativePanelPointerRegion {
        frame: PanelRect {
            x: 20.0,
            y: 10.0,
            width: 120.0,
            height: 36.0,
        },
        kind: NativePanelPointerRegionKind::CompactBar,
    }];

    super::platform_loop::sync_windows_native_panel_hit_regions(Some(hwnd), &regions);

    assert_eq!(
        super::platform_loop::resolve_windows_native_panel_cached_hit_test(
            hwnd,
            PanelPoint { x: 40.0, y: 20.0 }
        ),
        super::hit_region::WindowsNativePanelHitTest::Client
    );
    assert_eq!(
        super::platform_loop::resolve_windows_native_panel_cached_hit_test(
            hwnd,
            PanelPoint { x: 4.0, y: 4.0 }
        ),
        super::hit_region::WindowsNativePanelHitTest::Transparent
    );

    super::platform_loop::clear_windows_native_panel_hit_regions(Some(hwnd));

    assert_eq!(
        super::platform_loop::resolve_windows_native_panel_cached_hit_test(
            hwnd,
            PanelPoint { x: 40.0, y: 20.0 }
        ),
        super::hit_region::WindowsNativePanelHitTest::Transparent
    );
}

#[test]
fn windows_runtime_pump_platform_loop_syncs_hit_regions_after_presenter_frame() {
    let mut runtime = super::WindowsNativePanelRuntime::default();

    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");
    let hwnd = runtime.host.shell.raw_window_handle().expect("shell hwnd");
    runtime.host.presenter.present(shell_draw_frame(
        vec![NativePanelPointerRegion {
            frame: PanelRect {
                x: 110.0,
                y: 60.0,
                width: 100.0,
                height: 30.0,
            },
            kind: NativePanelPointerRegionKind::CompactBar,
        }],
        false,
    ));

    runtime
        .pump_platform_loop()
        .expect("pump presenter hit regions");

    assert_eq!(
        super::platform_loop::resolve_windows_native_panel_cached_hit_test(
            hwnd,
            PanelPoint { x: 120.0, y: 70.0 }
        ),
        super::hit_region::WindowsNativePanelHitTest::Client
    );
    assert_eq!(
        super::platform_loop::resolve_windows_native_panel_cached_hit_test(
            hwnd,
            PanelPoint { x: 10.0, y: 10.0 }
        ),
        super::hit_region::WindowsNativePanelHitTest::Transparent
    );
}

#[test]
fn windows_platform_loop_surface_resource_revision_tracks_physical_rect_changes() {
    let mut state = super::platform_loop::WindowsNativePanelPlatformLoopState::default();
    let first = Some(super::dpi::WindowsPhysicalRect {
        x: -1600,
        y: 0,
        width: 316,
        height: 100,
    });
    let second = Some(super::dpi::WindowsPhysicalRect {
        x: -1600,
        y: 0,
        width: 380,
        height: 120,
    });

    state.sync_surface_resource_rect(first);
    assert_eq!(state.surface_resource_revision, 1);
    assert_eq!(state.last_physical_window_rect, first);

    state.sync_surface_resource_rect(first);
    assert_eq!(state.surface_resource_revision, 1);

    state.sync_surface_resource_rect(second);
    assert_eq!(state.surface_resource_revision, 2);
    assert_eq!(state.last_physical_window_rect, second);
}

#[test]
fn windows_platform_loop_records_physical_window_rect_from_sync_command() {
    let mut state = super::platform_loop::WindowsNativePanelPlatformLoopState::default();
    let mut raw_window_handle = Some(1);
    let window_state = NativePanelHostWindowState {
        frame: Some(PanelRect {
            x: -1280.0,
            y: -16.0,
            width: 253.0,
            height: 80.0,
        }),
        visible: true,
        preferred_display_index: 1,
    };

    state
        .consume_shell_command(
            &mut raw_window_handle,
            super::window_shell::WindowsNativePanelShellCommand::SyncWindowState(window_state),
        )
        .expect("sync window state");

    assert_eq!(
        state.last_physical_window_rect,
        Some(super::dpi::WindowsPhysicalRect {
            x: -1280,
            y: -16,
            width: 253,
            height: 80,
        })
    );
    assert_eq!(state.surface_resource_revision, 1);
}

#[test]
fn windows_native_window_state_positioning_keeps_panel_topmost_without_activation() {
    let behavior = super::platform_loop::windows_native_window_positioning_behavior();

    assert!(behavior.topmost);
    assert!(behavior.no_activate);
    assert!(!behavior.preserve_existing_z_order);
}

#[test]
fn windows_platform_loop_clamps_offscreen_physical_rect_to_virtual_bounds() {
    let rect = super::dpi::WindowsPhysicalRect {
        x: 4000,
        y: -2400,
        width: 420,
        height: 80,
    };
    let bounds = super::dpi::WindowsPhysicalRect {
        x: -1920,
        y: -300,
        width: 4480,
        height: 1740,
    };

    assert_eq!(
        super::platform_loop::clamp_windows_physical_rect_to_bounds(rect, bounds),
        super::dpi::WindowsPhysicalRect {
            x: 2140,
            y: -300,
            width: 420,
            height: 80,
        }
    );
}

#[test]
fn windows_platform_loop_clamps_oversized_physical_rect_to_virtual_bounds() {
    let rect = super::dpi::WindowsPhysicalRect {
        x: -5000,
        y: 6000,
        width: 9000,
        height: 2000,
    };
    let bounds = super::dpi::WindowsPhysicalRect {
        x: -1200,
        y: 0,
        width: 3200,
        height: 1080,
    };

    assert_eq!(
        super::platform_loop::clamp_windows_physical_rect_to_bounds(rect, bounds),
        super::dpi::WindowsPhysicalRect {
            x: -1200,
            y: 0,
            width: 3200,
            height: 1080,
        }
    );
}

#[test]
fn windows_platform_loop_surface_resource_revision_tracks_dpi_scale_changes() {
    let mut state = super::platform_loop::WindowsNativePanelPlatformLoopState::default();
    let physical_rect = Some(super::dpi::WindowsPhysicalRect {
        x: 100,
        y: 40,
        width: 300,
        height: 120,
    });

    state.sync_surface_resource_state(physical_rect, super::dpi::WindowsDpiScale::from_scale(1.0));
    assert_eq!(state.surface_resource_revision, 1);
    assert_eq!(
        state.last_surface_dpi_scale,
        Some(super::dpi::WindowsDpiScale::from_scale(1.0))
    );

    state.sync_surface_resource_state(physical_rect, super::dpi::WindowsDpiScale::from_scale(1.0));
    assert_eq!(state.surface_resource_revision, 1);

    state.sync_surface_resource_state(physical_rect, super::dpi::WindowsDpiScale::from_scale(1.5));
    assert_eq!(state.surface_resource_revision, 2);
    assert_eq!(
        state.last_surface_dpi_scale,
        Some(super::dpi::WindowsDpiScale::from_scale(1.5))
    );
}

#[test]
fn windows_platform_loop_tracks_negative_origin_physical_rect_after_dpi_change() {
    let mut state = super::platform_loop::WindowsNativePanelPlatformLoopState::default();
    let logical_frame = PanelRect {
        x: -1280.0,
        y: -24.0,
        width: 253.0,
        height: 80.0,
    };

    state.sync_surface_resource_state(
        Some(super::dpi::WindowsDpiScale::from_scale(1.25).rect_to_physical(logical_frame)),
        super::dpi::WindowsDpiScale::from_scale(1.25),
    );
    assert_eq!(
        state.last_physical_window_rect,
        Some(super::dpi::WindowsPhysicalRect {
            x: -1600,
            y: -30,
            width: 316,
            height: 100,
        })
    );
    assert_eq!(state.surface_resource_revision, 1);

    state.sync_surface_resource_state(
        Some(super::dpi::WindowsDpiScale::from_scale(1.5).rect_to_physical(logical_frame)),
        super::dpi::WindowsDpiScale::from_scale(1.5),
    );
    assert_eq!(
        state.last_physical_window_rect,
        Some(super::dpi::WindowsPhysicalRect {
            x: -1920,
            y: -36,
            width: 380,
            height: 120,
        })
    );
    assert_eq!(state.surface_resource_revision, 2);
}

#[test]
fn windows_runtime_display_reposition_updates_platform_physical_rect() {
    let mut runtime = super::WindowsNativePanelRuntime::default();
    runtime
        .host
        .apply_animation_descriptor(PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 120.0,
            visible_height: 120.0,
            width_progress: 1.0,
            height_progress: 0.0,
            shoulder_progress: 0.0,
            drop_progress: 0.0,
            cards_progress: 0.0,
        })
        .expect("seed descriptor");
    runtime.create_panel().expect("create panel");
    runtime.pump_platform_loop().expect("pump create");

    runtime
        .host
        .reposition_to_display(
            2,
            Some(PanelRect {
                x: -1280.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            }),
        )
        .expect("reposition display");
    runtime.pump_platform_loop().expect("pump reposition");

    let width_spec = crate::native_panel_core::island_width_spec(
        crate::app_settings::current_app_settings().island_width_preset,
    );
    let expected_frame = super::host_window::resolve_windows_panel_window_frame(
        PanelAnimationDescriptor {
            kind: PanelAnimationKind::Open,
            canvas_height: 120.0,
            visible_height: 120.0,
            width_progress: 1.0,
            height_progress: 0.0,
            shoulder_progress: 0.0,
            drop_progress: 0.0,
            cards_progress: 0.0,
        },
        PanelRect {
            x: -1280.0,
            y: 0.0,
            width: 1280.0,
            height: 720.0,
        },
        width_spec.canvas_width,
        width_spec.canvas_width,
    );

    assert_eq!(
        runtime.platform_loop.last_physical_window_rect,
        Some(super::dpi::WindowsPhysicalRect {
            x: expected_frame.x as i32,
            y: expected_frame.y as i32,
            width: expected_frame.width as i32,
            height: expected_frame.height as i32,
        })
    );
    assert_eq!(
        runtime
            .platform_loop
            .last_window_state
            .expect("last window state")
            .preferred_display_index,
        2
    );
    assert!(runtime.platform_loop.surface_resource_revision >= 2);
}
