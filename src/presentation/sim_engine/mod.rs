mod pixel_font;
mod scene_builder;
mod scenes;
mod social_icons;

use std::sync::Arc;

use ::sim_engine::{
    Camera2d, FrameBudget, FrameComposerError, FramePassOptions, LogicalScreenPosition,
    LogicalViewport, PhysicalScreenPosition, RendererFrameError, RendererPresentMode,
    RendererSurfaceStatus, Vec2, WgpuRenderer, WgpuRendererOptions,
};
use winit::window::Window;

use super::ui::catalog::{PhysSubdomain, ProjectTemplate, Screen};
use super::ui::{PhysicsSnapshotRef, geometry::Point, layout::UiLayout, state::UiState};
use scene_builder::BuiltFrame;
use social_icons::SocialIconAtlas;

pub(in crate::presentation) struct SimEnginePresenter {
    renderer: WgpuRenderer,
    social_icons: SocialIconAtlas,
    scientific_camera: Camera2d,
    camera_context: Option<PhysicsCameraContext>,
    retained_resources_ready: bool,
}

enum RenderAttemptError {
    Adapter(String),
    Engine(FrameComposerError),
}

impl RenderAttemptError {
    fn into_message(self) -> String {
        match self {
            Self::Adapter(message) => message,
            Self::Engine(error) => error.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PhysicsCameraContext {
    subdomain: PhysSubdomain,
    project: ProjectTemplate,
}

const UI_FRAME_BUDGET: FrameBudget =
    FrameBudget::new(8, 32_768, 2_000_000, 192 * 1024 * 1024, 64 * 1024, 32_768);

impl SimEnginePresenter {
    pub(in crate::presentation) fn new(
        window: Arc<Window>,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<Self, String> {
        let options = WgpuRendererOptions::new(RendererPresentMode::Vsync, scale_factor)
            .map_err(|error| error.to_string())?;
        let mut renderer = pollster::block_on(WgpuRenderer::new_with_options(
            Arc::clone(&window),
            width,
            height,
            options,
        ))
        .map_err(|error| error.to_string())?;
        renderer.set_pre_present_notify(move || window.pre_present_notify());
        let social_icons = SocialIconAtlas::new(&renderer)?;
        let scientific_camera =
            Camera2d::new(Vec2::ZERO, 1.0).map_err(|error| error.to_string())?;
        Ok(Self {
            renderer,
            social_icons,
            scientific_camera,
            camera_context: None,
            retained_resources_ready: true,
        })
    }

    pub(in crate::presentation) fn logical_size(&self) -> (f32, f32) {
        self.renderer.logical_size()
    }

    pub(in crate::presentation) fn resize(
        &mut self,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<(), String> {
        self.renderer
            .resize_with_scale_factor(width, height, scale_factor)
            .map_err(|error| error.to_string())
    }

    pub(in crate::presentation) fn set_scale_factor(
        &mut self,
        scale_factor: f64,
    ) -> Result<(), String> {
        self.renderer
            .set_scale_factor(scale_factor)
            .map_err(|error| error.to_string())
    }

    pub(in crate::presentation) fn physical_to_logical(
        &self,
        x: f32,
        y: f32,
    ) -> Result<Point, String> {
        self.renderer
            .physical_to_logical_screen(PhysicalScreenPosition::new(x, y))
            .map(|position| Point::new(position.to_vec2().x(), position.to_vec2().y()))
            .map_err(|error| error.to_string())
    }

    pub(in crate::presentation) fn logical_to_scientific_world(
        &self,
        point: Point,
        canvas: super::ui::geometry::UiRect,
    ) -> Result<Point, String> {
        logical_to_world(self.scientific_camera, point, canvas)
    }

    pub(in crate::presentation) fn scientific_world_to_logical(
        &self,
        point: Point,
        canvas: super::ui::geometry::UiRect,
    ) -> Result<Point, String> {
        let viewport = LogicalViewport::new(canvas.width(), canvas.height())
            .map_err(|error| error.to_string())?;
        self.scientific_camera
            .world_to_screen(Vec2::new(point.x, point.y), viewport)
            .map(|screen| {
                let local = screen.to_vec2();
                Point::new(canvas.min.x + local.x(), canvas.min.y + local.y())
            })
            .map_err(|error| error.to_string())
    }

    pub(in crate::presentation) fn pan_scientific_camera(
        &mut self,
        logical_delta: Point,
    ) -> Result<(), String> {
        let zoom = self.scientific_camera.zoom();
        self.scientific_camera
            .pan_by(Vec2::new(-logical_delta.x / zoom, logical_delta.y / zoom))
            .map_err(|error| error.to_string())
    }

    pub(in crate::presentation) fn zoom_scientific_camera(
        &mut self,
        anchor: Point,
        canvas: super::ui::geometry::UiRect,
        factor: f32,
    ) -> Result<(), String> {
        let viewport = LogicalViewport::new(canvas.width(), canvas.height())
            .map_err(|error| error.to_string())?;
        let local = LogicalScreenPosition::new(anchor.x - canvas.min.x, anchor.y - canvas.min.y);
        self.scientific_camera
            .zoom_about_screen(factor, local, viewport)
            .map_err(|error| error.to_string())
    }

    pub(in crate::presentation) fn reset_scientific_camera(&mut self, state: &UiState) {
        self.camera_context = None;
        self.sync_camera_context(state);
    }

    pub(in crate::presentation) fn render(
        &mut self,
        layout: UiLayout,
        state: &UiState,
        snapshot: Option<PhysicsSnapshotRef<'_>>,
        physics_error: Option<&str>,
    ) -> Result<(), String> {
        self.ensure_retained_resources()?;
        self.sync_camera_context(state);
        let visual_frame = scenes::build(
            layout,
            state,
            snapshot,
            physics_error,
            self.scientific_camera,
        )?;

        match self.render_once(&visual_frame, layout, state) {
            Ok(()) => Ok(()),
            Err(error) if render_error_requires_recovery(&error) => {
                self.recover_renderer_and_resources()?;
                self.render_once(&visual_frame, layout, state)
                    .map_err(|retry_error| {
                        format!(
                            "Sim;Engine frame failed after one renderer recovery: {}",
                            retry_error.into_message()
                        )
                    })
            }
            Err(error) => Err(error.into_message()),
        }
    }

    fn render_once(
        &mut self,
        visual_frame: &BuiltFrame,
        layout: UiLayout,
        state: &UiState,
    ) -> Result<(), RenderAttemptError> {
        let mut frame = self
            .renderer
            .begin_frame(visual_frame.screen.background(), UI_FRAME_BUDGET)
            .map_err(RenderAttemptError::Engine)?;
        frame
            .draw_screen_scene(&visual_frame.screen, FramePassOptions::new(0))
            .map_err(RenderAttemptError::Engine)?;
        if let Some(scientific) = &visual_frame.scientific {
            frame
                .draw_scene(&scientific.scene, scientific.camera, scientific.options)
                .map_err(RenderAttemptError::Engine)?;
        }
        if let Some(heads_up) = &visual_frame.heads_up {
            frame
                .draw_screen_scene(heads_up, FramePassOptions::new(20))
                .map_err(RenderAttemptError::Engine)?;
        }
        if let Some(overlay) = &visual_frame.overlay {
            frame
                .draw_screen_scene(overlay, FramePassOptions::new(30))
                .map_err(RenderAttemptError::Engine)?;
        }
        self.social_icons
            .add_to_frame(&mut frame, layout, state)
            .map_err(RenderAttemptError::Adapter)?;
        frame
            .present()
            .map(|_| ())
            .map_err(RenderAttemptError::Engine)
    }

    fn ensure_retained_resources(&mut self) -> Result<(), String> {
        if self.retained_resources_ready {
            return Ok(());
        }
        self.social_icons.restore(&self.renderer)?;
        self.retained_resources_ready = true;
        Ok(())
    }

    fn recover_renderer_and_resources(&mut self) -> Result<(), String> {
        pollster::block_on(self.renderer.recover_device_and_surface())
            .map_err(|error| format!("could not recover Sim;Engine device and surface: {error}"))?;
        self.retained_resources_ready = false;
        self.ensure_retained_resources()
    }

    fn sync_camera_context(&mut self, state: &UiState) {
        let is_physics_workspace = matches!(
            state.screen,
            Screen::PhysicsEditor | Screen::PhysicsView | Screen::PhysicsViewExit
        );
        if !is_physics_workspace {
            return;
        }
        let context = PhysicsCameraContext {
            subdomain: state.selected_subdomain,
            project: state.selected_project,
        };
        if self.camera_context == Some(context) {
            return;
        }
        self.scientific_camera = default_camera(context.subdomain);
        self.camera_context = Some(context);
    }
}

fn render_error_requires_recovery(error: &RenderAttemptError) -> bool {
    matches!(
        error,
        RenderAttemptError::Engine(FrameComposerError::Frame(RendererFrameError::Surface(
            RendererSurfaceStatus::Lost
        )))
    )
}

fn default_camera(subdomain: PhysSubdomain) -> Camera2d {
    let (center, zoom) = match subdomain {
        PhysSubdomain::Mechanics => (Vec2::ZERO, 90.0),
        PhysSubdomain::WavesAndOptics => (Vec2::new(3.2, 0.0), 100.0),
        PhysSubdomain::Electromagnetism => (Vec2::ZERO, 130.0),
        PhysSubdomain::Thermodynamics
        | PhysSubdomain::Relativity
        | PhysSubdomain::FluidDynamics
        | PhysSubdomain::Sandbox => (Vec2::ZERO, 1.0),
    };
    Camera2d::new(center, zoom).unwrap_or_default()
}

fn logical_to_world(
    camera: Camera2d,
    point: Point,
    canvas: super::ui::geometry::UiRect,
) -> Result<Point, String> {
    let viewport =
        LogicalViewport::new(canvas.width(), canvas.height()).map_err(|error| error.to_string())?;
    let local = LogicalScreenPosition::new(point.x - canvas.min.x, point.y - canvas.min.y);
    camera
        .screen_to_world(local, viewport)
        .map(|world| Point::new(world.x(), world.y()))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use ::sim_engine::{FrameComposerError, RendererFrameError, RendererSurfaceStatus};

    use super::{
        RenderAttemptError, default_camera, logical_to_world, render_error_requires_recovery,
    };
    use crate::presentation::ui::{
        catalog::PhysSubdomain,
        geometry::{Point, UiRect},
    };

    #[test]
    fn mechanics_pointer_conversion_uses_meters() {
        let canvas = UiRect::from_min_size(Point::new(100.0, 50.0), 900.0, 600.0);
        let camera = default_camera(PhysSubdomain::Mechanics);
        let center = logical_to_world(camera, canvas.center(), canvas).expect("center");
        let one_meter_right = logical_to_world(
            camera,
            Point::new(canvas.center().x + 90.0, canvas.center().y),
            canvas,
        )
        .expect("one meter");

        assert_eq!(center, Point::new(0.0, 0.0));
        assert_eq!(one_meter_right, Point::new(1.0, 0.0));
    }

    #[test]
    fn only_typed_surface_loss_requests_renderer_recovery() {
        let lost = RenderAttemptError::Engine(FrameComposerError::Frame(
            RendererFrameError::Surface(RendererSurfaceStatus::Lost),
        ));
        let invalid_geometry = RenderAttemptError::Engine(FrameComposerError::Frame(
            RendererFrameError::InvalidGeometryTransform,
        ));
        let adapter = RenderAttemptError::Adapter("adapter failure".to_owned());

        assert!(render_error_requires_recovery(&lost));
        for status in [
            RendererSurfaceStatus::Timeout,
            RendererSurfaceStatus::Occluded,
            RendererSurfaceStatus::Outdated,
            RendererSurfaceStatus::Validation,
        ] {
            let transient_or_invalid = RenderAttemptError::Engine(FrameComposerError::Frame(
                RendererFrameError::Surface(status),
            ));
            assert!(!render_error_requires_recovery(&transient_or_invalid));
        }
        assert!(!render_error_requires_recovery(&invalid_geometry));
        assert!(!render_error_requires_recovery(&adapter));
    }
}
