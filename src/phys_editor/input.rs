use sim_logic::prelude::*;

use super::{
    catalog_input,
    catalog_layout::CatalogLayout,
    commands::{activate, report},
    document::Point,
    layout::Layout,
    selection::SelectionGesture,
    state::{Control, Drag, EditorState, Mode, Tool},
};

pub use crate::actions::AppAction as EditorAction;

pub(crate) fn bind(app: &mut Application<EditorAction>) -> LogicResult {
    crate::actions::bind(app)
}

fn sample_in_canvas(state: &EditorState, sample: PointerSample) -> Option<Point> {
    let layout = Layout::for_state(sample.viewport(), state);
    if state.environment_open || !layout.usable() || !layout.canvas.contains(sample.position()) {
        return None;
    }
    let catalog = CatalogLayout::new(&layout, &state.catalog);
    if (state.catalog.open
        && !matches!(
            state.drag,
            Some(Drag::Place {
                from_palette: true,
                ..
            })
        ))
        || (state.catalog.blend > 0.001 && catalog.panel.contains(sample.position()))
    {
        return None;
    }
    Some(state.camera.unproject(sample.position(), layout.canvas))
}

fn move_drag(state: &mut EditorState, point: Point) {
    if let Some(gesture) = &mut state.selection_gesture {
        gesture.update(point, 3.0 / state.camera.pixels_per_m);
    }
    state.drag = match state.drag {
        Some(Drag::MoveSelection { origin, .. }) => Some(Drag::MoveSelection {
            origin,
            delta: state.snapped(Point::new(point.x - origin.x, point.y - origin.y)),
        }),
        Some(Drag::Move { id, offset, .. }) => Some(Drag::Move {
            id,
            offset,
            position: state.snapped(Point::new(point.x + offset.x, point.y + offset.y)),
        }),
        Some(Drag::Place {
            placement,
            from_palette,
            ..
        }) => Some(Drag::Place {
            placement,
            from_palette,
            position: state.snapped(point),
        }),
        other => other,
    };
}

fn begin_canvas(state: &mut EditorState, point: Point) {
    if matches!(state.tool, Tool::Rod | Tool::Spring) {
        let attachment = state.pick_attachment(point);
        state.select_one(attachment.map(|point| point.body));
        state.drag = attachment.map(|attachment| Drag::Link { attachment });
        return;
    }
    if state.tool == Tool::Select {
        if state.controls_held.iter().any(|held| *held) {
            state.selection_gesture = Some(SelectionGesture::lasso(point));
            return;
        }
        if state.shifts_held.iter().any(|held| *held) {
            state.selection_gesture = Some(SelectionGesture::rectangle(point));
            return;
        }
    }
    let picked = state.pick(point);
    if state.tool == Tool::Select
        && state.selection.len() > 1
        && picked.is_some_and(|id| state.is_selected(id))
    {
        state.drag = Some(Drag::MoveSelection {
            origin: point,
            delta: Point::default(),
        });
        return;
    }
    state.select_one(picked);
    if state.tool == Tool::Erase {
        state.drag = picked.map(|id| Drag::Erase { id });
    } else if let Some(id) = picked {
        if let Some(object) = state.document.object(id) {
            state.drag = Some(Drag::Move {
                id,
                offset: Point::new(object.position.x - point.x, object.position.y - point.y),
                position: object.position,
            });
        }
    } else if state.tool == Tool::Build {
        state.drag = Some(Drag::Place {
            placement: state.brush,
            position: state.snapped(point),
            from_palette: false,
        });
    }
}

