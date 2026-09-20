//! Bounded retained editor visuals; authoring data never depends on these types.

use sim_logic::prelude::*;

use super::{
    assets::EditorAssets,
    attachment::Attachment,
    document::{LinkKind, MAX_LINKS, MAX_OBJECTS, ObjectKind, Point},
    layout::{Layout, Rect},
    state::{Control, EditorState, Mode, Tool},
};

const OBJECT_SLOTS: usize = MAX_OBJECTS + 1;
const GRID_PER_AXIS: usize = 64;
const LINK_SEGMENTS: usize = 8;
const SELECTION_SEGMENTS: usize = 256;
pub(crate) const LINE_COUNT: usize =
    GRID_PER_AXIS * 2 + 2 + OBJECT_SLOTS * 6 + MAX_LINKS * LINK_SEGMENTS + 1 + SELECTION_SEGMENTS;
pub(crate) const CIRCLE_COUNT: usize = OBJECT_SLOTS + 1 + MAX_LINKS * 2 + 2;
pub(crate) const RECT_COUNT: usize = 6 + Control::ALL.len() * 2 + 2;
pub(crate) const TEXT_COUNT: usize = Control::ALL.len() + 36;
pub(crate) const ENTITY_COUNT: usize = LINE_COUNT + CIRCLE_COUNT + RECT_COUNT + TEXT_COUNT + 1;

const BACKGROUND: Color = Color::rgb(0.003677, 0.005182, 0.008023);
const INK: Color = Color::rgb(0.822786, 0.871367, 0.863157);
const MUTED: Color = Color::rgb(0.270498, 0.341914, 0.401978);
const ACCENT: Color = Color::rgb(0.258183, 0.723055, 0.508881);

#[derive(Clone, Copy)]
pub(crate) enum Panel {
    Header,
    Toolbar,
    Palette,
    Inspector,
    ModalScrim,
    Environment,
}

#[derive(Clone, Copy)]
pub(crate) enum Caption {
    Brand,
    Project,
    Unsaved,
    Mode,
    ToolbarHint,
    PaletteTitle,
    PaletteHint,
    PaletteDetail,
    PaletteCancel,
    InspectorTitle,
    Selection,
    SelectionDetail,
    PositionX,
    PositionY,
    MassTitle,
    MassValue,
    SizeTitle,
    SizeValue,
    HeightTitle,
    HeightValue,
    RestitutionTitle,
    RestitutionValue,
    Rotation,
    InspectorHint,
    EmptyTitle,
    EmptyDetail,
    Status,
    Metrics,
    EnvironmentTitle,
    EnvironmentScope,
    GravityX,
    GravityY,
    LinearDrag,
    EnvironmentModel,
    EnvironmentNote,
    RuntimeStats,
}

#[derive(Component, Clone, Copy)]
pub(crate) enum VisualSlot {
    Panel(Panel),
    Button(Control),
    Accent(Control),
    ButtonText(Control),
    Text(Caption),
    Grid { axis: u8, index: usize },
    Axis(u8),
    ObjectEdge { index: usize, edge: usize },
    ObjectDisc(usize),
    PaletteIcon(ObjectKind),
    LinkEdge { index: usize, edge: usize },
    LinkAnchor { index: usize, end: usize },
    PendingLink,
    PendingAnchor(usize),
    SelectionEdge(usize),
}

fn empty_rectangle() -> LogicResult<ScreenRectangleVisual> {
    let mut visual = ScreenRectangleVisual::new(
        LogicalScreenPosition::new(0.0, 0.0),
        LogicalScreenVector::new(1.0, 1.0),
        Color::TRANSPARENT,
    )?;
    visual.set_draw_order_depth(4.0)?;
    visual.set_clip(ScreenClip::Empty);
    Ok(visual)
}

fn empty_line() -> LogicResult<ScreenLineVisual> {
    let mut visual = ScreenLineVisual::new(
        LogicalScreenPosition::new(0.0, 0.0),
        LogicalScreenPosition::new(1.0, 0.0),
        1.0,
        Color::TRANSPARENT,
    )?;
    visual.set_clip(ScreenClip::Empty);
    Ok(visual)
}

fn empty_circle() -> LogicResult<ScreenCircleVisual> {
    let mut visual = ScreenCircleVisual::new(
        LogicalScreenPosition::new(0.0, 0.0),
        1.0,
        Color::TRANSPARENT,
    )?;
    visual.set_draw_order_depth(2.0)?;
    visual.set_clip(ScreenClip::Empty);
    Ok(visual)
}

fn empty_text(font: &TextFont) -> LogicResult<ScreenTextVisual> {
    let mut visual = ScreenTextVisual::new(font.clone(), "", LogicalScreenPosition::new(0.0, 0.0))?;
    visual.set_draw_order_depth(6.0)?;
    visual.set_clip(ScreenClip::Empty);
    Ok(visual)
}

pub(crate) fn spawn(world: &mut WorldBuilder, assets: &EditorAssets) -> LogicResult {
    world.spawn(ActiveCamera2d::centered(1.0)?)?;
    world.insert_resource(WorldBackground::new(BACKGROUND)?)?;
    for panel in [
        Panel::Header,
        Panel::Toolbar,
        Panel::Palette,
        Panel::Inspector,
        Panel::ModalScrim,
        Panel::Environment,
    ] {
        world.spawn((VisualSlot::Panel(panel), empty_rectangle()?))?;
    }
    for control in Control::ALL {
        world.spawn((VisualSlot::Button(control), empty_rectangle()?))?;
        world.spawn((VisualSlot::Accent(control), empty_rectangle()?))?;
        world.spawn((VisualSlot::ButtonText(control), empty_text(&assets.body)?))?;
    }
    for caption in [
        Caption::Brand,
        Caption::Project,
        Caption::Unsaved,
        Caption::Mode,
        Caption::ToolbarHint,
        Caption::PaletteTitle,
        Caption::PaletteHint,
        Caption::PaletteDetail,
        Caption::PaletteCancel,
        Caption::InspectorTitle,
        Caption::Selection,
        Caption::SelectionDetail,
        Caption::PositionX,
        Caption::PositionY,
        Caption::MassTitle,
        Caption::MassValue,
        Caption::SizeTitle,
        Caption::SizeValue,
        Caption::HeightTitle,
        Caption::HeightValue,
        Caption::RestitutionTitle,
        Caption::RestitutionValue,
        Caption::Rotation,
        Caption::InspectorHint,
        Caption::EmptyTitle,
        Caption::EmptyDetail,
        Caption::Status,
        Caption::Metrics,
        Caption::EnvironmentTitle,
        Caption::EnvironmentScope,
        Caption::GravityX,
        Caption::GravityY,
        Caption::LinearDrag,
        Caption::EnvironmentModel,
        Caption::EnvironmentNote,
        Caption::RuntimeStats,
    ] {
        let font = match caption {
            Caption::Brand
            | Caption::Selection
            | Caption::EmptyTitle
            | Caption::EnvironmentTitle => &assets.heading,
            Caption::Project
            | Caption::MassValue
            | Caption::SizeValue
            | Caption::HeightValue
            | Caption::RestitutionValue => &assets.body,
            _ => &assets.small,
        };
        world.spawn((VisualSlot::Text(caption), empty_text(font)?))?;
    }
    for axis in 0..2 {
        world.spawn((VisualSlot::Axis(axis), empty_line()?))?;
        for index in 0..GRID_PER_AXIS {
            world.spawn((VisualSlot::Grid { axis, index }, empty_line()?))?;
        }
    }
    for index in 0..OBJECT_SLOTS {
        world.spawn((VisualSlot::ObjectDisc(index), empty_circle()?))?;
        for edge in 0..6 {
            world.spawn((VisualSlot::ObjectEdge { index, edge }, empty_line()?))?;
        }
    }
    for index in 0..MAX_LINKS {
        for end in 0..2 {
            world.spawn((VisualSlot::LinkAnchor { index, end }, empty_circle()?))?;
        }
        for edge in 0..LINK_SEGMENTS {
            world.spawn((VisualSlot::LinkEdge { index, edge }, empty_line()?))?;
        }
    }
    world.spawn((VisualSlot::PendingLink, empty_line()?))?;
    for end in 0..2 {
        world.spawn((VisualSlot::PendingAnchor(end), empty_circle()?))?;
    }
    for index in 0..SELECTION_SEGMENTS {
        world.spawn((VisualSlot::SelectionEdge(index), empty_line()?))?;
    }
    for kind in ObjectKind::ALL {
        if kind == ObjectKind::Ball {
            world.spawn((VisualSlot::PaletteIcon(kind), empty_circle()?))?;
        } else {
            world.spawn((VisualSlot::PaletteIcon(kind), empty_rectangle()?))?;
        }
    }
    Ok(())
}

