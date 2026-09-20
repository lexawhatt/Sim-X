use sim_logic::prelude::*;
use std::collections::BTreeSet;

use super::{
    attachment::{Attachment, pick_surface, world_point},
    catalog::{CatalogState, CatalogTarget},
    document::{Document, Object, ObjectKind, PhysicsEnvironment, Point},
    layout::Rect,
    placement::Placement,
    selection::SelectionGesture,
    session::{PlaybackSpeed, RunSession},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Editor,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Select,
    Build,
    Erase,
    Rod,
    Spring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    Tool(Tool),
    Palette(ObjectKind),
    Run,
    Back,
    Undo,
    Redo,
    Snap,
    Home,
    Delete,
    Duplicate,
    MassDown,
    MassUp,
    SizeDown,
    SizeUp,
    RotateLeft,
    RotateRight,
    Pendulum,
    SpringPair,
    Environment,
    CloseEnvironment,
    GravityLeft,
    GravityRight,
    GravityDown,
    GravityUp,
    EarthGravity,
    ZeroGravity,
    DragDown,
    DragUp,
    Pause,
    Slow,
    Normal,
    Fast,
    Fixed,
    HeightDown,
    HeightUp,
    RestitutionDown,
    RestitutionUp,
}

impl Control {
    pub(crate) const ALL: [Self; 43] = [
        Self::Tool(Tool::Select),
        Self::Tool(Tool::Build),
        Self::Tool(Tool::Erase),
        Self::Palette(ObjectKind::Ball),
        Self::Palette(ObjectKind::Box),
        Self::Palette(ObjectKind::Anchor),
        Self::Run,
        Self::Back,
        Self::Undo,
        Self::Redo,
        Self::Snap,
        Self::Home,
        Self::Delete,
        Self::Duplicate,
        Self::MassDown,
        Self::MassUp,
        Self::SizeDown,
        Self::SizeUp,
        Self::RotateLeft,
        Self::RotateRight,
        Self::Tool(Tool::Rod),
        Self::Tool(Tool::Spring),
        Self::Pendulum,
        Self::SpringPair,
        Self::Environment,
        Self::CloseEnvironment,
        Self::GravityLeft,
        Self::GravityRight,
        Self::GravityDown,
        Self::GravityUp,
        Self::EarthGravity,
        Self::ZeroGravity,
        Self::DragDown,
        Self::DragUp,
        Self::Pause,
        Self::Slow,
        Self::Normal,
        Self::Fast,
        Self::Fixed,
        Self::HeightDown,
        Self::HeightUp,
        Self::RestitutionDown,
        Self::RestitutionUp,
    ];
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Tool(Tool::Select) => 0,
            Self::Tool(Tool::Build) => 1,
            Self::Tool(Tool::Erase) => 2,
            Self::Palette(kind) => 3 + kind.index(),
            Self::Run => 6,
            Self::Back => 7,
            Self::Undo => 8,
            Self::Redo => 9,
            Self::Snap => 10,
            Self::Home => 11,
            Self::Delete => 12,
            Self::Duplicate => 13,
            Self::MassDown => 14,
            Self::MassUp => 15,
            Self::SizeDown => 16,
            Self::SizeUp => 17,
            Self::RotateLeft => 18,
            Self::RotateRight => 19,
            Self::Tool(Tool::Rod) => 20,
            Self::Tool(Tool::Spring) => 21,
            Self::Pendulum => 22,
            Self::SpringPair => 23,
            Self::Environment => 24,
            Self::CloseEnvironment => 25,
            Self::GravityLeft => 26,
            Self::GravityRight => 27,
            Self::GravityDown => 28,
            Self::GravityUp => 29,
            Self::EarthGravity => 30,
            Self::ZeroGravity => 31,
            Self::DragDown => 32,
            Self::DragUp => 33,
            Self::Pause => 34,
            Self::Slow => 35,
            Self::Normal => 36,
            Self::Fast => 37,
            Self::Fixed => 38,
            Self::HeightDown => 39,
            Self::HeightUp => 40,
            Self::RestitutionDown => 41,
            Self::RestitutionUp => 42,
        }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Tool(Tool::Select) => "Select  [1]",
            Self::Tool(Tool::Build) => "Build  [2]",
            Self::Tool(Tool::Erase) => "Erase  [3]",
            Self::Palette(kind) => kind.label(),
            Self::Run => "Run  [F5]",
            Self::Back => "Stop / Editor",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Snap => "Snap",
            Self::Home => "Home",
            Self::Delete => "Delete",
            Self::Duplicate => "Duplicate",
            Self::MassDown | Self::SizeDown | Self::HeightDown | Self::RestitutionDown => "-",
            Self::MassUp | Self::SizeUp | Self::HeightUp | Self::RestitutionUp => "+",
            Self::RotateLeft => "-15°",
            Self::RotateRight => "+15°",
            Self::Tool(Tool::Rod) => "Rod",
            Self::Tool(Tool::Spring) => "Spring",
            Self::Pendulum => "Pendulum",
            Self::SpringPair => "Oscillator",
            Self::Environment => "Environment",
            Self::CloseEnvironment => "Done",
            Self::GravityLeft | Self::GravityDown | Self::DragDown => "-",
            Self::GravityRight | Self::GravityUp | Self::DragUp => "+",
            Self::EarthGravity => "Earth",
            Self::ZeroGravity => "Zero gravity",
            Self::Pause => "Pause / Play",
            Self::Slow => "0.25x",
            Self::Normal => "1x",
            Self::Fast => "4x",
            Self::Fixed => "Fixed body",
        }
    }

    pub(crate) const fn is_environment(self) -> bool {
        matches!(
            self,
            Self::CloseEnvironment
                | Self::GravityLeft
                | Self::GravityRight
                | Self::GravityDown
                | Self::GravityUp
                | Self::EarthGravity
                | Self::ZeroGravity
                | Self::DragDown
                | Self::DragUp
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Camera {
    pub(crate) center: Point,
    pub(crate) pixels_per_m: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Point::new(0.0, 0.0),
            pixels_per_m: 48.0,
        }
    }
}