fn release_drag(state: &mut EditorState, sample: Option<PointerSample>, cancelled: bool) {
    if let Some(mut gesture) = state.selection_gesture.take() {
        if !cancelled && let Some(point) = sample.and_then(|sample| sample_in_canvas(state, sample))
        {
            gesture.update(point, 0.0);
            state.selection = state
                .document
                .objects()
                .iter()
                .filter(|object| gesture.contains(object.position))
                .map(|object| object.id)
                .collect();
            state.selected = state.selection.iter().next().copied();
            state.status = "Selection changed. Drag a selected object to move the group; Delete removes the group.";
        }
        return;
    }
    let Some(drag) = state.drag.take() else {
        return;
    };
    if cancelled {
        state.link_start = None;
        state.status = "Gesture cancelled. Scene unchanged.";
        return;
    }
    let Some(point) = sample.and_then(|sample| sample_in_canvas(state, sample)) else {
        state.status = "Release inside the canvas to place or move an object.";
        return;
    };
    // Final coordinates belong to this release, not the frame's final pointer.
    state.drag = Some(drag);
    move_drag(state, point);
    let committed = state.drag.take();
    let result = match committed {
        Some(Drag::Place {
            placement,
            position,
            ..
        }) => placement.commit(&mut state.document, position).map(|id| {
            state.select_one(Some(id));
        }),
        Some(Drag::MoveSelection { delta, .. }) => {
            let ids: Vec<_> = state.selection.iter().copied().collect();
            state.document.translate_many(&ids, delta)
        }
        Some(Drag::Move { id, position, .. }) => state.document.move_object(id, position),
        Some(Drag::Erase { id }) if state.pick(point) == Some(id) => state.document.remove(id),
        Some(Drag::Link { attachment }) => {
            let Some(endpoint) = state
                .pick_attachment(point)
                .filter(|endpoint| endpoint.body == attachment.body)
            else {
                return;
            };
            if let Some(first) = state.link_start.take() {
                let result = if state.tool == Tool::Rod {
                    state.document.add_rod(first.body, endpoint.body)
                } else {
                    state.document.add_spring_at(first, endpoint)
                };
                result.map(|_| ())
            } else {
                state.link_start = Some(endpoint);
                state.status = if state.tool == Tool::Spring {
                    "Surface point selected. Click another body surface; corners snap. Escape cancels."
                } else {
                    "Body centre selected. Rods join centres; click another body. Escape cancels."
                };
                return;
            }
        }
        _ => return,
    };
    report(state, result);
}

fn pan_to(state: &mut EditorState, sample: PointerSample) {
    if let Some(previous) = state.pan {
        if previous.viewport() == sample.viewport() {
            let delta = sample.position().to_vec2() - previous.position().to_vec2();
            state.camera.center.x = (state.camera.center.x
                - f64::from(delta.x()) / state.camera.pixels_per_m)
                .clamp(-1e6, 1e6);
            state.camera.center.y = (state.camera.center.y
                + f64::from(delta.y()) / state.camera.pixels_per_m)
                .clamp(-1e6, 1e6);
        }
        state.pan = Some(sample);
    }
}