fn clip(rect: Rect) -> LogicResult<ScreenClip> {
    Ok(ScreenClip::new(rect.position(), rect.size())?)
}

fn control_color(control: Control, state: &EditorState) -> Color {
    let emphasis =
        state.emphasis[control.index()].max(if state.active(control) { 0.6 } else { 0.0 });
    let primary = matches!(
        control,
        Control::Run | Control::Back | Control::CloseEnvironment
    );
    if primary {
        return Color::rgb8(
            (39.0 + emphasis * 10.0) as u8,
            (91.0 + emphasis * 25.0) as u8,
            (78.0 + emphasis * 16.0) as u8,
        );
    }
    Color::rgb8(
        (24.0 + emphasis * 14.0) as u8,
        (33.0 + emphasis * 32.0) as u8,
        (42.0 + emphasis * 27.0) as u8,
    )
}

fn rectangle_style(
    slot: VisualSlot,
    layout: &Layout,
    state: &EditorState,
) -> Option<(Rect, Color, f32)> {
    match slot {
        VisualSlot::Panel(panel) => {
            if matches!(panel, Panel::ModalScrim | Panel::Environment) {
                if !state.environment_open {
                    return None;
                }
                return Some(if matches!(panel, Panel::ModalScrim) {
                    (
                        Rect::new(0.0, 0.0, layout.width, layout.height),
                        Color::rgb8(3, 7, 11).with_alpha(0.82),
                        0.0,
                    )
                } else {
                    (layout.modal, Color::rgb8(22, 31, 41), 12.0)
                });
            }
            let rect = match panel {
                Panel::Header => Rect::new(0.0, 0.0, layout.width, 78.0),
                Panel::Toolbar => Rect::new(0.0, 78.0, layout.width, 48.0),
                Panel::Palette if state.mode == Mode::Editor => layout.palette,
                Panel::Inspector if state.mode == Mode::Editor => layout.inspector,
                _ => return None,
            };
            Some((
                rect,
                Color::rgb8(17, 24, 32),
                if matches!(panel, Panel::Palette | Panel::Inspector) {
                    10.0
                } else {
                    0.0
                },
            ))
        }
        VisualSlot::Button(control) | VisualSlot::Accent(control) => {
            let mut rect = layout.control(control)?;
            if matches!(slot, VisualSlot::Accent(_)) {
                let weight = state.emphasis[control.index()].max(if state.active(control) {
                    1.0
                } else {
                    0.0
                });
                if weight <= 0.001 || !state.enabled(control) {
                    return None;
                }
                rect.x += 10.0;
                rect.y += rect.height - 2.0;
                rect.width -= 20.0;
                rect.height = 2.0;
                Some((rect, ACCENT.with_alpha(weight), 0.0))
            } else {
                Some((
                    rect,
                    control_color(control, state).with_alpha(if state.enabled(control) {
                        1.0
                    } else {
                        0.35
                    }),
                    6.0,
                ))
            }
        }
        VisualSlot::PaletteIcon(kind) => {
            let card = layout.control(Control::Palette(kind))?;
            Some((
                Rect::new(card.x + card.width * 0.5 - 12.0, card.y + 14.0, 24.0, 24.0),
                if kind == ObjectKind::Anchor {
                    Color::rgb8(230, 188, 121)
                } else {
                    ACCENT
                },
                if kind == ObjectKind::Box { 3.0 } else { 0.0 },
            ))
        }
        _ => None,
    }
}

struct CaptionStyle {
    text: String,
    position: LogicalScreenPosition,
    color: Color,
    alignment: TextAlignment,
    bounds: Rect,
    depth: f32,
}

