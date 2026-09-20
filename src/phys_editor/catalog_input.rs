//! Catalog owns search, selection and capture before canvas tools see input.

use sim_logic::prelude::*;

use super::{
    catalog::{CatalogTarget, Category, EntryAction, EntryId},
    catalog_layout::CatalogLayout,
    commands::activate,
    input::EditorAction,
    layout::Layout,
    placement::Placement,
    state::{Control, Drag, EditorState, Mode, Tool},
};

fn placement(entry: EntryId) -> Option<Placement> {
    match entry.entry().action {
        EntryAction::Primitive(kind) => Some(Placement::Primitive(kind)),
        EntryAction::Pendulum => Some(Placement::Pendulum),
        EntryAction::Oscillator => Some(Placement::Oscillator),
        EntryAction::BounceLab => Some(Placement::BounceLab),
        _ => None,
    }
}

fn choose(state: &mut EditorState, entry: EntryId) {
    state.cancel_gesture();
    state.catalog.remember(entry);
    state.catalog.close();
    if let Some(placement) = placement(entry) {
        state.brush = placement;
        state.tool = Tool::Build;
        state.status = match placement {
            Placement::Primitive(_) => {
                "Object selected. Click in the canvas to place; Escape cancels."
            }
            _ => {
                "Assembly selected. Click to place its anchor; all parts remain ordinary primitives."
            }
        };
    } else {
        activate(
            state,
            match entry.entry().action {
                EntryAction::Rod => Control::Tool(Tool::Rod),
                EntryAction::Spring => Control::Tool(Tool::Spring),
                _ => return,
            },
        );
    }
}

fn activate_target(state: &mut EditorState, target: CatalogTarget) {
    state.cancel_gesture();
    match target {
        CatalogTarget::Category(category) => {
            state.catalog.select_category(category);
            state.catalog.focused = None;
            state.catalog.search_focused = false;
        }
        CatalogTarget::Entry(entry) | CatalogTarget::Quick(entry) => choose(state, entry),
        CatalogTarget::Favorite(entry) => {
            state.catalog.toggle_favorite(entry);
        }
        CatalogTarget::Open => {
            if state.catalog.open {
                state.catalog.close();
            } else {
                state.catalog.open();
            }
        }
        CatalogTarget::IdealFilter => {
            state.catalog.ideal_only = !state.catalog.ideal_only;
            state.catalog.page = 0;
        }
        CatalogTarget::Section(section) => {
            state.catalog.set_section(section);
        }
        CatalogTarget::Search => {
            state.catalog.search_focused = true;
            state.catalog.focused = Some(CatalogTarget::Search);
        }
        CatalogTarget::ClearSearch => {
            state.catalog.clear_query();
            state.catalog.search_focused = true;
            state.catalog.focused = Some(CatalogTarget::Search);
        }
        CatalogTarget::Close => state.catalog.close(),
        CatalogTarget::PreviousPage => state.catalog.page = state.catalog.page.saturating_sub(1),
        CatalogTarget::NextPage => {
            state.catalog.page = (state.catalog.page + 1).min(state.catalog.page_count() - 1)
        }
    }
}

fn focus_next(state: &mut EditorState, backwards: bool, layout: &Layout) {
    let mut targets = vec![CatalogTarget::Search];
    targets.extend(
        state
            .catalog
            .visible()
            .into_iter()
            .map(CatalogTarget::Entry),
    );
    targets.extend(Category::ALL.into_iter().map(CatalogTarget::Category));
    targets.extend(
        state
            .catalog
            .visible()
            .into_iter()
            .map(CatalogTarget::Favorite),
    );
    targets.extend((0..state.catalog.category.sections().len()).map(CatalogTarget::Section));
    targets.push(CatalogTarget::Close);
    targets.push(CatalogTarget::IdealFilter);
    let geometry = CatalogLayout::new(layout, &state.catalog);
    targets.retain(|target| geometry.target(*target, &state.catalog).is_some());
    let current = targets
        .iter()
        .position(|target| Some(*target) == state.catalog.focused);
    let index = match current {
        None => 0,
        Some(index) if backwards => (index + targets.len() - 1) % targets.len(),
        Some(index) => (index + 1) % targets.len(),
    };
    state.catalog.focused = Some(targets[index]);
    state.catalog.search_focused = targets[index] == CatalogTarget::Search;
}