pub(crate) fn route(
    input: FrameInput<EditorAction>,
    viewport: FrameViewport,
    time: FrameTime,
    state: Option<ResMut<EditorState>>,
    mut window: AppResMut<WindowControls>,
    mut commands: Commands,
) -> LogicResult {
    let Some(mut state) = state else {
        return Ok(());
    };
    // A zero fixed scale alone would retain undelivered fixed input forever.
    // Pause also clears that queue, while FrameUpdate/FrameTime remain live.
    commands.set_paused(true)?;
    let current = viewport.logical();
    let resized = state.last_viewport.is_some_and(|old| old != current);
    state.last_viewport = Some(current);
    if resized || input.focus_lost() {
        state.cancel_gesture();
    }
    if input.focus_lost() {
        state.controls_held.fill(false);
        state.shifts_held.fill(false);
        state.hovered = None;
        state.pointer_world = None;
        if let Some(run) = &mut state.run {
            run.pause(true);
        }
        window.request_cursor(CursorShape::Default);
        return Ok(());
    }
    let mut switched = false;
    for event in input.events() {
        match event {
            FrameInputEvent::Scroll(scroll) => {
                if switched
                    || state.drag.is_some()
                    || state.pan.is_some()
                    || state.selection_gesture.is_some()
                {
                    continue;
                }
                if state.catalog.open {
                    if scroll.delta().y() < 0.0 {
                        state.catalog.page =
                            (state.catalog.page + 1).min(state.catalog.page_count() - 1);
                    } else if scroll.delta().y() > 0.0 {
                        state.catalog.page = state.catalog.page.saturating_sub(1);
                    }
                    state.catalog_pointer.cancel();
                    state.catalog.pressed = None;
                    continue;
                }
                if let Some(sample) = scroll.pointer().filter(|sample| {
                    sample.viewport() == current && sample_in_canvas(&state, *sample).is_some()
                }) {
                    let delta = scroll.delta();
                    let steps = match delta.unit() {
                        ScrollUnit::Lines => delta.y(),
                        ScrollUnit::Pixels => delta.y() / 80.0,
                    };
                    let canvas = Layout::for_state(current, &state).canvas;
                    state.camera.zoom(
                        (steps.clamp(-8.0, 8.0) * 0.12).exp(),
                        sample.position(),
                        canvas,
                    );
                }
            }
            FrameInputEvent::Action(edge) => {
                let down = edge.state() == ButtonState::Pressed && !edge.is_cancelled();
                if edge.action() == EditorAction::Control {
                    let index = usize::from(
                        edge.control() == InputControl::Key(PhysicalKeyCode::ControlRight),
                    );
                    state.controls_held[index] = down;
                    continue;
                }
                if edge.action() == EditorAction::Shift {
                    let index = usize::from(
                        edge.control() == InputControl::Key(PhysicalKeyCode::ShiftRight),
                    );
                    state.shifts_held[index] = down;
                    continue;
                }
                if edge.action() == EditorAction::Fullscreen && down {
                    state.fullscreen_requested = !state.fullscreen_requested;
                    window.request_mode(if state.fullscreen_requested {
                        WindowMode::BorderlessFullscreen(FullscreenMonitor::Automatic)
                    } else {
                        WindowMode::Windowed
                    });
                }
                let layout = Layout::for_state(current, &state);
                let mut catalog_changed = false;
                if !switched {
                    let before = (
                        state.mode,
                        state.environment_open,
                        state.catalog.open,
                        state.catalog.category,
                        state.catalog.section,
                        state.catalog.page,
                    );
                    let catalog_owned = catalog_input::route_edge(&mut state, edge, &layout);
                    catalog_changed = before
                        != (
                            state.mode,
                            state.environment_open,
                            state.catalog.open,
                            state.catalog.category,
                            state.catalog.section,
                            state.catalog.page,
                        );
                    if catalog_owned {
                        switched = catalog_changed;
                        continue;
                    }
                }
                let sample = edge.pointer().filter(|sample| sample.viewport() == current);
                let hit = if !switched && layout.usable() {
                    sample
                        .and_then(|sample| layout.hit(sample.position()))
                        .filter(|control| state.enabled(*control))
                } else {
                    None
                };
                let outcome = state.pointer.process(edge, hit);
                if switched || !layout.usable() {
                    continue;
                }
                let previous_mode = (state.mode, state.environment_open);
                if let Some(PointerButtonEvent::Clicked { target, .. }) = outcome.event() {
                    activate(&mut state, target);
                } else if edge.action() == EditorAction::Point
                    && state.pan.is_none()
                    && state.mode == Mode::Editor
                {
                    if down {
                        if hit.is_none()
                            && let Some(point) =
                                sample.and_then(|sample| sample_in_canvas(&state, sample))
                        {
                            begin_canvas(&mut state, point);
                        }
                    } else if edge.state() == ButtonState::Released {
                        release_drag(&mut state, sample, edge.is_cancelled());
                    }
                }
                if edge.action() == EditorAction::Pan {
                    if down
                        && sample.is_some_and(|sample| sample_in_canvas(&state, sample).is_some())
                    {
                        state.cancel_gesture();
                        state.pan = sample;
                    } else if edge.state() == ButtonState::Released {
                        if !edge.is_cancelled()
                            && let Some(sample) = sample
                        {
                            pan_to(&mut state, sample);
                        }
                        state.pan = None;
                    }
                }
                if down && matches!(edge.control(), InputControl::Key(_)) {
                    if state.environment_open {
                        if edge.action() == EditorAction::Escape {
                            activate(&mut state, Control::CloseEnvironment);
                        }
                        switched = (state.mode, state.environment_open) != previous_mode;
                        continue;
                    }
                    let ctrl = state.controls_held.iter().any(|held| *held);
                    let shift = state.shifts_held.iter().any(|held| *held);
                    let control = match edge.action() {
                        EditorAction::Select => Some(Control::Tool(Tool::Select)),
                        EditorAction::Build => Some(Control::Tool(Tool::Build)),
                        EditorAction::Erase => Some(Control::Tool(Tool::Erase)),
                        EditorAction::Rod => Some(Control::Tool(Tool::Rod)),
                        EditorAction::Spring => Some(Control::Tool(Tool::Spring)),
                        EditorAction::Environment => Some(Control::Environment),
                        EditorAction::Pause => Some(Control::Pause),
                        EditorAction::Undo if ctrl => {
                            Some(if shift { Control::Redo } else { Control::Undo })
                        }
                        EditorAction::Redo if ctrl => Some(Control::Redo),
                        EditorAction::Duplicate if ctrl => Some(Control::Duplicate),
                        EditorAction::Delete => Some(Control::Delete),
                        EditorAction::Snap => Some(Control::Snap),
                        EditorAction::Home => Some(Control::Home),
                        EditorAction::Run => Some(Control::Run),
                        EditorAction::Escape => {
                            if state.drag.is_some()
                                || state.pan.is_some()
                                || state.link_start.is_some()
                                || state.selection_gesture.is_some()
                            {
                                state.cancel_gesture();
                                state.status = "Gesture cancelled. Scene unchanged.";
                            } else if state.mode == Mode::Preview {
                                activate(&mut state, Control::Back);
                            } else {
                                state.select_one(None);
                                state.tool = Tool::Select;
                            }
                            None
                        }
                        _ => None,
                    };
                    if let Some(control) = control {
                        activate(&mut state, control);
                    }
                }
                switched = catalog_changed || (state.mode, state.environment_open) != previous_mode;
            }
        }
    }
    let layout = Layout::for_state(current, &state);
    let sample = input
        .pointer()
        .filter(|sample| sample.viewport() == current);
    catalog_input::animate(&mut state, &layout, sample, time.seconds_f32());
    if !layout.usable() || sample.is_none() {
        state.cancel_gesture();
    }
    if let Some(sample) = sample {
        pan_to(&mut state, sample);
        state.pointer_world = sample_in_canvas(&state, sample);
        if let Some(point) = state.pointer_world {
            move_drag(&mut state, point);
        }
    } else {
        state.pointer_world = None;
    }
    state.hovered = if layout.usable() && !state.catalog.open {
        sample
            .and_then(|sample| layout.hit(sample.position()))
            .filter(|control| state.enabled(*control))
    } else {
        None
    };
    let blend = 1.0 - (-16.0 * time.seconds_f32().clamp(0.0, 0.25)).exp();
    for control in Control::ALL {
        let desired = if state.hovered == Some(control) {
            1.0
        } else {
            0.0
        };
        state.emphasis[control.index()] += (desired - state.emphasis[control.index()]) * blend;
    }
    window.request_cursor(if state.pan.is_some() {
        CursorShape::Grabbing
    } else if state.catalog.search_focused
        && state.catalog.open
        && state.catalog.hovered == Some(super::catalog::CatalogTarget::Search)
    {
        CursorShape::Text
    } else if state.hovered.is_some() || state.catalog.hovered.is_some() {
        CursorShape::Pointer
    } else if state.pointer_world.is_some()
        && state.mode == Mode::Editor
        && state.tool != Tool::Select
    {
        CursorShape::Crosshair
    } else {
        CursorShape::Default
    });
    Ok(())
}