fn caption_style(caption: Caption, layout: &Layout, state: &EditorState) -> Option<CaptionStyle> {
    if let Some(style) = environment_caption(caption, layout, state) {
        return Some(style);
    }
    let editor = state.mode == Mode::Editor;
    let selected = state.selected_object();
    let inspector = layout.inspector;
    let canvas = layout.canvas;
    let mut bounds = Rect::new(0.0, 0.0, layout.width, layout.height);
    let mut alignment = TextAlignment::Left;
    let inspector_caption = matches!(
        caption,
        Caption::InspectorTitle
            | Caption::Selection
            | Caption::SelectionDetail
            | Caption::PositionX
            | Caption::PositionY
            | Caption::MassTitle
            | Caption::MassValue
            | Caption::SizeTitle
            | Caption::SizeValue
            | Caption::HeightTitle
            | Caption::HeightValue
            | Caption::RestitutionTitle
            | Caption::RestitutionValue
            | Caption::Rotation
            | Caption::InspectorHint
    );
    if inspector_caption {
        if !editor {
            return None;
        }
        bounds = inspector;
    }
    let (text, x, y, color) = match caption {
        Caption::Brand => ("Sim;Phys".into(), 22.0, 44.0, ACCENT),
        Caption::Project => {
            bounds = Rect::new(174.0, 16.0, (layout.width - 530.0).max(0.0), 28.0);
            (state.project_name.clone(), 174.0, 36.0, INK)
        }
        Caption::Unsaved => ("LOCAL / UNSAVED PROTOTYPE".into(), 174.0, 57.0, MUTED),
        Caption::Mode => {
            bounds = canvas;
            let text = if editor {
                if state.tool == Tool::Spring {
                    if state.link_start.is_some() {
                        "SPRING / Choose the second attachment; Esc cancels."
                    } else {
                        "SPRING / Click an edge or corner to attach."
                    }
                } else if state.tool == Tool::Rod {
                    if state.link_start.is_some() {
                        "CONNECTION / Select the second body; Esc cancels."
                    } else {
                        "CONNECTION / Select the first body."
                    }
                } else {
                    "EDITOR / STATIC AUTHORING"
                }
            } else if state.run.as_ref().is_some_and(|run| run.failure.is_some()) {
                "SIMULATION / STOPPED ON VALIDATION ERROR"
            } else if state.run.as_ref().is_some_and(|run| run.paused) {
                "SIMULATION / PAUSED"
            } else {
                "SIMULATION / RUNNING"
            };
            (text.into(), canvas.x + 14.0, canvas.y + 26.0, MUTED)
        }
        Caption::ToolbarHint => {
            if editor && layout.width < 1100.0 {
                return None;
            }
            (
                if editor {
                    "MMB pan   /   Wheel zoom   /   Home reset"
                } else {
                    "2D rotation / frictionless contacts"
                }
                .into(),
                if editor { 710.0 } else { 112.0 },
                109.0,
                if editor {
                    MUTED
                } else {
                    Color::rgb8(230, 188, 121)
                },
            )
        }
        Caption::PaletteTitle
        | Caption::PaletteHint
        | Caption::PaletteDetail
        | Caption::PaletteCancel => return None,
        Caption::InspectorTitle => (
            "INSPECTOR".into(),
            inspector.x + 16.0,
            inspector.y + 28.0,
            MUTED,
        ),
        Caption::Selection => (
            if state.selection.len() > 1 {
                format!("{} objects selected", state.selection.len())
            } else {
                selected
                    .map_or("No selection", |object| object.kind.label())
                    .into()
            },
            inspector.x + 16.0,
            inspector.y + 54.0,
            INK,
        ),
        Caption::SelectionDetail => (
            if state.selection.len() > 1 {
                "Drag together / Delete / Undo".into()
            } else {
                selected.map_or_else(
                    || "Click an object to inspect it.".into(),
                    |object| {
                        format!(
                            "OBJECT {:03}  /  {}",
                            object.id,
                            if object.fixed { "FIXED" } else { "DYNAMIC" }
                        )
                    },
                )
            },
            inspector.x + 16.0,
            inspector.y + 78.0,
            MUTED,
        ),
        Caption::PositionX | Caption::PositionY => {
            let x_axis = matches!(caption, Caption::PositionX);
            let value = selected
                .map(|object| state.display_position(object))
                .map(|point| if x_axis { point.x } else { point.y });
            (
                format!(
                    "{}    {}",
                    if x_axis { "X" } else { "Y" },
                    value.map_or_else(|| "--".into(), |value| format!("{value:.2} m"))
                ),
                inspector.x + 16.0,
                inspector.y + if x_axis { 102.0 } else { 122.0 },
                MUTED,
            )
        }
        Caption::MassTitle => (
            "MASS".into(),
            inspector.x + 16.0,
            inspector.y + 190.0,
            MUTED,
        ),
        Caption::MassValue => (
            selected.map_or_else(
                || "--".into(),
                |object| {
                    if object.kind == ObjectKind::Anchor {
                        "Fixed anchor".into()
                    } else if object.mass_kg >= 1000.0 {
                        format!("{:.2e} kg", object.mass_kg)
                    } else {
                        format!("{:.3} kg", object.mass_kg)
                    }
                },
            ),
            inspector.x + 16.0,
            inspector.y + 207.0,
            INK,
        ),
        Caption::SizeTitle => (
            selected
                .map_or("SIZE", |object| match object.kind {
                    ObjectKind::Ball => "DIAMETER",
                    ObjectKind::Box => "WIDTH",
                    ObjectKind::Anchor => "MARKER SIZE",
                })
                .into(),
            inspector.x + 16.0,
            inspector.y + 226.0,
            MUTED,
        ),
        Caption::SizeValue => (
            selected.map_or_else(|| "--".into(), |object| format!("{:.2} m", object.size_m)),
            inspector.x + 16.0,
            inspector.y + 243.0,
            INK,
        ),
        Caption::HeightTitle => (
            "HEIGHT".into(),
            inspector.x + 16.0,
            inspector.y + 262.0,
            MUTED,
        ),
        Caption::HeightValue => (
            selected
                .filter(|object| object.kind == ObjectKind::Box)
                .map_or_else(|| "--".into(), |object| format!("{:.2} m", object.height_m)),
            inspector.x + 16.0,
            inspector.y + 279.0,
            INK,
        ),
        Caption::RestitutionTitle => (
            "RESTITUTION".into(),
            inspector.x + 16.0,
            inspector.y + 298.0,
            MUTED,
        ),
        Caption::RestitutionValue => (
            selected
                .filter(|object| object.kind != ObjectKind::Anchor)
                .map_or_else(
                    || "--".into(),
                    |object| format!("{:.2}", object.restitution),
                ),
            inspector.x + 16.0,
            inspector.y + 315.0,
            INK,
        ),
        Caption::Rotation => (
            selected
                .filter(|object| object.kind != ObjectKind::Anchor)
                .map_or_else(
                    || "ANGLE --".into(),
                    |object| format!("{:.0} deg", object.rotation_deg),
                ),
            inspector.x + 16.0,
            inspector.y + 344.0,
            MUTED,
        ),
        Caption::InspectorHint => (
            "2D bodies; no friction.".into(),
            inspector.x + 16.0,
            inspector.y + inspector.height - 18.0,
            MUTED,
        ),
        Caption::EmptyTitle | Caption::EmptyDetail => {
            if !state.document.objects().is_empty() {
                return None;
            }
            alignment = TextAlignment::Center;
            bounds = canvas;
            let title = matches!(caption, Caption::EmptyTitle);
            (
                if editor {
                    if title {
                        "A space to build."
                    } else {
                        "Choose from quick access below, or press / for the full catalog."
                    }
                } else if title {
                    "Your scene is empty."
                } else {
                    "Return to the editor to add objects."
                }
                .into(),
                canvas.x + canvas.width * 0.5,
                canvas.y + canvas.height * 0.5 + if title { -8.0 } else { 20.0 },
                if title { INK.with_alpha(0.7) } else { MUTED },
            )
        }
        Caption::Status => {
            bounds = Rect::new(
                16.0,
                layout.height - 28.0,
                (layout.width - 410.0).max(1.0),
                28.0,
            );
            let failure = state.run.as_ref().and_then(|run| run.failure.as_deref());
            let text = failure.or(state.notice.as_deref()).unwrap_or(state.status);
            (
                text.into(),
                16.0,
                layout.height - 11.0,
                if failure.is_some() {
                    Color::rgb8(239, 163, 120)
                } else {
                    MUTED
                },
            )
        }
        Caption::Metrics => {
            alignment = TextAlignment::Right;
            bounds = Rect::new(
                (layout.width - 380.0).max(0.0),
                layout.height - 28.0,
                364.0,
                28.0,
            );
            (
                format!(
                    "{} objects / {} links  |  {:.0} px/m",
                    state.document.objects().len(),
                    state.document.links().len(),
                    state.camera.pixels_per_m,
                ),
                layout.width - 16.0,
                layout.height - 11.0,
                MUTED,
            )
        }
        Caption::RuntimeStats => {
            if editor {
                return None;
            }
            let run = state.run.as_ref()?;
            bounds = Rect::new(
                400.0,
                layout.height - 82.0,
                (layout.width - 416.0).max(1.0),
                54.0,
            );
            let text = if run.failure.is_some() {
                "Run stopped. Return to Editor to revise the scene.".into()
            } else if run.dropped_simulation_s > 0.001 {
                format!(
                    "t {:.2} s / step {}  |  {:.2} s overload time skipped",
                    run.world().elapsed_s(),
                    run.world().step_index(),
                    run.dropped_simulation_s
                )
            } else if let Some(report) = run.world().last_report() {
                format!(
                    "t {:.2} s / step {}  |  E {:.3e} J",
                    report.elapsed_s, report.step_index, report.energy.total_j
                )
            } else {
                format!(
                    "t {:.2} s / step {}  |  Energy after first step",
                    run.world().elapsed_s(),
                    run.world().step_index()
                )
            };
            (text, 400.0, layout.height - 54.0, MUTED)
        }
        Caption::EnvironmentTitle
        | Caption::EnvironmentScope
        | Caption::GravityX
        | Caption::GravityY
        | Caption::LinearDrag
        | Caption::EnvironmentModel
        | Caption::EnvironmentNote => return None,
    };
    Some(CaptionStyle {
        text,
        position: LogicalScreenPosition::new(x, y),
        color,
        alignment,
        bounds,
        depth: 6.0,
    })
}