impl Camera {
    pub(crate) fn project(self, point: Point, canvas: Rect) -> LogicalScreenPosition {
        LogicalScreenPosition::new(
            canvas.x + canvas.width * 0.5 + ((point.x - self.center.x) * self.pixels_per_m) as f32,
            canvas.y + canvas.height * 0.5 - ((point.y - self.center.y) * self.pixels_per_m) as f32,
        )
    }
    pub(crate) fn unproject(self, point: LogicalScreenPosition, canvas: Rect) -> Point {
        let point = point.to_vec2();
        Point::new(
            self.center.x
                + f64::from(point.x() - canvas.x - canvas.width * 0.5) / self.pixels_per_m,
            self.center.y
                - f64::from(point.y() - canvas.y - canvas.height * 0.5) / self.pixels_per_m,
        )
    }
    pub(crate) fn zoom(&mut self, factor: f64, pointer: LogicalScreenPosition, canvas: Rect) {
        let before = self.unproject(pointer, canvas);
        self.pixels_per_m = (self.pixels_per_m * factor).clamp(8.0, 240.0);
        let after = self.unproject(pointer, canvas);
        self.center = Point::new(
            (self.center.x + before.x - after.x).clamp(-1e6, 1e6),
            (self.center.y + before.y - after.y).clamp(-1e6, 1e6),
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Drag {
    Place {
        placement: Placement,
        position: Point,
        from_palette: bool,
    },
    Move {
        id: u64,
        offset: Point,
        position: Point,
    },
    Erase {
        id: u64,
    },
    Link {
        attachment: Attachment,
    },
    MoveSelection {
        origin: Point,
        delta: Point,
    },
}

#[derive(Resource)]
pub(crate) struct EditorState {
    pub(crate) project_name: String,
    pub(crate) document: Document,
    pub(crate) mode: Mode,
    pub(crate) tool: Tool,
    pub(crate) brush: Placement,
    pub(crate) catalog: CatalogState,
    pub(crate) catalog_pointer: PointerButton<CatalogTarget>,
    pub(crate) selected: Option<u64>,
    pub(crate) selection: BTreeSet<u64>,
    pub(crate) selection_gesture: Option<SelectionGesture>,
    pub(crate) camera: Camera,
    pub(crate) snap: bool,
    pub(crate) hovered: Option<Control>,
    pub(crate) emphasis: [f32; Control::ALL.len()],
    pub(crate) status: &'static str,
    pub(crate) drag: Option<Drag>,
    pub(crate) pan: Option<PointerSample>,
    pub(crate) pointer: PointerButton<Control>,
    pub(crate) pointer_world: Option<Point>,
    pub(crate) controls_held: [bool; 2],
    pub(crate) shifts_held: [bool; 2],
    pub(crate) fullscreen_requested: bool,
    pub(crate) last_viewport: Option<LogicalViewport>,
    pub(crate) run: Option<RunSession>,
    pub(crate) environment_open: bool,
    pub(crate) notice: Option<String>,
    pub(crate) link_start: Option<Attachment>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            project_name: "Untitled scene".into(),
            document: Document::default(),
            mode: Mode::Editor,
            tool: Tool::Build,
            brush: Placement::Primitive(ObjectKind::Ball),
            catalog: CatalogState::default(),
            catalog_pointer: PointerButton::new(MouseButton::Left),
            selected: None,
            selection: BTreeSet::new(),
            selection_gesture: None,
            camera: Camera::default(),
            snap: true,
            hovered: None,
            emphasis: [0.0; Control::ALL.len()],
            status: "Choose an object, then click or drag it onto the canvas.",
            drag: None,
            pan: None,
            pointer: PointerButton::new(MouseButton::Left),
            pointer_world: None,
            controls_held: [false; 2],
            shifts_held: [false; 2],
            fullscreen_requested: true,
            last_viewport: None,
            run: None,
            environment_open: false,
            notice: None,
            link_start: None,
        }
    }
}

impl EditorState {
    pub(crate) fn selected_object(&self) -> Option<&Object> {
        if self.selection.len() > 1 {
            return None;
        }
        self.selected.and_then(|id| self.document.object(id))
    }
    pub(crate) fn select_one(&mut self, selected: Option<u64>) {
        self.selected = selected;
        self.selection.clear();
        self.selection.extend(selected);
    }