fn ascii_key(key: PhysicalKeyCode) -> Option<char> {
    use PhysicalKeyCode::*;
    Some(match key {
        KeyA => 'a',
        KeyB => 'b',
        KeyC => 'c',
        KeyD => 'd',
        KeyE => 'e',
        KeyF => 'f',
        KeyG => 'g',
        KeyH => 'h',
        KeyI => 'i',
        KeyJ => 'j',
        KeyK => 'k',
        KeyL => 'l',
        KeyM => 'm',
        KeyN => 'n',
        KeyO => 'o',
        KeyP => 'p',
        KeyQ => 'q',
        KeyR => 'r',
        KeyS => 's',
        KeyT => 't',
        KeyU => 'u',
        KeyV => 'v',
        KeyW => 'w',
        KeyX => 'x',
        KeyY => 'y',
        KeyZ => 'z',
        Digit0 => '0',
        Digit1 => '1',
        Digit2 => '2',
        Digit3 => '3',
        Digit4 => '4',
        Digit5 => '5',
        Digit6 => '6',
        Digit7 => '7',
        Digit8 => '8',
        Digit9 => '9',
        Space => ' ',
        Minus => '-',
        _ => return None,
    })
}

fn keyboard(state: &mut EditorState, key: PhysicalKeyCode, down: bool, layout: &Layout) -> bool {
    let ctrl = state.controls_held.iter().any(|held| *held);
    if down
        && ((ctrl && matches!(key, PhysicalKeyCode::KeyF | PhysicalKeyCode::KeyK))
            || (key == PhysicalKeyCode::Slash && !state.catalog.search_focused))
    {
        state.cancel_gesture();
        state.catalog.open();
        return true;
    }
    if !state.catalog.open {
        return false;
    }
    if !down {
        return true;
    }
    match key {
        PhysicalKeyCode::Escape => {
            state.cancel_gesture();
            state.catalog.close();
        }
        PhysicalKeyCode::Tab => {
            focus_next(state, state.shifts_held.iter().any(|held| *held), layout)
        }
        PhysicalKeyCode::Enter => {
            let target = if state.catalog.search_focused {
                state
                    .catalog
                    .visible()
                    .first()
                    .copied()
                    .map(CatalogTarget::Entry)
            } else {
                state.catalog.focused
            };
            if let Some(target) = target {
                // Filtering can remove the focused entry without another key
                // movement. Never activate a hidden or stale catalog target.
                let geometry = CatalogLayout::new(layout, &state.catalog);
                if geometry.target(target, &state.catalog).is_some() {
                    activate_target(state, target);
                } else {
                    state.catalog.focused = Some(CatalogTarget::Search);
                    state.catalog.search_focused = true;
                }
            }
        }
        PhysicalKeyCode::Backspace if state.catalog.search_focused => {
            if ctrl {
                state.catalog.clear_query();
            } else {
                state.catalog.backspace();
            }
            state.catalog_pointer.cancel();
            state.catalog.pressed = None;
            state.drag = None;
        }
        PhysicalKeyCode::ArrowDown
        | PhysicalKeyCode::ArrowUp
        | PhysicalKeyCode::ArrowLeft
        | PhysicalKeyCode::ArrowRight => {
            let visible = state.catalog.visible();
            let current = visible
                .iter()
                .position(|id| state.catalog.focused == Some(CatalogTarget::Entry(*id)));
            let delta = match key {
                PhysicalKeyCode::ArrowDown => 3,
                PhysicalKeyCode::ArrowUp => -3,
                PhysicalKeyCode::ArrowLeft => -1,
                _ => 1,
            };
            if !visible.is_empty() {
                let index = current.map_or(0, |index| {
                    (index as isize + delta).clamp(0, visible.len() as isize - 1) as usize
                });
                state.catalog.focused = Some(CatalogTarget::Entry(visible[index]));
                state.catalog.search_focused = false;
            }
        }
        _ if state.catalog.search_focused && !ctrl => {
            if let Some(character) = ascii_key(key) {
                state.catalog.append_ascii(character);
                state.catalog_pointer.cancel();
                state.catalog.pressed = None;
                state.drag = None;
            }
        }
        _ => {}
    }
    true
}