fn environment_caption(
    caption: Caption,
    layout: &Layout,
    state: &EditorState,
) -> Option<CaptionStyle> {
    if !state.environment_open {
        return None;
    }
    let environment = state.environment();
    let (text, offset_y, color) = match caption {
        Caption::EnvironmentTitle => ("Environment".into(), 40.0, INK),
        Caption::EnvironmentScope => (
            if state.mode == Mode::Editor {
                "Scene parameters used by every new Run."
            } else {
                "Changes apply to this run at the next step."
            }
            .into(),
            68.0,
            MUTED,
        ),
        Caption::GravityX => (
            format!("Gravity X    {:+.3} m/s²", environment.gravity_m_s2.x),
            117.0,
            INK,
        ),
        Caption::GravityY => (
            format!("Gravity Y    {:+.3} m/s²", environment.gravity_m_s2.y),
            171.0,
            INK,
        ),
        Caption::LinearDrag => (
            format!("Linear drag    {:.2} /s", environment.linear_drag_per_s),
            225.0,
            INK,
        ),
        Caption::EnvironmentModel => (
            "2D rigid bodies; frictionless contacts.".into(),
            310.0,
            MUTED,
        ),
        Caption::EnvironmentNote => (
            if state.mode == Mode::Editor {
                "Teacher instruments and full nerd-mode are planned."
            } else {
                "The authored scene stays unchanged."
            }
            .into(),
            330.0,
            MUTED,
        ),
        _ => return None,
    };
    Some(CaptionStyle {
        text,
        position: LogicalScreenPosition::new(layout.modal.x + 24.0, layout.modal.y + offset_y),
        color,
        alignment: TextAlignment::Left,
        bounds: Rect::new(
            layout.modal.x + 20.0,
            layout.modal.y + 12.0,
            if matches!(
                caption,
                Caption::GravityX | Caption::GravityY | Caption::LinearDrag
            ) {
                290.0
            } else {
                layout.modal.width - 40.0
            },
            layout.modal.height - 24.0,
        ),
        depth: 10.0,
    })
}

#[derive(Clone, Copy)]
struct DisplayObject {
    kind: ObjectKind,
    center: LogicalScreenPosition,
    radius: f32,
    half_height: f32,
    angle: f64,
    color: Color,
}

fn display_objects(state: &EditorState, canvas: Rect) -> [Option<DisplayObject>; OBJECT_SLOTS] {
    std::array::from_fn(|index| {
        let (kind, position, size, height, angle, fixed, selected, ghost) = if index == MAX_OBJECTS
        {
            let (kind, position) = state.ghost()?;
            (
                kind,
                position,
                1.0,
                1.0,
                0.0,
                kind == ObjectKind::Anchor,
                false,
                true,
            )
        } else {
            let object = state.document.objects().get(index)?;
            (
                object.kind,
                state.display_position(object),
                object.size_m,
                if object.kind == ObjectKind::Box {
                    object.height_m
                } else {
                    object.size_m
                },
                state.display_rotation_deg(object),
                object.fixed,
                state.is_selected(object.id)
                    || state
                        .link_start
                        .is_some_and(|anchor| anchor.body == object.id),
                false,
            )
        };
        let center = state.camera.project(position, canvas);
        let radius = (size * state.camera.pixels_per_m * 0.5) as f32;
        let half_height = (height * state.camera.pixels_per_m * 0.5) as f32;
        let bounds = radius.hypot(half_height);
        let point = center.to_vec2();
        if point.x() + bounds < canvas.x
            || point.x() - bounds > canvas.x + canvas.width
            || point.y() + bounds < canvas.y
            || point.y() - bounds > canvas.y + canvas.height
        {
            return None;
        }
        let base = if fixed && selected {
            Color::rgb8(255, 224, 157)
        } else if fixed {
            Color::rgb8(230, 188, 121)
        } else if selected {
            ACCENT
        } else {
            Color::rgb8(101, 164, 148)
        };
        Some(DisplayObject {
            kind,
            center,
            radius,
            half_height,
            angle: angle.to_radians(),
            color: base.with_alpha(if ghost { 0.35 } else { 0.9 }),
        })
    })
}

#[derive(Clone, Copy)]
struct DisplayLink {
    from: LogicalScreenPosition,
    to: LogicalScreenPosition,
    spring: bool,
}

fn display_links(state: &EditorState, canvas: Rect) -> [Option<DisplayLink>; MAX_LINKS] {
    std::array::from_fn(|index| {
        let link = state.document.links().get(index)?;
        let a = state.display_attachment(Attachment {
            body: link.a,
            local_m: link.a_local_m,
        })?;
        let b = state.display_attachment(Attachment {
            body: link.b,
            local_m: link.b_local_m,
        })?;
        Some(DisplayLink {
            from: state.camera.project(a, canvas),
            to: state.camera.project(b, canvas),
            spring: matches!(link.kind, LinkKind::Spring { .. }),
        })
    })
}

/// Uses the same snapped attachment as input; no cosmetic endpoint shortening.
fn pending_attachment_position(state: &EditorState, end: usize) -> Option<Point> {
    if state.mode != Mode::Editor
        || state.environment_open
        || state.catalog.open
        || !matches!(state.tool, Tool::Spring | Tool::Rod)
    {
        return None;
    }
    if end == 0 {
        return state.display_attachment(state.link_start?);
    }
    let pointer = state.pointer_world?;
    state
        .pick_attachment(pointer)
        .and_then(|attachment| state.display_attachment(attachment))
        .or_else(|| state.link_start.map(|_| pointer))
}

