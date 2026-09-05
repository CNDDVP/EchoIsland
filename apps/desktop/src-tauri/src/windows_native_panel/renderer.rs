use crate::{
    native_panel_core::{
        ExpandedSurface, PanelAnimationDescriptor, PanelAnimationKind, PanelGeometryMetrics,
        PanelLayout, PanelLayoutInput, PanelRect, PanelRenderState, PanelRenderStateInput,
        resolve_panel_layout, resolve_panel_render_state,
    },
    native_panel_renderer::facade::{
        descriptor::{
            NativePanelHostWindowDescriptor, NativePanelHostWindowState, NativePanelPointerRegion,
            NativePanelPointerRegionKind, NativePanelTimelineDescriptor,
            native_panel_timeline_descriptor_for_animation,
        },
        presentation::{
            NativePanelPresentationModel, estimated_scene_content_height_for_card_width,
        },
        renderer::{
            NativePanelCachedRendererBackend, NativePanelClosePresentationPlan,
            NativePanelRenderCommandBundle, NativePanelRenderer, NativePanelRuntimeSceneCache,
            apply_native_panel_preserved_close_presentation_slots,
            cache_host_window_descriptor_on_renderer, cache_host_window_state_on_renderer,
            cache_pointer_regions_on_renderer, cache_render_command_bundle_on_renderer,
            cache_scene_runtime_on_renderer, cache_timeline_descriptor_on_renderer,
            cached_runtime_render_state, cached_scene,
            resolve_and_cache_presentation_from_scene_cache_on_renderer,
            resolve_cached_presentation_model, resolve_native_panel_animation_plan,
        },
    },
    native_panel_scene::{PanelRuntimeRenderState, PanelScene},
};

use super::{WINDOWS_FALLBACK_PANEL_SCREEN_FRAME, host_runtime::WindowsNativePanelHost};

pub(crate) struct WindowsNativePanelRenderer {
    pub(super) scene_cache: NativePanelRuntimeSceneCache,
    pub(super) last_screen_frame: Option<PanelRect>,
    pub(super) last_animation_descriptor: Option<PanelAnimationDescriptor>,
    pub(super) last_timeline_descriptor: Option<NativePanelTimelineDescriptor>,
    pub(super) last_host_window_descriptor: Option<NativePanelHostWindowDescriptor>,
    pub(super) last_layout: Option<PanelLayout>,
    pub(super) last_render_state: Option<PanelRenderState>,
    pub(super) last_window_state: Option<NativePanelHostWindowState>,
    pub(super) last_pointer_regions: Vec<NativePanelPointerRegion>,
    pub(super) last_presentation_model: Option<NativePanelPresentationModel>,
    pub(super) active_close_presentation_plan: Option<NativePanelClosePresentationPlan>,
}

impl Default for WindowsNativePanelRenderer {
    fn default() -> Self {
        Self {
            scene_cache: NativePanelRuntimeSceneCache::default(),
            last_screen_frame: None,
            last_animation_descriptor: Some(default_windows_panel_animation_descriptor()),
            last_timeline_descriptor: None,
            last_host_window_descriptor: None,
            last_layout: None,
            last_render_state: None,
            last_window_state: None,
            last_pointer_regions: Vec::new(),
            last_presentation_model: None,
            active_close_presentation_plan: None,
        }
    }
}

fn default_windows_panel_animation_descriptor() -> PanelAnimationDescriptor {
    PanelAnimationDescriptor {
        kind: PanelAnimationKind::Open,
        canvas_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
        visible_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
        width_progress: 0.0,
        height_progress: 0.0,
        shoulder_progress: 0.0,
        drop_progress: 0.0,
        cards_progress: 0.0,
    }
}

impl NativePanelCachedRendererBackend for WindowsNativePanelRenderer {
    type Error = String;

    fn scene_cache_mut(&mut self) -> &mut NativePanelRuntimeSceneCache {
        &mut self.scene_cache
    }

    fn timeline_descriptor_slot(&mut self) -> &mut Option<NativePanelTimelineDescriptor> {
        &mut self.last_timeline_descriptor
    }

    fn host_window_descriptor_slot(&mut self) -> &mut Option<NativePanelHostWindowDescriptor> {
        &mut self.last_host_window_descriptor
    }

    fn host_window_state_slot(&mut self) -> &mut Option<NativePanelHostWindowState> {
        &mut self.last_window_state
    }