    pub(crate) fn is_selected(&self, id: u64) -> bool {
        self.selected == Some(id) || self.selection.contains(&id)
    }
    pub(crate) fn display_position(&self, object: &Object) -> Point {
        if self.mode == Mode::Preview
            && let Some(run) = &self.run
        {
            return run.position(object.id).unwrap_or(object.position);
        }
        match self.drag {
            Some(Drag::MoveSelection { delta, .. }) if self.is_selected(object.id) => {
                Point::new(object.position.x + delta.x, object.position.y + delta.y)
            }
            Some(Drag::Move { id, position, .. }) if id == object.id => position,
            _ => object.position,
        }
    }
    /// Reads orientation from the independent Run; authoring never advances it.
    pub(crate) fn display_rotation_deg(&self, object: &Object) -> f64 {
        if self.mode == Mode::Preview
            && let Some(run) = &self.run
        {
            return run.angle_deg(object.id).unwrap_or(object.rotation_deg);
        }
        object.rotation_deg
    }

    /// Resolves a material endpoint through the displayed pose, including drag.
    pub(crate) fn display_attachment(&self, attachment: Attachment) -> Option<Point> {
        let object = self.document.object(attachment.body)?;
        Some(world_point(
            self.display_position(object),
            self.display_rotation_deg(object),
            attachment.local_m,
        ))
    }
    pub(crate) fn ghost(&self) -> Option<(ObjectKind, Point)> {
        if self.mode != Mode::Editor {
            return None;
        }
        match self.drag {
            Some(Drag::Place {
                placement,
                position,
                ..
            }) => self.pointer_world.map(|_| (placement.marker(), position)),
            None if self.tool == Tool::Build => self
                .pointer_world
                .map(|point| (self.brush.marker(), self.snapped(point))),
            _ => None,
        }
    }
    pub(crate) fn snapped(&self, point: Point) -> Point {
        if self.snap {
            Point::new((point.x * 2.0).round() / 2.0, (point.y * 2.0).round() / 2.0)
        } else {
            point
        }
    }
    pub(crate) fn active(&self, control: Control) -> bool {
        match control {
            Control::Tool(tool) => self.tool == tool,
            Control::Palette(kind) => self.brush == Placement::Primitive(kind),
            Control::Snap => self.snap,
            Control::Fixed => self.selected_object().is_some_and(|object| object.fixed),
            Control::Pause => self.run.as_ref().is_some_and(|run| run.paused),
            Control::Slow => self
                .run
                .as_ref()
                .is_some_and(|run| run.speed == PlaybackSpeed::Quarter),
            Control::Normal => self
                .run
                .as_ref()
                .is_some_and(|run| run.speed == PlaybackSpeed::Normal),
            Control::Fast => self
                .run
                .as_ref()
                .is_some_and(|run| run.speed == PlaybackSpeed::Fast),
            _ => false,
        }
    }
    pub(crate) fn enabled(&self, control: Control) -> bool {
        if self.environment_open {
            return control.is_environment();
        }
        if control.is_environment() {
            return false;
        }
        if self.mode == Mode::Preview {
            return matches!(
                control,
                Control::Back
                    | Control::Home
                    | Control::Environment
                    | Control::Pause
                    | Control::Slow
                    | Control::Normal
                    | Control::Fast
            );
        }
        match control {
            Control::Back | Control::Pause | Control::Slow | Control::Normal | Control::Fast => {
                false
            }
            Control::Undo => self.document.can_undo(),
            Control::Redo => self.document.can_redo(),
            Control::Delete => self.selected.is_some() || !self.selection.is_empty(),
            Control::Duplicate | Control::SizeDown | Control::SizeUp => {
                self.selected_object().is_some()
            }
            Control::HeightDown | Control::HeightUp => self
                .selected_object()
                .is_some_and(|object| object.kind == ObjectKind::Box),
            Control::Fixed
            | Control::RestitutionDown
            | Control::RestitutionUp
            | Control::RotateLeft
            | Control::RotateRight => self
                .selected_object()
                .is_some_and(|object| object.kind != ObjectKind::Anchor),
            Control::MassDown | Control::MassUp => self
                .selected_object()
                .is_some_and(|object| object.kind != ObjectKind::Anchor),
            _ => true,
        }
    }
    pub(crate) fn cancel_gesture(&mut self) {
        self.drag = None;
        self.selection_gesture = None;
        self.pan = None;
        self.pointer.cancel();
        self.catalog_pointer.cancel();
        self.catalog.pressed = None;
        self.link_start = None;
    }