fn link_edge(
    link: DisplayLink,
    edge: usize,
) -> Option<(LogicalScreenPosition, LogicalScreenPosition)> {
    if !link.spring {
        return (edge == 0).then_some((link.from, link.to));
    }
    let from = link.from.to_vec2();
    let delta = link.to.to_vec2() - from;
    let length = delta.x().hypot(delta.y());
    if !length.is_finite() || length < 0.5 {
        return None;
    }
    // A screen-space coil identifies a spring. It does not claim a physical
    // wire thickness, coil count, or extra attachment geometry.
    let amplitude = 6.0_f32.min(length / 12.0);
    let point = |index: usize| {
        let weight = index as f32 / LINK_SEGMENTS as f32;
        let offset = if index == 0 || index == LINK_SEGMENTS {
            0.0
        } else if index.is_multiple_of(2) {
            amplitude
        } else {
            -amplitude
        };
        LogicalScreenPosition::new(
            from.x() + delta.x() * weight - delta.y() / length * offset,
            from.y() + delta.y() * weight + delta.x() / length * offset,
        )
    };
    (edge < LINK_SEGMENTS).then(|| (point(edge), point(edge + 1)))
}

/// Crops before tessellation, not merely by GPU scissor. Degenerate subpixel
/// segments are omitted to keep the Engine geometry proof well-conditioned.
fn crop_segment(
    from: LogicalScreenPosition,
    to: LogicalScreenPosition,
    rect: Rect,
) -> Option<(LogicalScreenPosition, LogicalScreenPosition)> {
    let from = from.to_vec2();
    let to = to.to_vec2();
    let dx = to.x() - from.x();
    let dy = to.y() - from.y();
    let mut lower = 0.0_f32;
    let mut upper = 1.0_f32;
    for (p, q) in [
        (-dx, from.x() - rect.x),
        (dx, rect.x + rect.width - from.x()),
        (-dy, from.y() - rect.y),
        (dy, rect.y + rect.height - from.y()),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let ratio = q / p;
            if p < 0.0 {
                lower = lower.max(ratio);
            } else {
                upper = upper.min(ratio);
            }
            if lower > upper {
                return None;
            }
        }
    }
    let start = LogicalScreenPosition::new(from.x() + lower * dx, from.y() + lower * dy);
    let end = LogicalScreenPosition::new(from.x() + upper * dx, from.y() + upper * dy);
    let delta = end.to_vec2() - start.to_vec2();
    (delta.x().hypot(delta.y()) >= 0.5).then_some((start, end))
}

fn object_edge(
    object: DisplayObject,
    edge: usize,
) -> Option<(LogicalScreenPosition, LogicalScreenPosition)> {
    if object.kind == ObjectKind::Ball {
        // Orientation is physical even when a disk's silhouette is invariant.
        return (edge == 0).then(|| {
            let center = object.center.to_vec2();
            (
                object.center,
                LogicalScreenPosition::new(
                    (f64::from(center.x()) + f64::from(object.radius) * 0.65 * object.angle.cos())
                        as f32,
                    (f64::from(center.y()) - f64::from(object.radius) * 0.65 * object.angle.sin())
                        as f32,
                ),
            )
        });
    }
    let points = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let (from, to) = if edge < 4 {
        (points[edge], points[(edge + 1) % 4])
    } else if object.kind == ObjectKind::Anchor {
        if edge == 4 {
            ((-0.6, 0.0), (0.6, 0.0))
        } else {
            ((0.0, -0.6), (0.0, 0.6))
        }
    } else {
        return None;
    };
    let transform = |(x, y): (f32, f32)| {
        let center = object.center.to_vec2();
        LogicalScreenPosition::new(
            (f64::from(center.x())
                + f64::from(x * object.radius) * object.angle.cos()
                + f64::from(y * object.half_height) * object.angle.sin()) as f32,
            (f64::from(center.y()) - f64::from(x * object.radius) * object.angle.sin()
                + f64::from(y * object.half_height) * object.angle.cos()) as f32,
        )
    };
    Some((transform(from), transform(to)))
}