    fn pointer_regions_slot(&mut self) -> &mut Vec<NativePanelPointerRegion> {
        &mut self.last_pointer_regions
    }

    fn presentation_model_slot(&mut self) -> Option<&mut Option<NativePanelPresentationModel>> {
        Some(&mut self.last_presentation_model)
    }

    fn after_scene_runtime_cached(&mut self) -> Result<(), Self::Error> {
        self.refresh_cached_render_inputs();
        Ok(())
    }

    fn after_timeline_descriptor_cached(
        &mut self,
        descriptor: NativePanelTimelineDescriptor,
    ) -> Result<(), Self::Error> {
        self.last_animation_descriptor = Some(descriptor.animation);
        self.refresh_cached_render_inputs();
        Ok(())
    }

    fn after_render_command_bundle_cached(
        &mut self,
        bundle: &NativePanelRenderCommandBundle,
    ) -> Result<(), Self::Error> {
        self.last_layout = Some(bundle.layout);
        self.last_render_state = Some(bundle.render_state);
        Ok(())
    }
}

impl NativePanelRenderer for WindowsNativePanelRenderer {
    type Error = String;

    fn render_scene(
        &mut self,
        scene: &PanelScene,
        runtime: PanelRuntimeRenderState,
    ) -> Result<(), Self::Error> {
        cache_scene_runtime_on_renderer(self, scene, runtime)
    }

    fn apply_animation_descriptor(
        &mut self,
        descriptor: PanelAnimationDescriptor,
    ) -> Result<(), Self::Error> {
        self.last_animation_descriptor = Some(descriptor);
        self.refresh_cached_render_inputs();
        Ok(())
    }

    fn apply_timeline_descriptor(
        &mut self,
        descriptor: NativePanelTimelineDescriptor,
    ) -> Result<(), Self::Error> {
        cache_timeline_descriptor_on_renderer(self, descriptor)
    }

    fn sync_host_window_state(
        &mut self,
        state: NativePanelHostWindowState,
    ) -> Result<(), Self::Error> {
        cache_host_window_state_on_renderer(self, state)
    }

    fn sync_screen_frame(&mut self, screen_frame: Option<PanelRect>) -> Result<(), Self::Error> {
        self.update_screen_frame(screen_frame);
        Ok(())
    }