/// Returns true when catalog input must not reach editor commands or canvas.
pub(crate) fn route_edge(
    state: &mut EditorState,
    edge: ActionEdge<EditorAction>,
    layout: &Layout,
) -> bool {
    if state.mode != Mode::Editor || state.environment_open || !layout.usable() {
        return false;
    }
    let down = edge.state() == ButtonState::Pressed && !edge.is_cancelled();
    if let InputControl::Key(key) = edge.control() {
        return keyboard(state, key, down, layout);
    }
    if edge.action() != EditorAction::Point {
        return state.catalog.open;
    }
    let geometry = CatalogLayout::new(layout, &state.catalog);
    let sample = edge.pointer().filter(|sample| {
        sample.viewport().width() == layout.width && sample.viewport().height() == layout.height
    });
    let hit = sample.and_then(|sample| geometry.hit(sample.position(), &state.catalog));
    let was_open = state.catalog.open;
    let was_pressed = state.catalog.pressed;
    if !was_open
        && state.catalog.blend > 0.001
        && sample.is_some_and(|sample| geometry.panel.contains(sample.position()))
    {
        state.cancel_gesture();
        return true;
    }
    if down && (was_open || hit.is_some()) {
        state.cancel_gesture();
    }
    let outcome = state.catalog_pointer.process(edge, hit);
    if down {
        state.catalog.pressed = hit;
        if let Some(CatalogTarget::Entry(id) | CatalogTarget::Quick(id)) = hit {
            if let Some(placement) = placement(id) {
                state.drag = Some(Drag::Place {
                    placement,
                    position: state.camera.center,
                    from_palette: true,
                });
            }
        } else if hit.is_none() && was_open {
            if sample.is_none_or(|sample| !geometry.panel.contains(sample.position())) {
                state.catalog.close();
            }
            state.catalog.search_focused = false;
        }
        return was_open || hit.is_some();
    }
    if let Some(PointerButtonEvent::Clicked { target, .. }) = outcome.event() {
        activate_target(state, target);
        return true;
    }
    state.catalog.pressed = None;
    if let Some(CatalogTarget::Entry(id) | CatalogTarget::Quick(id)) = was_pressed {
        if !edge.is_cancelled()
            && state.drag.is_some()
            && sample.is_some_and(|sample| {
                layout.canvas.contains(sample.position())
                    && (!was_open || !geometry.panel.contains(sample.position()))
            })
        {
            // Canvas release commits the captured recipe exactly once.
            state.catalog.close();
            state.catalog.remember(id);
            if let Some(placement) = placement(id) {
                state.brush = placement;
                state.tool = Tool::Build;
            }
            return false;
        }
        state.drag = None;
        return true;
    }
    was_open || was_pressed.is_some() || hit.is_some()
}

pub(crate) fn animate(
    state: &mut EditorState,
    layout: &Layout,
    sample: Option<PointerSample>,
    seconds: f32,
) {
    let blend = 1.0 - (-18.0 * seconds.clamp(0.0, 0.25)).exp();
    let goal = if state.catalog.open && state.mode == Mode::Editor && !state.environment_open {
        1.0
    } else {
        0.0
    };
    state.catalog.blend += (goal - state.catalog.blend) * blend;
    if (goal - state.catalog.blend).abs() < 0.001 {
        state.catalog.blend = goal;
    }
    let geometry = CatalogLayout::new(layout, &state.catalog);
    state.catalog.hovered =
        if state.mode == Mode::Editor && !state.environment_open && layout.usable() {
            sample.and_then(|sample| geometry.hit(sample.position(), &state.catalog))
        } else {
            None
        };
    for category in Category::ALL {
        let target = if state.catalog.hovered == Some(CatalogTarget::Category(category)) {
            1.0
        } else {
            0.0
        };
        state.catalog.emphasis[category.index()] +=
            (target - state.catalog.emphasis[category.index()]) * blend;
    }
}
