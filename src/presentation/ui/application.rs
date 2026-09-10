use std::{
    error::Error,
    process::{Child, Command},
    sync::Arc,
    time::Duration,
    time::Instant,
};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{CursorIcon, Fullscreen, Window, WindowId},
};

use crate::presentation::{audio::EasterEggAudio, sim_engine::SimEnginePresenter};

use super::{
    DesktopUiModel, PhysicsEditorCommand, PhysicsSnapshotRef,
    catalog::{MechanicsTool, PhysSubdomain, Screen},
    geometry::Point,
    layout::{InteractionTarget, UiLayout},
    state::{UiAction, UiState},
};

const TARGET_FRAME_INTERVAL: Duration = Duration::from_nanos(8_333_333);
const BODY_PICK_RADIUS_LOGICAL: f32 = 26.0;
const MINIMUM_EDITOR_DRAG_LOGICAL: f32 = 2.0;
const MAX_EXTERNAL_LINK_PROCESSES: usize = 8;

#[derive(Clone, Copy, Debug)]
enum EditorDrag {
    MoveBody {
        entity: crate::foundation::EntityId,
        started_at: Point,
    },
    Force {
        entity: crate::foundation::EntityId,
        started_at: Point,
        world_start: Point,
    },
}

#[derive(Debug, Default)]
struct ExternalLinkLauncher {
    children: Vec<Child>,
}

impl ExternalLinkLauncher {
    fn open(&mut self, url: &str) -> Result<(), String> {
        self.reap_finished();
        if self.children.len() >= MAX_EXTERNAL_LINK_PROCESSES {
            return Err(format!(
                "external link process limit {MAX_EXTERNAL_LINK_PROCESSES} reached"
            ));
        }
        let child = Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|error| error.to_string())?;
        self.children.push(child);
        Ok(())
    }

    fn reap_finished(&mut self) {
        self.children.retain_mut(|child| match child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) | Err(_) => true,
        });
    }
}

pub(crate) fn run<Model>(model: Model) -> Result<(), Box<dyn Error>>
where
    Model: DesktopUiModel + 'static,
{
    env_logger::init();
    let event_loop = EventLoop::new()?;
    let mut application = SimXApplication::new(model);
    event_loop.run_app(&mut application)?;
    Ok(())
}

struct SimXApplication<Model> {
    window: Option<Arc<Window>>,
    presenter: Option<SimEnginePresenter>,
    state: UiState,
    model: Model,
    easter_egg_audio: EasterEggAudio,
    external_link_launcher: ExternalLinkLauncher,
    pointer: Point,
    pointer_inside: bool,
    previous_frame_at: Instant,
    next_frame_at: Instant,
    editor_drag: Option<EditorDrag>,
    camera_pan_active: bool,
}