    fn sync_shared_body_height(
        &mut self,
        _shared_body_height: Option<f64>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn record_host_window_descriptor(
        &mut self,
        descriptor: NativePanelHostWindowDescriptor,
    ) -> Result<(), Self::Error> {
        cache_host_window_descriptor_on_renderer(self, descriptor)
    }

    fn sync_pointer_regions(
        &mut self,
        regions: &[NativePanelPointerRegion],
    ) -> Result<(), Self::Error> {
        cache_pointer_regions_on_renderer(self, regions)
    }

    fn apply_render_command_bundle(
        &mut self,
        bundle: &NativePanelRenderCommandBundle,
    ) -> Result<(), Self::Error> {
        cache_render_command_bundle_on_renderer(self, bundle)
    }

    fn set_visible(&mut self, _visible: bool) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl WindowsNativePanelRenderer {
    fn current_width_spec(&self) -> crate::native_panel_core::PanelIslandWidthSpec {
        let preset = self
            .scene_cache
            .last_cache_key
            .as_ref()
            .map(|key| key.scene_input.settings.island_width_preset)
            .unwrap_or_else(|| crate::app_settings::current_app_settings().island_width_preset);
        crate::native_panel_core::island_width_spec(preset)
    }

    pub(super) fn apply_close_presentation_plan(
        &mut self,
        preserved_presentation: Option<&NativePanelPresentationModel>,
        plan: NativePanelClosePresentationPlan,
    ) {
        self.active_close_presentation_plan = Some(plan);
        if !plan.should_apply_preserved_card_stack {
            if plan.should_suppress_edge_actions {
                self.suppress_edge_actions_for_close_transition();
            }
            return;
        }
        let Some(preserved_presentation) =
            preserved_presentation.filter(|presentation| !presentation.card_stack.cards.is_empty())
        else {
            self.hide_card_stack_for_close_transition();
            if plan.should_suppress_edge_actions {
                self.suppress_edge_actions_for_close_transition();
            }
            return;
        };
        apply_native_panel_preserved_close_presentation_slots(
            preserved_presentation,
            self.scene_cache.last_scene.as_mut(),
            self.scene_cache.last_render_command_bundle.as_mut(),
            self.last_presentation_model.as_mut(),
        );
        if plan.should_suppress_edge_actions {
            self.suppress_edge_actions_for_close_transition();
        }
    }

    fn suppress_edge_actions_for_close_transition(&mut self) {
        if let Some(scene) = self.scene_cache.last_scene.as_mut() {
            scene.compact_bar.actions_visible = false;
            scene.surface_scene.edge_actions_visible = false;
        }
        if let Some(bundle) = self.scene_cache.last_render_command_bundle.as_mut() {
            bundle.scene.compact_bar.actions_visible = false;
            bundle.scene.surface_scene.edge_actions_visible = false;
            bundle.compact_bar.actions_visible = false;
            for button in &mut bundle.action_buttons {
                button.visible = false;
            }
            bundle.pointer_regions.retain(|region| {
                !matches!(region.kind, NativePanelPointerRegionKind::EdgeAction(_))
            });
        }
        if let Some(render_state) = self.last_render_state.as_mut() {
            render_state.layer_style.edge_actions_visible = false;
        }
        if let Some(presentation) = self.last_presentation_model.as_mut() {
            presentation.compact_bar.actions_visible = false;
            presentation.action_buttons.visible = false;
        }
        self.last_pointer_regions
            .retain(|region| !matches!(region.kind, NativePanelPointerRegionKind::EdgeAction(_)));
    }

    fn hide_card_stack_for_close_transition(&mut self) {
        if let Some(scene) = self.scene_cache.last_scene.as_mut() {
            scene.cards.clear();
        }
        if let Some(bundle) = self.scene_cache.last_render_command_bundle.as_mut() {
            bundle.scene.cards.clear();
            bundle.card_stack.cards.clear();
            bundle.card_stack.visible = false;
            bundle.card_stack.content_height = 0.0;
            bundle.card_stack.body_height = 0.0;
        }
        if let Some(presentation) = self.last_presentation_model.as_mut() {
            presentation.card_stack.cards.clear();
            presentation.card_stack.visible = false;
            presentation.card_stack.content_height = 0.0;
            presentation.card_stack.body_height = 0.0;
        }
    }

    pub(super) fn current_presentation_model(&self) -> Option<NativePanelPresentationModel> {
        resolve_cached_presentation_model(self.last_presentation_model.as_ref(), &self.scene_cache)
    }

    pub(super) fn latest_scene_presentation_model(&self) -> Option<NativePanelPresentationModel> {
        resolve_cached_presentation_model(None, &self.scene_cache)
            .or_else(|| self.current_presentation_model())
    }

    pub(super) fn latest_scene_body_height_for_current_width(&self) -> Option<f64> {
        let scene = cached_scene(&self.scene_cache)?;
        let width_spec = self.current_width_spec();
        let card_width = crate::native_panel_core::resolve_expanded_cards_width(
            width_spec.expanded_width,
            crate::native_panel_core::EXPANDED_CARDS_SIDE_INSET,
        );
        Some(
            estimated_scene_content_height_for_card_width(&scene, card_width)
                .min(crate::native_panel_core::EXPANDED_MAX_BODY_HEIGHT),
        )
    }

    pub(super) fn update_screen_frame(&mut self, screen_frame: Option<PanelRect>) {
        self.last_screen_frame = screen_frame;
        self.refresh_cached_render_inputs();
    }

    fn refresh_cached_render_inputs(&mut self) {
        let descriptor = self
            .last_animation_descriptor
            .unwrap_or_else(default_windows_panel_animation_descriptor);
        self.last_animation_descriptor = Some(descriptor);
        let screen_frame = self
            .last_screen_frame
            .unwrap_or(WINDOWS_FALLBACK_PANEL_SCREEN_FRAME);
        let scene = cached_scene(&self.scene_cache);
        let card_count = scene
            .as_ref()
            .map(|scene| scene.cards.len())
            .unwrap_or_default();
        let timeline = self
            .last_timeline_descriptor
            .unwrap_or_else(|| native_panel_timeline_descriptor_for_animation(descriptor));
        let animation_plan = resolve_native_panel_animation_plan(timeline, card_count);
        let cards_visibility = animation_plan.card_stack.visibility_progress;
        let width_spec = self.current_width_spec();
        let layout = resolve_panel_layout(PanelLayoutInput {
            screen_frame,
            metrics: PanelGeometryMetrics {
                compact_height: crate::native_panel_core::DEFAULT_COMPACT_PILL_HEIGHT,
                compact_width: width_spec.compact_width,
                expanded_width: width_spec.expanded_width,
                panel_width: width_spec.canvas_width,
            },
            canvas_height: descriptor.canvas_height,
            visible_height: descriptor.visible_height,
            bar_progress: descriptor.width_progress,
            height_progress: descriptor.height_progress,
            drop_progress: descriptor.drop_progress,
            content_visibility: cards_visibility,
            collapsed_height: crate::native_panel_core::COLLAPSED_PANEL_HEIGHT,
            drop_distance: crate::native_panel_core::PANEL_DROP_DISTANCE,
            content_top_gap: crate::native_panel_core::EXPANDED_CONTENT_TOP_GAP,
            content_bottom_inset: crate::native_panel_core::EXPANDED_CONTENT_BOTTOM_INSET,
            cards_side_inset: crate::native_panel_core::EXPANDED_CARDS_SIDE_INSET,
            shoulder_size: crate::native_panel_core::COMPACT_SHOULDER_SIZE,
            separator_side_inset: crate::native_panel_core::EXPANDED_SEPARATOR_SIDE_INSET,
        });
        let status_surface_active = scene
            .as_ref()
            .is_some_and(|scene| scene.surface == ExpandedSurface::Status);
        let runtime = cached_runtime_render_state(&self.scene_cache).unwrap_or_default();
        let close_transition = descriptor.kind == PanelAnimationKind::Close;
        if !close_transition {
            self.active_close_presentation_plan = None;
        }
        let suppress_edge_actions = close_transition
            && self
                .active_close_presentation_plan
                .is_some_and(|plan| plan.should_suppress_edge_actions);
        let render_state = resolve_panel_render_state(PanelRenderStateInput {
            shared_expanded_enabled: false,
            shell_visible: layout.shell_visible,
            separator_visibility: layout.separator_visibility,
            bar_progress: descriptor.width_progress,
            height_progress: descriptor.height_progress,
            chrome_transition_progress:
                crate::native_panel_core::resolve_panel_chrome_transition_progress(descriptor),
            shoulder_progress: descriptor.shoulder_progress,
            cards_height: layout.cards_frame.height,
            status_surface_active,
            content_visibility: cards_visibility,
            transitioning: runtime.transitioning,
            headline_emphasized: runtime.shell_scene.headline_emphasized,
            edge_actions_visible: runtime.shell_scene.edge_actions_visible
                && !suppress_edge_actions,
        });
        self.refresh_cached_window_state(descriptor, screen_frame);

        if scene.is_none() {
            self.last_layout = Some(layout);
            self.last_render_state = Some(render_state);
            self.last_pointer_regions = Vec::new();
            self.last_presentation_model = None;
            self.scene_cache.last_render_command_bundle = None;
            return;
        }
        let _ = resolve_and_cache_presentation_from_scene_cache_on_renderer(
            self,
            layout,
            render_state,
            None,
        );
        if suppress_edge_actions {
            self.suppress_edge_actions_for_close_transition();
        }
    }

    fn refresh_cached_window_state(
        &mut self,
        descriptor: PanelAnimationDescriptor,
        screen_frame: PanelRect,
    ) {
        let width_spec = self.current_width_spec();
        let frame = super::host_window::resolve_windows_panel_window_frame(
            descriptor,
            screen_frame,
            width_spec.canvas_width,
            width_spec.canvas_width,
        );
        let (screen_scale_factor, screen_physical_frame, visible, preferred_display_index) =
            if let Some(host) = self.last_host_window_descriptor {
                (
                    host.screen_scale_factor,
                    host.screen_physical_frame,
                    host.visible,
                    host.preferred_display_index,
                )
            } else if let Some(state) = self.last_window_state {
                (
                    state.screen_scale_factor,
                    state.screen_physical_frame,
                    state.visible,
                    state.preferred_display_index,
                )
            } else {
                (None, None, false, 0)
            };

        self.last_window_state = Some(NativePanelHostWindowState {
            screen_scale_factor,
            screen_physical_frame,
            frame: Some(frame),
            visible,
            preferred_display_index,
        });
    }
}

impl WindowsNativePanelHost {
    pub(super) fn renderer_ref(&self) -> &WindowsNativePanelRenderer {
        &self.renderer
    }
}