    pub(crate) fn environment(&self) -> PhysicsEnvironment {
        if self.mode == Mode::Preview
            && let Some(run) = &self.run
        {
            run.environment()
        } else {
            self.document.environment()
        }
    }
    pub(crate) fn pick(&self, point: Point) -> Option<u64> {
        self.document
            .objects()
            .iter()
            .rev()
            .find(|object| {
                let dx = point.x - object.position.x;
                let dy = point.y - object.position.y;
                let angle = object.rotation_deg.to_radians();
                let local_x = dx * angle.cos() + dy * angle.sin();
                let local_y = -dx * angle.sin() + dy * angle.cos();
                let half = object.size_m * 0.5;
                match object.kind {
                    ObjectKind::Ball => dx.hypot(dy) <= half,
                    ObjectKind::Box => {
                        local_x.abs() <= half && local_y.abs() <= object.height_m * 0.5
                    }
                    ObjectKind::Anchor => local_x.abs() <= half && local_y.abs() <= half,
                }
            })
            .map(|object| object.id)
    }

    /// Uses the same material-point geometry for spring previews and clicks.
    /// Spring corners/surfaces snap within eight logical pixels; rods remain
    /// centre-only. Topmost eligible body wins, matching ordinary picking.
    pub(crate) fn pick_attachment(&self, point: Point) -> Option<Attachment> {
        if self.tool != Tool::Spring {
            return self.pick(point).map(Attachment::center);
        }
        self.document.objects().iter().rev().find_map(|object| {
            pick_surface(object, point, 8.0 / self.camera.pixels_per_m).map(|local_m| Attachment {
                body: object.id,
                local_m,
            })
        })
    }
}

#[cfg(test)]
mod collision_authoring_tests {
    use super::*;

    #[test]
    fn rotated_rectangle_picking_uses_physical_height_not_width() {
        let mut state = EditorState::default();
        let id = state
            .document
            .add(ObjectKind::Box, Point::new(2.0, 3.0))
            .unwrap();
        state.document.set_size(id, 8.0).unwrap();
        state.document.set_height(id, 0.5).unwrap();
        assert_eq!(state.pick(Point::new(5.9, 3.2)), Some(id));
        assert_eq!(state.pick(Point::new(2.0, 3.3)), None);
        state.document.rotate(id, 90.0).unwrap();
        assert_eq!(state.pick(Point::new(2.2, 6.9)), Some(id));
        assert_eq!(state.pick(Point::new(2.3, 3.0)), None);
        assert_eq!(state.pick(Point::new(5.0, 3.0)), None);
    }