impl<Model> SimXApplication<Model>
where
    Model: DesktopUiModel,
{
    fn new(model: Model) -> Self {
        let now = Instant::now();
        Self {
            window: None,
            presenter: None,
            state: UiState::default(),
            model,
            easter_egg_audio: EasterEggAudio::new(),
            external_link_launcher: ExternalLinkLauncher::default(),
            pointer: Point::default(),
            pointer_inside: false,
            previous_frame_at: now,
            next_frame_at: now,
            editor_drag: None,
            camera_pan_active: false,
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
            .then(|| {
                layout.hit_test(
                    self.state.screen,
                    self.state.selected_subdomain,
                    self.pointer,
                )
            })
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

    fn handle_ui_action(&mut self, event_loop: &ActiveEventLoop, action: Option<UiAction>) {
        if !screen_allows_pointer_gestures(self.state.screen)
            || matches!(
                action,
                Some(
                    UiAction::OpenPhysicsProject { .. }
                        | UiAction::ResetPhysicsProject
                        | UiAction::EnterPhysicsView
                        | UiAction::ApplyPhysicsView
                        | UiAction::DiscardPhysicsView
                        | UiAction::ClosePhysicsProject
                )
            )
        {
            self.cancel_pointer_gestures();
        }
        match action {
            Some(UiAction::ExitRequested) => event_loop.exit(),
            Some(UiAction::OpenExternalLink(link)) => {
                if let Err(error) = self.external_link_launcher.open(link.url()) {
                    eprintln!("Could not open {}: {error}", link.url());
                    self.state.report_external_link_failure(link);
                }
            }
            Some(UiAction::OpenPhysicsProject {
                subdomain,
                template,
            }) => {
                if self
                    .model
                    .open_physics_project(subdomain, template)
                    .is_err()
                {
                    self.state.reject_project_open();
                } else if let Some(presenter) = self.presenter.as_mut() {
                    presenter.reset_scientific_camera(&self.state);
                }
            }
            Some(UiAction::ResetPhysicsProject) => {
                self.model.reset_physics_project();
                if let Some(presenter) = self.presenter.as_mut() {
                    presenter.reset_scientific_camera(&self.state);
                }
            }
            Some(UiAction::EditPhysics(command)) => match self.model.edit_physics(command) {
                Ok(outcome) => self.state.report_editor_outcome(outcome),
                Err(_) => self.state.report_editor_failure(),
            },
            Some(UiAction::EnterPhysicsView) => {
                if self.model.enter_physics_view().is_err() {
                    self.state.reject_view_entry();
                }
            }
            Some(UiAction::ApplyPhysicsView) => self.model.apply_physics_view(),
            Some(UiAction::DiscardPhysicsView) => self.model.discard_physics_view(),
            Some(UiAction::ClosePhysicsProject) => self.model.close_physics_project(),
            Some(UiAction::SetPlaybackRate(rate)) => self.model.set_playback_rate(rate),
            None => {}
        }
        self.state.sync_playback_rate(self.model.playback_rate());
    }

    fn physics_canvas(&self, layout: UiLayout) -> Option<super::geometry::UiRect> {
        if self.state.selected_subdomain == PhysSubdomain::Thermodynamics {
            return None;
        }
        match self.state.screen {
            Screen::PhysicsEditor => Some(layout.editor_canvas()),
            Screen::PhysicsView => Some(layout.view_canvas()),
            _ => None,
        }
    }

    fn cancel_pointer_gestures(&mut self) {
        clear_pointer_gestures(
            &mut self.editor_drag,
            &mut self.camera_pan_active,
            &mut self.state,
        );
    }

    fn mechanics_body_at_pointer(&self, layout: UiLayout) -> Option<crate::foundation::EntityId> {
        if self.state.selected_subdomain != PhysSubdomain::Mechanics {
            return None;
        }
        let presenter = self.presenter.as_ref()?;
        let PhysicsSnapshotRef::Mechanics(project) = self.model.physics_snapshot()? else {
            return None;
        };
        let canvas = layout.editor_canvas();
        project
            .world
            .bodies
            .iter()
            .filter_map(|body| {
                let world = Point::new(
                    finite_f64_to_f32(body.position.x().get())?,
                    finite_f64_to_f32(body.position.y().get())?,
                );
                let screen = presenter.scientific_world_to_logical(world, canvas).ok()?;
                let dx = screen.x - self.pointer.x;
                let dy = screen.y - self.pointer.y;
                let distance_squared = dx.mul_add(dx, dy * dy);
                (distance_squared <= BODY_PICK_RADIUS_LOGICAL * BODY_PICK_RADIUS_LOGICAL)
                    .then_some((distance_squared, body.id))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, entity)| entity)
    }

    fn pointer_world(&self, canvas: super::geometry::UiRect) -> Result<Point, String> {
        self.presenter
            .as_ref()
            .ok_or_else(|| "renderer is unavailable".to_owned())?
            .logical_to_scientific_world(self.pointer, canvas)
    }

    fn begin_editor_canvas_action(&mut self, layout: UiLayout) {
        if self.state.selected_subdomain != PhysSubdomain::Mechanics {
            self.state.report_editor_failure();
            return;
        }
        let canvas = layout.editor_canvas();
        let selected = self.mechanics_body_at_pointer(layout);
        match (self.state.selected_tool, selected) {
            (MechanicsTool::Body, Some(entity)) => {
                self.state.select_body(Some(entity));
                self.editor_drag = Some(EditorDrag::MoveBody {
                    entity,
                    started_at: self.pointer,
                });
            }
            (MechanicsTool::Body, None) => match self.pointer_world(canvas) {
                Ok(world) => {
                    match self
                        .model
                        .edit_physics(PhysicsEditorCommand::PlaceMechanicsBody {
                            x_meters: f64::from(world.x),
                            y_meters: f64::from(world.y),
                        }) {
                        Ok(outcome) => self.state.report_editor_outcome(outcome),
                        Err(_) => self.state.report_editor_failure(),
                    }
                }
                Err(_) => self.state.report_editor_failure(),
            },
            (MechanicsTool::Force, Some(entity)) => match self.pointer_world(canvas) {
                Ok(world_start) => {
                    self.state.select_body(Some(entity));
                    self.state
                        .set_force_drag_preview(Some((self.pointer, self.pointer)));
                    self.editor_drag = Some(EditorDrag::Force {
                        entity,
                        started_at: self.pointer,
                        world_start,
                    });
                }
                Err(_) => self.state.report_editor_failure(),
            },
            (MechanicsTool::Force, None) | (MechanicsTool::Pendulum | MechanicsTool::Spring, _) => {
                self.state.report_editor_failure();
            }
        }
    }

    fn finish_editor_drag(&mut self, layout: UiLayout) {
        let Some(drag) = self.editor_drag.take() else {
            return;
        };
        self.state.set_force_drag_preview(None);
        if self.state.screen != Screen::PhysicsEditor
            || self.state.selected_subdomain != PhysSubdomain::Mechanics
        {
            return;
        }
        let command = match drag {
            EditorDrag::MoveBody { entity, started_at } => {
                if drag_is_click(started_at, self.pointer) {
                    return;
                }
                let Ok(world) = self.pointer_world(layout.editor_canvas()) else {
                    self.state.report_editor_failure();
                    return;
                };
                PhysicsEditorCommand::MoveMechanicsBody {
                    entity,
                    x_meters: f64::from(world.x),
                    y_meters: f64::from(world.y),
                }
            }
            EditorDrag::Force {
                entity,
                started_at,
                world_start,
            } => {
                let world_end = if drag_is_click(started_at, self.pointer) {
                    world_start
                } else {
                    let Ok(world_end) = self.pointer_world(layout.editor_canvas()) else {
                        self.state.report_editor_failure();
                        return;
                    };
                    world_end
                };
                force_drag_command(entity, started_at, self.pointer, world_start, world_end)
            }
        };
        match self.model.edit_physics(command) {
            Ok(outcome) => self.state.report_editor_outcome(outcome),
            Err(_) => self.state.report_editor_failure(),
        }
    }
}

fn drag_is_click(start: Point, end: Point) -> bool {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    dx.mul_add(dx, dy * dy) < MINIMUM_EDITOR_DRAG_LOGICAL * MINIMUM_EDITOR_DRAG_LOGICAL
}

fn finite_f64_to_f32(value: f64) -> Option<f32> {
    if value.is_finite() && value >= f64::from(f32::MIN) && value <= f64::from(f32::MAX) {
        Some(value as f32)
    } else {
        None
    }
}

fn force_drag_command(
    entity: crate::foundation::EntityId,
    screen_start: Point,
    screen_end: Point,
    world_start: Point,
    world_end: Point,
) -> PhysicsEditorCommand {
    let (x_newtons, y_newtons) = if drag_is_click(screen_start, screen_end) {
        (0.0, 0.0)
    } else {
        (
            f64::from(world_end.x - world_start.x),
            f64::from(world_end.y - world_start.y),
        )
    };
    PhysicsEditorCommand::SetMechanicsForce {
        entity,
        x_newtons,
        y_newtons,
    }
}

impl<Model> ApplicationHandler for SimXApplication<Model>
where
    Model: DesktopUiModel,
{
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
            WindowEvent::CloseRequested => {
                if self.state.screen == Screen::PhysicsView {
                    if let Some(layout) = self.layout() {
                        let action =
                            self.state
                                .activate(InteractionTarget::Back, Point::default(), layout);
                        self.handle_ui_action(event_loop, action);
                        window.request_redraw();
                    }
                } else if self.state.screen != Screen::PhysicsViewExit {
                    event_loop.exit();
                }
            }
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
                    let action = self.state.handle_key(key_code, layout, hovered);
                    self.handle_ui_action(event_loop, action);
                    self.sync_easter_egg_audio(previous_screen);
                    self.update_cursor(&window, layout);
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let previous_pointer = self.pointer;
                if let Some(presenter) = self.presenter.as_ref() {
                    match presenter.physical_to_logical(position.x as f32, position.y as f32) {
                        Ok(point) => {
                            self.pointer = point;
                            self.pointer_inside = true;
                        }
                        Err(error) => eprintln!("Could not convert pointer coordinates: {error}"),
                    }
                }
                if self.camera_pan_active
                    && let Some(presenter) = self.presenter.as_mut()
                    && let Err(error) = presenter.pan_scientific_camera(Point::new(
                        self.pointer.x - previous_pointer.x,
                        self.pointer.y - previous_pointer.y,
                    ))
                {
                    eprintln!("Could not pan scientific camera: {error}");
                }
                if matches!(self.editor_drag, Some(EditorDrag::Force { .. })) {
                    let start = match self.editor_drag {
                        Some(EditorDrag::Force { started_at, .. }) => started_at,
                        _ => self.pointer,
                    };
                    self.state
                        .set_force_drag_preview(Some((start, self.pointer)));
                }
                if let Some(layout) = self.layout() {
                    self.update_cursor(&window, layout);
                }
                window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer_inside = false;
                self.cancel_pointer_gestures();
                window.set_cursor(CursorIcon::Default);
                window.request_redraw();
            }
            WindowEvent::Focused(false) => {
                self.pointer_inside = false;
                self.cancel_pointer_gestures();
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
                    if target == InteractionTarget::EditorCanvas
                        && self.state.screen == Screen::PhysicsEditor
                    {
                        self.begin_editor_canvas_action(layout);
                    } else {
                        let previous_screen = self.state.screen;
                        let action = self.state.activate(target, self.pointer, layout);
                        self.handle_ui_action(event_loop, action);
                        self.sync_easter_egg_audio(previous_screen);
                    }
                    self.update_cursor(&window, layout);
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(layout) = self.layout() {
                    self.finish_editor_drag(layout);
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Middle,
                ..
            } => {
                self.camera_pan_active = state == ElementState::Pressed
                    && self
                        .layout()
                        .and_then(|layout| self.physics_canvas(layout))
                        .is_some_and(|canvas| canvas.contains(self.pointer));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(layout) = self.layout()
                    && let Some(canvas) = self.physics_canvas(layout)
                    && canvas.contains(self.pointer)
                {
                    let steps = match delta {
                        MouseScrollDelta::LineDelta(_, vertical) => vertical,
                        MouseScrollDelta::PixelDelta(delta) => {
                            (delta.y as f32 / 80.0).clamp(-4.0, 4.0)
                        }
                    };
                    let factor = 1.15_f32.powf(steps);
                    if let Some(presenter) = self.presenter.as_mut()
                        && let Err(error) =
                            presenter.zoom_scientific_camera(self.pointer, canvas, factor)
                    {
                        eprintln!("Could not zoom scientific camera: {error}");
                    }
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let elapsed = now
                    .saturating_duration_since(self.previous_frame_at)
                    .as_secs_f32();
                self.previous_frame_at = now;
                self.model.advance(f64::from(elapsed));
                self.state.sync_playback_rate(self.model.playback_rate());
                let Some(layout) = self.layout() else {
                    return;
                };
                let hovered = self.hovered_target(layout);
                self.state.update_animations(hovered, elapsed);
                if let Some(presenter) = self.presenter.as_mut()
                    && let Err(error) = presenter.render(
                        layout,
                        &self.state,
                        self.model.physics_snapshot(),
                        self.model.physics_error(),
                    )
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
        self.external_link_launcher.reap_finished();
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

fn clear_pointer_gestures(
    editor_drag: &mut Option<EditorDrag>,
    camera_pan_active: &mut bool,
    state: &mut UiState,
) {
    *editor_drag = None;
    *camera_pan_active = false;
    state.set_force_drag_preview(None);
}

fn screen_allows_pointer_gestures(screen: Screen) -> bool {
    matches!(screen, Screen::PhysicsEditor | Screen::PhysicsView)
}

#[cfg(test)]
mod tests {
    use super::{
        EditorDrag, clear_pointer_gestures, drag_is_click, force_drag_command,
        screen_allows_pointer_gestures,
    };
    use crate::presentation::ui::{PhysicsEditorCommand, geometry::Point};

    #[test]
    fn force_click_threshold_is_stable_for_the_clear_gesture() {
        let start = Point::new(100.0, 100.0);
        assert!(drag_is_click(start, Point::new(101.0, 101.0)));
        assert!(!drag_is_click(start, Point::new(103.0, 100.0)));

        let entity = crate::foundation::EntityId::new(1).expect("entity");
        assert_eq!(
            force_drag_command(
                entity,
                start,
                Point::new(101.0, 101.0),
                Point::new(3.0, 4.0),
                Point::new(7.0, 9.0),
            ),
            PhysicsEditorCommand::SetMechanicsForce {
                entity,
                x_newtons: 0.0,
                y_newtons: 0.0,
            }
        );
    }

    #[test]
    fn pointer_loss_cancels_every_armed_gesture_without_committing() {
        let entity = crate::foundation::EntityId::new(1).expect("entity");
        let point = Point::new(4.0, 5.0);
        let mut editor_drag = Some(EditorDrag::Force {
            entity,
            started_at: point,
            world_start: point,
        });
        let mut camera_pan_active = true;
        let mut state = crate::presentation::ui::state::UiState::default();
        state.set_force_drag_preview(Some((point, point)));

        clear_pointer_gestures(&mut editor_drag, &mut camera_pan_active, &mut state);

        assert!(editor_drag.is_none());
        assert!(!camera_pan_active);
        assert!(state.force_drag_preview.is_none());
    }

    #[test]
    fn view_exit_modal_never_allows_an_armed_pointer_gesture() {
        assert!(screen_allows_pointer_gestures(
            crate::presentation::ui::catalog::Screen::PhysicsView
        ));
        assert!(!screen_allows_pointer_gestures(
            crate::presentation::ui::catalog::Screen::PhysicsViewExit
        ));
    }
}
