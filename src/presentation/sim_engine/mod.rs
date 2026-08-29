mod pixel_font;
mod scenes;

use std::sync::Arc;

use ::sim_engine::{
    Camera2d, PhysicalScreenPosition, RendererPresentMode, Vec2, WgpuRenderer, WgpuRendererOptions,
};
use winit::window::Window;

use super::ui::{geometry::Point, layout::UiLayout, state::UiState};

pub(in crate::presentation) struct SimEnginePresenter {
    renderer: WgpuRenderer,
    camera: Camera2d,
}

impl SimEnginePresenter {
    pub(in crate::presentation) fn new(
        window: Arc<Window>,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<Self, String> {
        let options = WgpuRendererOptions::new(RendererPresentMode::Vsync, scale_factor)
            .map_err(|error| error.to_string())?;
        let renderer = pollster::block_on(WgpuRenderer::new_with_options(
            window, width, height, options,
        ))
        .map_err(|error| error.to_string())?;
        let camera = Camera2d::new(Vec2::ZERO, 1.0).map_err(|error| error.to_string())?;
        Ok(Self { renderer, camera })
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

    pub(in crate::presentation) fn render(
        &mut self,
        layout: UiLayout,
        state: &UiState,
    ) -> Result<(), String> {
        let scene = scenes::build(layout, state);
        self.renderer
            .render(&scene, &self.camera)
            .map(|_| ())
            .map_err(|error| format!("{error:?}"))
    }
}