    #[test]
    fn collider_controls_respect_kind_mode_and_multiple_selection() {
        let mut state = EditorState::default();
        let block = state
            .document
            .add(ObjectKind::Box, Point::default())
            .unwrap();
        let ball = state
            .document
            .add(ObjectKind::Ball, Point::new(4.0, 0.0))
            .unwrap();
        let anchor = state
            .document
            .add(ObjectKind::Anchor, Point::new(8.0, 0.0))
            .unwrap();
        let controls = [
            Control::Fixed,
            Control::HeightDown,
            Control::HeightUp,
            Control::RestitutionDown,
            Control::RestitutionUp,
        ];
        state.select_one(Some(block));
        assert!(controls.into_iter().all(|control| state.enabled(control)));
        state.document.set_fixed(block, true).unwrap();
        assert!(state.active(Control::Fixed));
        state.select_one(Some(ball));
        assert!(!state.enabled(Control::HeightUp));
        assert!(state.enabled(Control::RestitutionUp));
        assert!(state.enabled(Control::RotateRight));
        state.select_one(Some(anchor));
        assert!(controls.into_iter().all(|control| !state.enabled(control)));
        assert!(!state.enabled(Control::RotateRight));
        state.select_one(Some(block));
        state.selection.insert(ball);
        assert!(controls.into_iter().all(|control| !state.enabled(control)));
        state.select_one(Some(block));
        state.mode = Mode::Preview;
        assert!(controls.into_iter().all(|control| !state.enabled(control)));
    }

    #[test]
    fn pending_attachment_uses_displayed_drag_position_and_authored_rotation() {
        let mut state = EditorState::default();
        let body = state
            .document
            .add(ObjectKind::Box, Point::new(2.0, 3.0))
            .unwrap();
        state.document.rotate(body, 90.0).unwrap();
        let attachment = Attachment {
            body,
            local_m: Point::new(0.5, 0.5),
        };
        assert_eq!(
            state.display_attachment(attachment),
            Some(Point::new(1.5, 3.5))
        );
        state.drag = Some(Drag::Move {
            id: body,
            offset: Point::default(),
            position: Point::new(8.0, -2.0),
        });
        assert_eq!(
            state.display_attachment(attachment),
            Some(Point::new(7.5, -1.5))
        );
        assert_eq!(
            state.document.object(body).unwrap().position,
            Point::new(2.0, 3.0)
        );
        state.select_one(Some(body));
        state.drag = Some(Drag::MoveSelection {
            origin: Point::default(),
            delta: Point::new(-2.0, 4.0),
        });
        let position = state.display_attachment(attachment).unwrap();
        assert!((position.x + 0.5).abs() < 1e-12);
        assert_eq!(position.y, 7.5);
        assert!(
            state
                .display_attachment(Attachment::center(u64::MAX))
                .is_none()
        );
    }

    #[test]
    fn spring_surface_snap_radius_is_in_logical_pixels_and_rods_stay_centred() {
        let mut state = EditorState::default();
        let body = state
            .document
            .add(ObjectKind::Box, Point::default())
            .unwrap();
        state.tool = Tool::Spring;
        for pixels_per_m in [8.0, 80.0, 240.0] {
            state.camera.pixels_per_m = pixels_per_m;
            let near_corner = Point::new(0.5 + 4.0 / pixels_per_m, 0.5 + 4.0 / pixels_per_m);
            assert_eq!(
                state.pick_attachment(near_corner),
                Some(Attachment {
                    body,
                    local_m: Point::new(0.5, 0.5)
                })
            );
            assert!(
                state
                    .pick_attachment(Point::new(0.5 + 9.0 / pixels_per_m, 0.0))
                    .is_none()
            );
        }
        state.tool = Tool::Rod;
        assert_eq!(
            state.pick_attachment(Point::new(0.4, 0.4)),
            Some(Attachment::center(body))
        );
    }
}