fn grid_step(state: &EditorState, canvas: Rect) -> f64 {
    let desired_pixels = 42.0_f64.max(f64::from(canvas.width.max(canvas.height)) / 60.0);
    let minimum = desired_pixels / state.camera.pixels_per_m;
    let power = 10.0_f64.powf(minimum.log10().floor());
    [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .find(|factor| factor * power >= minimum)
        .unwrap_or(10.0)
        * power
}

fn line_style(
    slot: VisualSlot,
    layout: &Layout,
    state: &EditorState,
    objects: &[Option<DisplayObject>; OBJECT_SLOTS],
    links: &[Option<DisplayLink>; MAX_LINKS],
    spacing: f64,
    selection: &[(Point, Point)],
) -> Option<(LogicalScreenPosition, LogicalScreenPosition, Color, f32)> {
    let canvas = layout.canvas;
    match slot {
        VisualSlot::SelectionEdge(index) => {
            let (from, to) = *selection.get(index)?;
            Some((
                state.camera.project(from, canvas),
                state.camera.project(to, canvas),
                ACCENT.with_alpha(0.8),
                1.5,
            ))
        }
        VisualSlot::Grid { axis, index } => {
            let lower = state.camera.unproject(
                LogicalScreenPosition::new(canvas.x, canvas.y + canvas.height),
                canvas,
            );
            let start = if axis == 0 { lower.x } else { lower.y };
            let coordinate = (start / spacing).floor() * spacing + index as f64 * spacing;
            if coordinate.abs() < spacing * 0.001 {
                return None;
            }
            let point = state
                .camera
                .project(
                    if axis == 0 {
                        Point::new(coordinate, 0.0)
                    } else {
                        Point::new(0.0, coordinate)
                    },
                    canvas,
                )
                .to_vec2();
            let (from, to) = if axis == 0 {
                (
                    LogicalScreenPosition::new(point.x(), canvas.y),
                    LogicalScreenPosition::new(point.x(), canvas.y + canvas.height),
                )
            } else {
                (
                    LogicalScreenPosition::new(canvas.x, point.y()),
                    LogicalScreenPosition::new(canvas.x + canvas.width, point.y()),
                )
            };
            Some((from, to, Color::rgb8(28, 41, 50), 1.0))
        }
        VisualSlot::Axis(axis) => {
            let origin = state.camera.project(Point::new(0.0, 0.0), canvas).to_vec2();
            let (from, to) = if axis == 0 {
                (
                    LogicalScreenPosition::new(canvas.x, origin.y()),
                    LogicalScreenPosition::new(canvas.x + canvas.width, origin.y()),
                )
            } else {
                (
                    LogicalScreenPosition::new(origin.x(), canvas.y),
                    LogicalScreenPosition::new(origin.x(), canvas.y + canvas.height),
                )
            };
            Some((from, to, Color::rgb8(48, 72, 80), 1.0))
        }
        VisualSlot::ObjectEdge { index, edge } => {
            let object = objects[index]?;
            let (from, to) = object_edge(object, edge)?;
            let color = if object.kind == ObjectKind::Ball {
                INK.with_alpha(0.55)
            } else {
                object.color
            };
            Some((from, to, color, 2.0))
        }
        VisualSlot::LinkEdge { index, edge } => {
            let link = links[index]?;
            let (from, to) = link_edge(link, edge)?;
            Some((
                from,
                to,
                if link.spring {
                    Color::rgb8(219, 173, 100)
                } else {
                    Color::rgb8(124, 175, 162)
                },
                2.0,
            ))
        }
        VisualSlot::PendingLink => {
            if state.mode != Mode::Editor || state.environment_open {
                return None;
            }
            let start = state.display_attachment(state.link_start?)?;
            let from = state.camera.project(start, canvas);
            let to = state
                .camera
                .project(pending_attachment_position(state, 1)?, canvas);
            Some((from, to, ACCENT.with_alpha(0.5), 1.5))
        }
        _ => None,
    }
}

fn object_depth(index: usize) -> f32 {
    // Match reverse-insertion picking across all primitive types. The final
    // slot is the placement ghost; the complete range stays below UI panels.
    1.0 + index as f32 / OBJECT_SLOTS as f32
}

pub(crate) fn refresh(
    viewport: FrameViewport,
    state: Option<Res<EditorState>>,
    mut rectangles: Query<(&VisualSlot, &mut ScreenRectangleVisual)>,
    mut texts: Query<(&VisualSlot, &mut ScreenTextVisual)>,
    mut lines: Query<(&VisualSlot, &mut ScreenLineVisual)>,
    mut circles: Query<(&VisualSlot, &mut ScreenCircleVisual)>,
) -> LogicResult {
    let Some(state) = state else {
        return Ok(());
    };
    let layout = Layout::for_state(viewport.logical(), &state);
    let viewport_clip = clip(Rect::new(0.0, 0.0, layout.width, layout.height))?;
    let canvas_clip = clip(layout.canvas)?;
    let objects = display_objects(&state, layout.canvas);
    let links = display_links(&state, layout.canvas);
    let spacing = grid_step(&state, layout.canvas);
    let selection = state
        .selection_gesture
        .as_ref()
        .map_or_else(Vec::new, |gesture| gesture.outline());
    for (slot, mut visual) in &mut rectangles {
        let style = if layout.usable() {
            rectangle_style(*slot, &layout, &state)
        } else {
            None
        };
        if let Some((rect, color, radius)) = style {
            visual.set_geometry(rect.position(), rect.size())?;
            visual.set_color(color)?;
            visual.set_corner_radius(radius)?;
            // ECS iteration is not a painter's order. Panels, buttons and
            // foreground marks need distinct depths even within one shape type.
            visual.set_draw_order_depth(match slot {
                VisualSlot::Panel(Panel::ModalScrim) => 7.0,
                VisualSlot::Panel(Panel::Environment) => 8.0,
                VisualSlot::Button(control) if control.is_environment() => 9.0,
                VisualSlot::Accent(control) if control.is_environment() => 9.5,
                VisualSlot::Panel(_) => 3.0,
                VisualSlot::Button(_) => 4.0,
                _ => 5.0,
            })?;
            visual.set_clip(viewport_clip);
        } else {
            visual.set_clip(ScreenClip::Empty);
        }
    }
    for (slot, mut visual) in &mut texts {
        visual.set_clip(ScreenClip::Empty);
        visual.set_draw_order_depth(6.0)?;
        if !layout.usable() {
            if matches!(slot, VisualSlot::Text(Caption::EmptyTitle)) {
                visual.set_text("Enlarge the window to at least 900 x 600")?;
                visual.set_alignment(TextAlignment::Center)?;
                visual.set_position(LogicalScreenPosition::new(
                    layout.width * 0.5,
                    layout.height * 0.5,
                ))?;
                visual.set_tint(INK)?;
                visual.set_clip(viewport_clip);
            }
            continue;
        }
        match slot {
            VisualSlot::ButtonText(control) => {
                let Some(rect) = layout.control(*control) else {
                    continue;
                };
                let label = if matches!(control, Control::Pause)
                    && state.run.as_ref().is_some_and(|run| run.paused)
                {
                    "Resume"
                } else {
                    control.label()
                };
                visual.set_text(label)?;
                if control.is_environment() {
                    visual.set_draw_order_depth(10.0)?;
                }
                visual.set_alignment(TextAlignment::Center)?;
                let center_y = rect.y
                    + if matches!(control, Control::Palette(_)) {
                        66.0
                    } else {
                        rect.height * 0.5
                    };
                let metrics = visual.metrics();
                visual.set_position(LogicalScreenPosition::new(
                    rect.x + rect.width * 0.5,
                    center_y + (metrics.ascent() + metrics.descent()) * 0.5,
                ))?;
                visual.set_tint(if state.enabled(*control) {
                    INK
                } else {
                    MUTED.with_alpha(0.45)
                })?;
                visual.set_clip(clip(rect)?);
            }
            VisualSlot::Text(caption) => {
                if let Some(style) = caption_style(*caption, &layout, &state) {
                    visual.set_text(&style.text)?;
                    visual.set_alignment(style.alignment)?;
                    visual.set_position(style.position)?;
                    visual.set_tint(style.color)?;
                    visual.set_draw_order_depth(style.depth)?;
                    visual.set_clip(clip(style.bounds)?);
                }
            }
            _ => {}
        }
    }
    for (slot, mut visual) in &mut lines {
        visual.set_clip(ScreenClip::Empty);
        if !layout.usable() {
            continue;
        }
        let Some((from, to, color, width)) = line_style(
            *slot, &layout, &state, &objects, &links, spacing, &selection,
        ) else {
            continue;
        };
        let Some((from, to)) = crop_segment(from, to, layout.canvas) else {
            continue;
        };
        *visual = ScreenLineVisual::new(from, to, width, color)?;
        visual.set_draw_order_depth(match slot {
            VisualSlot::ObjectEdge { index, .. } => object_depth(*index) + 0.001,
            VisualSlot::LinkEdge { .. } | VisualSlot::PendingLink => 0.5,
            VisualSlot::SelectionEdge(_) => 2.5,
            _ => 0.0,
        })?;
        visual.set_clip(canvas_clip);
    }
    for (slot, mut visual) in &mut circles {
        visual.set_clip(ScreenClip::Empty);
        if !layout.usable() {
            continue;
        }
        let (center, radius, color, bounds) = match slot {
            VisualSlot::LinkAnchor { index, end } => {
                let Some(link) = links[*index].filter(|link| link.spring) else {
                    continue;
                };
                (
                    if *end == 0 { link.from } else { link.to },
                    3.5,
                    Color::rgb8(255, 212, 140),
                    canvas_clip,
                )
            }
            VisualSlot::PendingAnchor(end) => {
                let Some(point) = pending_attachment_position(&state, *end) else {
                    continue;
                };
                (
                    state.camera.project(point, layout.canvas),
                    5.0,
                    ACCENT,
                    canvas_clip,
                )
            }
            VisualSlot::ObjectDisc(index) => {
                let Some(object) = objects[*index] else {
                    continue;
                };
                if object.kind != ObjectKind::Ball {
                    continue;
                }
                (object.center, object.radius, object.color, canvas_clip)
            }
            VisualSlot::PaletteIcon(kind) => {
                let Some(rect) = layout.control(Control::Palette(*kind)) else {
                    continue;
                };
                (
                    LogicalScreenPosition::new(rect.x + rect.width * 0.5, rect.y + 26.0),
                    13.0,
                    ACCENT,
                    viewport_clip,
                )
            }
            _ => continue,
        };
        visual.set_geometry(center, radius)?;
        visual.set_color(color)?;
        visual.set_draw_order_depth(match slot {
            VisualSlot::ObjectDisc(index) => object_depth(*index),
            VisualSlot::LinkAnchor { .. } | VisualSlot::PendingAnchor(_) => 2.4,
            _ => 5.0,
        })?;
        visual.set_clip(bounds);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corner_spring_scene() -> (EditorState, u64) {
        let mut state = EditorState::default();
        state
            .document
            .set_environment(super::super::document::PhysicsEnvironment {
                gravity_m_s2: Point::default(),
                linear_drag_per_s: 0.0,
            })
            .unwrap();
        let fixed = state
            .document
            .add(ObjectKind::Anchor, Point::new(-3.0, 2.0))
            .unwrap();
        let body = state
            .document
            .add(ObjectKind::Box, Point::new(1.0, 0.0))
            .unwrap();
        state.document.set_size(body, 2.0).unwrap();
        state
            .document
            .add_spring_at(
                Attachment {
                    body: fixed,
                    local_m: Point::default(),
                },
                Attachment {
                    body,
                    local_m: Point::new(-1.0, 0.5),
                },
            )
            .unwrap();
        // Stretch the real spring without redefining its rest length.
        state
            .document
            .move_object(fixed, Point::new(-4.0, 2.0))
            .unwrap();
        (state, body)
    }

    #[test]
    fn rendered_spring_ends_at_the_authored_corner_not_the_body_center() {
        let (state, _) = corner_spring_scene();
        let canvas = Rect::new(16.0, 132.0, 800.0, 480.0);
        let link = display_links(&state, canvas)[0].unwrap();
        assert_eq!(
            link.from,
            state.camera.project(Point::new(-4.0, 2.0), canvas)
        );
        assert_eq!(link.to, state.camera.project(Point::new(0.0, 0.5), canvas));
        assert_ne!(link.to, display_objects(&state, canvas)[1].unwrap().center);
        assert_eq!(link_edge(link, LINK_SEGMENTS - 1).unwrap().1, link.to);
    }

    #[test]
    fn runtime_box_and_spring_render_the_same_canonical_rotating_corner() {
        let (mut state, body) = corner_spring_scene();
        let authored = state.document.clone();
        let mut run = super::super::session::RunSession::new(&state.document).unwrap();
        run.advance(0.0);
        for _ in 0..60 {
            run.advance(run.world().solver().fixed_dt_s);
            assert!(run.failure.is_none(), "{:?}", run.failure);
        }
        let angle_deg = run.angle_deg(body).unwrap();
        assert!(
            angle_deg > 0.01,
            "corner spring must produce rotation: {angle_deg}"
        );
        state.mode = Mode::Preview;
        state.run = Some(run);
        let canvas = Rect::new(16.0, 132.0, 800.0, 480.0);
        let box_view = display_objects(&state, canvas)[1].unwrap();
        assert!((box_view.angle - angle_deg.to_radians()).abs() < 1e-12);
        let (top_left, _) = object_edge(box_view, 0).unwrap();
        // View corners use screen Y down; local (-1,+1) is edge 0's first point.
        let endpoint = display_links(&state, canvas)[0].unwrap().to;
        let difference = endpoint.to_vec2() - top_left.to_vec2();
        assert!(difference.x().hypot(difference.y()) < 0.0001);
        assert_eq!(state.document.objects(), authored.objects());
        assert_eq!(state.document.links(), authored.links());
    }

    #[test]
    fn rotating_ball_has_an_orientation_indicator_but_no_fake_geometry() {
        let object = DisplayObject {
            kind: ObjectKind::Ball,
            center: LogicalScreenPosition::new(100.0, 100.0),
            radius: 20.0,
            half_height: 20.0,
            angle: std::f64::consts::FRAC_PI_2,
            color: ACCENT,
        };
        let (from, to) = object_edge(object, 0).unwrap();
        assert_eq!(from, object.center);
        assert!((to.to_vec2().x() - 100.0).abs() < 0.0001);
        assert!((to.to_vec2().y() - 87.0).abs() < 0.0001);
        assert!((1..6).all(|edge| object_edge(object, edge).is_none()));
    }

    #[test]
    fn many_revolutions_preserve_corner_and_marker_alignment() {
        let angle: f64 = 999_999.97;
        let object = DisplayObject {
            kind: ObjectKind::Box,
            center: LogicalScreenPosition::new(500.0, 400.0),
            radius: 120.0,
            half_height: 60.0,
            angle,
            color: ACCENT,
        };
        let (corner, _) = object_edge(object, 0).unwrap();
        let (sin, cos) = angle.sin_cos();
        let expected = LogicalScreenPosition::new(
            (500.0 - 120.0 * cos - 60.0 * sin) as f32,
            (400.0 + 120.0 * sin - 60.0 * cos) as f32,
        );
        assert_eq!(
            corner, expected,
            "do not narrow the angle before evaluating rotation"
        );
    }

    #[test]
    fn rectangle_rendering_and_picking_share_physical_extents() {
        let mut state = EditorState::default();
        let id = state
            .document
            .add(ObjectKind::Box, Point::default())
            .unwrap();
        state.document.set_size(id, 8.0).unwrap();
        state.document.set_height(id, 0.5).unwrap();
        state.document.rotate(id, 30.0).unwrap();
        let canvas = Rect::new(16.0, 132.0, 800.0, 480.0);
        let object = display_objects(&state, canvas)[0].unwrap();
        let length = |edge| {
            let (a, b) = object_edge(object, edge).unwrap();
            let delta = b.to_vec2() - a.to_vec2();
            delta.x().hypot(delta.y())
        };
        assert!((length(0) - 8.0 * 48.0).abs() < 0.001);
        assert!((length(1) - 0.5 * 48.0).abs() < 0.001);
        let angle = 30.0_f64.to_radians();
        for (local_x, local_y, inside) in [(3.9, 0.2, true), (0.0, 0.3, false)] {
            let point = Point::new(
                local_x * angle.cos() - local_y * angle.sin(),
                local_x * angle.sin() + local_y * angle.cos(),
            );
            assert_eq!(state.pick(point), inside.then_some(id));
        }
        state.document.set_fixed(id, true).unwrap();
        assert_ne!(
            display_objects(&state, canvas)[0].unwrap().color,
            object.color
        );
    }

    #[test]
    fn compact_inspector_exposes_physical_values_without_display_only_labels() -> LogicResult {
        let mut state = EditorState::default();
        let id = state
            .document
            .add(ObjectKind::Box, Point::default())
            .unwrap();
        state.select_one(Some(id));
        state.document.set_height(id, 0.5).unwrap();
        let layout = Layout::for_state(LogicalViewport::new(900.0, 600.0)?, &state);
        assert_eq!(
            caption_style(Caption::SizeTitle, &layout, &state)
                .unwrap()
                .text,
            "WIDTH"
        );
        assert_eq!(
            caption_style(Caption::HeightValue, &layout, &state)
                .unwrap()
                .text,
            "0.50 m"
        );
        assert_eq!(
            caption_style(Caption::RestitutionValue, &layout, &state)
                .unwrap()
                .text,
            "0.35"
        );
        for caption in [
            Caption::MassTitle,
            Caption::MassValue,
            Caption::SizeTitle,
            Caption::SizeValue,
            Caption::HeightTitle,
            Caption::HeightValue,
            Caption::RestitutionTitle,
            Caption::RestitutionValue,
            Caption::Rotation,
            Caption::InspectorHint,
        ] {
            let style = caption_style(caption, &layout, &state).unwrap();
            assert!(!style.text.contains("DISPLAY"));
            assert!(style.position.to_vec2().y() < layout.inspector.y + layout.inspector.height);
        }
        Ok(())
    }

    #[test]
    fn inspector_property_text_stays_left_of_controls_at_minimum_viewport() -> LogicResult {
        let mut app = Application::<super::super::input::EditorAction>::new(AppConfig::default())?;
        let assets = EditorAssets::register(&mut app)?;
        let mut state = EditorState::default();
        let id = state
            .document
            .add(ObjectKind::Box, Point::default())
            .unwrap();
        state.document.set_mass(id, 1_000_000.0).unwrap();
        state.document.set_size(id, 1_000.0).unwrap();
        state.document.set_height(id, 1_000.0).unwrap();
        state.document.set_restitution(id, 1.0).unwrap();
        state.document.rotate(id, 345.0).unwrap();
        state.select_one(Some(id));
        let layout = Layout::for_state(LogicalViewport::new(900.0, 600.0)?, &state);
        for (caption, control, body_font) in [
            (Caption::MassTitle, Control::MassDown, false),
            (Caption::MassValue, Control::MassDown, true),
            (Caption::SizeTitle, Control::SizeDown, false),
            (Caption::SizeValue, Control::SizeDown, true),
            (Caption::HeightTitle, Control::HeightDown, false),
            (Caption::HeightValue, Control::HeightDown, true),
            (Caption::RestitutionTitle, Control::RestitutionDown, false),
            (Caption::RestitutionValue, Control::RestitutionDown, true),
            (Caption::Rotation, Control::RotateLeft, false),
        ] {
            let style = caption_style(caption, &layout, &state).unwrap();
            let font = if body_font {
                &assets.body
            } else {
                &assets.small
            };
            let text = ScreenTextVisual::new(font.clone(), &style.text, style.position)?;
            assert!(
                style.position.to_vec2().x() + text.metrics().advance() + 4.0
                    < layout.control(control).unwrap().x,
                "{} overlaps its control",
                style.text
            );
        }
        Ok(())
    }

    #[test]
    fn segment_crop_omits_offscreen_and_degenerate_geometry() {
        let rect = Rect::new(100.0, 100.0, 200.0, 200.0);
        let point = LogicalScreenPosition::new;
        assert!(crop_segment(point(0.0, 0.0), point(1.0, 1.0), rect).is_none());
        assert!(crop_segment(point(150.0, 150.0), point(150.1, 150.1), rect).is_none());
        let (from, to) = crop_segment(point(-10000.0, 200.0), point(10000.0, 200.0), rect).unwrap();
        assert!((from.to_vec2().x() - 100.0).abs() < 0.01);
        assert!((to.to_vec2().x() - 300.0).abs() < 0.01);
    }

    #[test]
    fn grid_work_is_bounded_even_for_very_large_windows() {
        let state = EditorState::default();
        for width in [600.0, 1920.0, 100000.0] {
            let canvas = Rect::new(0.0, 0.0, width, 1000.0);
            let pixels = grid_step(&state, canvas) * state.camera.pixels_per_m;
            assert!(pixels >= f64::from(width) / 60.0);
        }
    }

    #[test]
    fn rod_and_spring_geometry_preserve_endpoints_and_have_bounded_work() {
        let from = LogicalScreenPosition::new(100.0, 120.0);
        let to = LogicalScreenPosition::new(300.0, 240.0);
        let rod = DisplayLink {
            from,
            to,
            spring: false,
        };
        assert_eq!(link_edge(rod, 0), Some((from, to)));
        for edge in 1..LINK_SEGMENTS {
            assert!(link_edge(rod, edge).is_none());
        }
        let spring = DisplayLink {
            spring: true,
            ..rod
        };
        assert_eq!(link_edge(spring, 0).unwrap().0, from);
        assert_eq!(link_edge(spring, LINK_SEGMENTS - 1).unwrap().1, to);
        for edge in 0..LINK_SEGMENTS - 1 {
            assert_eq!(
                link_edge(spring, edge).unwrap().1,
                link_edge(spring, edge + 1).unwrap().0
            );
        }
        assert!(link_edge(spring, LINK_SEGMENTS).is_none());
        let tiny = DisplayLink { to: from, ..spring };
        assert!(link_edge(tiny, 0).is_none());
    }

    #[test]
    fn graph_visuals_follow_current_authored_geometry_without_owning_state() {
        let mut state = EditorState::default();
        let bob = state.document.add_pendulum(Point::default()).unwrap();
        let canvas = Rect::new(16.0, 132.0, 800.0, 480.0);
        let before = display_links(&state, canvas)[0].unwrap();
        state
            .document
            .move_object(bob, Point::new(3.0, -4.0))
            .unwrap();
        let after = display_links(&state, canvas)[0].unwrap();
        assert_eq!(before.from, after.from);
        assert_ne!(before.to, after.to);
        assert_eq!(
            after.to,
            state.camera.project(Point::new(3.0, -4.0), canvas)
        );
        assert_eq!(display_links(&state, canvas).iter().flatten().count(), 1);
    }

    #[test]
    fn environment_text_is_above_scrim_and_field_values_avoid_buttons() -> LogicResult {
        let mut state = EditorState {
            environment_open: true,
            ..EditorState::default()
        };
        let viewport = LogicalViewport::new(900.0, 600.0)?;
        let layout = Layout::for_state(viewport, &state);
        let title = caption_style(Caption::EnvironmentTitle, &layout, &state).unwrap();
        assert_eq!(title.depth, 10.0);
        for caption in [Caption::GravityX, Caption::GravityY, Caption::LinearDrag] {
            let field = caption_style(caption, &layout, &state).unwrap();
            assert!(field.bounds.x + field.bounds.width < layout.modal.x + 324.0);
            assert_eq!(field.depth, 10.0);
        }
        state.environment_open = false;
        assert!(caption_style(Caption::EnvironmentTitle, &layout, &state).is_none());
        Ok(())
    }

    #[test]
    fn view_hides_authoring_inspector_and_marks_translation_only_limits() -> LogicResult {
        let state = EditorState {
            mode: Mode::Preview,
            ..EditorState::default()
        };
        let layout = Layout::for_state(LogicalViewport::new(900.0, 600.0)?, &state);
        assert!(caption_style(Caption::InspectorTitle, &layout, &state).is_none());
        assert!(caption_style(Caption::PaletteTitle, &layout, &state).is_none());
        let hint = caption_style(Caption::ToolbarHint, &layout, &state).unwrap();
        assert!(hint.text.contains("frictionless"));
        assert!(!hint.text.contains("Static preview"));
        Ok(())
    }
}
