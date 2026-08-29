use std::{error::Error, sync::Arc, time::Duration, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{CursorIcon, Fullscreen, Window, WindowId},
};

use crate::presentation::{audio::EasterEggAudio, sim_engine::SimEnginePresenter};

use super::{
    geometry::Point,
    layout::{InteractionTarget, UiLayout},
    state::UiState,
};

const TARGET_FRAME_INTERVAL: Duration = Duration::from_nanos(8_333_333);

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    let mut application = SimXApplication::new();
    event_loop.run_app(&mut application)?;
    Ok(())
}

struct SimXApplication {
    window: Option<Arc<Window>>,
    presenter: Option<SimEnginePresenter>,
    state: UiState,
    easter_egg_audio: EasterEggAudio,
    pointer: Point,
    pointer_inside: bool,
    previous_frame_at: Instant,
    next_frame_at: Instant,
}

impl SimXApplication {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            window: None,
            presenter: None,
            state: UiState::default(),
            easter_egg_audio: EasterEggAudio::new(),
            pointer: Point::default(),
            pointer_inside: false,
            previous_frame_at: now,
            next_frame_at: now,
        }
    }

    fn layout(&self) -> Option<UiLayout> {
        self.presenter.as_ref().map(|presenter| {
            let (width, height) = presenter.logical_size();
            UiLayout::new(width, height)
        })
    }

    fn hovered_target(&self, layout: UiLayout) -> Option<InteractionTarget> {
        self.pointer_inside
            .then(|| layout.hit_test(self.state.screen, self.pointer))
            .flatten()
    }

    fn update_cursor(&self, window: &Window, layout: UiLayout) {
        window.set_cursor(if self.hovered_target(layout).is_some() {
            CursorIcon::Pointer
        } else {
            CursorIcon::Default
        });
    }

    fn sync_easter_egg_audio(&mut self, previous_screen: super::catalog::Screen) {
        use super::catalog::Screen;

        match (previous_screen, self.state.screen) {
            (Screen::TimeEasterEgg, Screen::TimeEasterEgg) => {}
            (_, Screen::TimeEasterEgg) => {
                if let Err(error) = self.easter_egg_audio.play_time_gate() {
                    eprintln!("Sim;Time opened without audio: {error}");
                }
            }
            (Screen::TimeEasterEgg, _) => self.easter_egg_audio.stop(),
            _ => {}
        }
    }
}

impl ApplicationHandler for SimXApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Sim;X")
            .with_inner_size(LogicalSize::new(1100.0, 760.0))
            .with_min_inner_size(LogicalSize::new(800.0, 620.0))
            .with_fullscreen(Some(Fullscreen::Borderless(None)));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("Could not create the Sim;X window: {error}");
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        let presenter = match SimEnginePresenter::new(
            window.clone(),
            size.width.max(1),
            size.height.max(1),
            window.scale_factor(),
        ) {
            Ok(presenter) => presenter,
            Err(error) => {
                eprintln!("Could not initialize Sim;Engine: {error}");
                event_loop.exit();
                return;
            }
        };

        window.request_redraw();
        self.window = Some(window);
        self.presenter = Some(presenter);
        let now = Instant::now();
        self.previous_frame_at = now;
        self.next_frame_at = now;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.clone() else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(presenter) = self.presenter.as_mut()
                    && let Err(error) =
                        presenter.resize(size.width, size.height, window.scale_factor())
                {
                    eprintln!("Could not resize Sim;Engine: {error}");
                }
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(presenter) = self.presenter.as_mut()
                    && let Err(error) = presenter.set_scale_factor(scale_factor)
                {
                    eprintln!("Could not update the display scale: {error}");
                }
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                if let PhysicalKey::Code(key_code) = event.physical_key
                    && let Some(layout) = self.layout()
                {
                    let previous_screen = self.state.screen;
                    let hovered = self.hovered_target(layout);
                    self.state.handle_key(key_code, layout, hovered);
                    self.sync_easter_egg_audio(previous_screen);
                    self.update_cursor(&window, layout);
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(presenter) = self.presenter.as_ref() {
                    match presenter.physical_to_logical(position.x as f32, position.y as f32) {
                        Ok(point) => {
                            self.pointer = point;
                            self.pointer_inside = true;
                        }
                        Err(error) => eprintln!("Could not convert pointer coordinates: {error}"),
                    }
                }
                if let Some(layout) = self.layout() {
                    self.update_cursor(&window, layout);
                }
                window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer_inside = false;
                window.set_cursor(CursorIcon::Default);
                window.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(layout) = self.layout()
                    && let Some(target) = self.hovered_target(layout)
                {
                    let previous_screen = self.state.screen;
                    self.state.activate(target, self.pointer, layout);
                    self.sync_easter_egg_audio(previous_screen);
                    self.update_cursor(&window, layout);
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let elapsed = now
                    .saturating_duration_since(self.previous_frame_at)
                    .as_secs_f32();
                self.previous_frame_at = now;
                let Some(layout) = self.layout() else {
                    return;
                };
                let hovered = self.hovered_target(layout);
                self.state.update_animations(hovered, elapsed);
                window.pre_present_notify();
                if let Some(presenter) = self.presenter.as_mut()
                    && let Err(error) = presenter.render(layout, &self.state)
                {
                    eprintln!("Sim;Engine frame failed: {error}");
                }
                self.next_frame_at = now + TARGET_FRAME_INTERVAL;
                event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_at));
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame_at {
            self.next_frame_at = now + TARGET_FRAME_INTERVAL;
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_at));
        }
        if self.window.is_none() {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
