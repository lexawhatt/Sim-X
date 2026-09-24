pub(crate) mod clipboard;
mod commands;
pub(super) mod editing;
pub(super) mod parameters;
pub(super) mod rows;

use crate::actions::AppAction;
use crate::math_editor::{
    formula::{Caret, layout::FormulaLayout},
    state::MathState,
    view::{
        assets::Fonts,
        layout::{Layout, Target},
    },
};
use sim_logic::prelude::*;

#[cfg(test)]
mod tests;

pub(super) fn fields(state: &mut MathState, fonts: &Fonts, layout: &Layout) {
    for i in 0..state.document.fields.len() {
        let formula = &state.document.fields[i];
        if state.layout_dirty[i] {
            state.layouts[i] =
                FormulaLayout::build(formula, |style, text| fonts.width(style, text));
            state.layout_dirty[i] = false;
        }
        let rendered = &state.layouts[i];
        let rect = layout.fields[i];
        if state.focus == Some(i)
            && let Some(stop) = rendered.stop(formula.caret)
        {
            let view = rect.w - 24.0;
            state.offsets[i][0] = state.offsets[i][0].min(stop.x).max(stop.x - view).max(0.0);
            if rendered.above + rendered.below > rect.h - 16.0 {
                let top = stop.y - stop.height;
                let bottom = stop.y + stop.height * 0.25;
                state.offsets[i][1] = state.offsets[i][1].min(top).max(bottom - (rect.h - 16.0));
            }
        } else {
            state.offsets[i][1] =
                -rendered.above - (rect.h - 16.0 - rendered.above - rendered.below).max(0.0) * 0.5;
        }
        if rendered.above + rendered.below <= rect.h - 16.0 {
            state.offsets[i][1] =
                -rendered.above - (rect.h - 16.0 - rendered.above - rendered.below) * 0.5;
        }
        state.offsets[i][0] = state.offsets[i][0].min((rendered.width - (rect.w - 24.0)).max(0.0));
    }
}

pub(super) fn route(
    state: Option<ResMut<MathState>>,
    fonts: Option<Res<Fonts>>,
    input: FrameInput<AppAction>,
    viewport: FrameViewport,
    time: FrameTime,
    mut window: AppResMut<WindowControls>,
) -> LogicResult {
    let (Some(mut state), Some(fonts)) = (state, fonts) else {
        return Ok(());
    };
    if state.leaving {
        return Ok(());
    }
    let current = viewport.logical();
    if state.viewport != Some(current) {
        state.cancel_gesture();
        state.viewport = Some(current);
        state.dirty = true;
    }
    let mut layout = Layout::for_state(current, &state);
    state.sidebar_scroll = state.sidebar_scroll.clamp(0.0, layout.max_scroll);
    state.catalog_scroll = state.catalog_scroll.clamp(0.0, layout.catalog_max);
    fields(&mut state, &fonts, &layout);
    if input.focus_lost() {
        if state.clipboard_pending.is_some() {
            state.complete_clipboard(Err("Cancelled on focus loss.".into()));
        }
        state.cancel_gesture();
        // The cancelled releases in this batch are deliberately not routed.
        // Reset ownership so the next ordinary click is not swallowed.
        state.pointer = PointerButton::new(MouseButton::Left);
        state.controls = [false; 2];
        state.shifts = [false; 2];
        state.hovered = None;
        return Ok(());
    }
    for event in input.events() {
        match event {
            FrameInputEvent::Scroll(scroll) => {
                state.editing.reset();
                if state.confirm_back {
                    continue;
                }
                if let Some(p) = scroll.pointer().filter(|p| p.viewport() == current) {
                    let xy = xy(p.position());
                    let steps = match scroll.delta().unit() {
                        ScrollUnit::Lines => scroll.delta().y(),
                        ScrollUnit::Pixels => scroll.delta().y() / 80.0,
                    };
                    if let Some(popup) = layout.popup {
                        if state.functions && popup.contains(xy) {
                            state.catalog_scroll = (state.catalog_scroll - steps as f32 * 44.0)
                                .clamp(0.0, layout.catalog_max);
                            layout = Layout::for_state(current, &state);
                        }
                        state.icon_hold = None;
                        continue;
                    }
                    if layout.list.contains(xy) {
                        state.sidebar_scroll = (state.sidebar_scroll - steps as f32 * 44.0)
                            .clamp(0.0, layout.max_scroll);
                        layout = Layout::for_state(current, &state);
                        state.pointer.cancel();
                        continue;
                    }
                    if layout.canvas.contains(xy) && state.pan.is_none() && state.drag.is_none() {
                        state.zoom_view((steps.clamp(-8.0, 8.0) * 0.12).exp(), xy, layout.canvas);
                        state.dirty = true;
                    }
                }
            }
            FrameInputEvent::Action(edge) => {
                let down = edge.state() == ButtonState::Pressed && !edge.is_cancelled();
                if let InputControl::Key(key) = edge.control() {
                    let was_modal = state.confirm_back;
                    match key {
                        PhysicalKeyCode::ShiftLeft => state.shifts[0] = down,
                        PhysicalKeyCode::ShiftRight => state.shifts[1] = down,
                        PhysicalKeyCode::ControlLeft => state.controls[0] = down,
                        PhysicalKeyCode::ControlRight => state.controls[1] = down,
                        PhysicalKeyCode::F11 if down => {
                            state.fullscreen = !state.fullscreen;
                            window.request_mode(if state.fullscreen {
                                WindowMode::BorderlessFullscreen(FullscreenMonitor::Automatic)
                            } else {
                                WindowMode::Windowed
                            });
                        }
                        _ if down && layout.usable() => state.editor_key(key),
                        _ => {}
                    }
                    if layout.usable() {
                        state.observe_editor_key(key, down);
                    } else {
                        state.editing.reset();
                    }
                    layout = reveal_focus(&mut state, current);
                    fields(&mut state, &fonts, &layout);
                    if was_modal != state.confirm_back {
                        break;
                    }
                    continue;
                }
                if down {
                    state.editing.reset();
                }
                let p = edge
                    .pointer()
                    .filter(|p| p.viewport() == current)
                    .map(|p| xy(p.position()));
                let target = p.and_then(|p| layout.hit(p, state.confirm_back));
                if edge.action() == AppAction::Point
                    && !down
                    && let Some((row, _)) = state.parameter_drag.as_ref()
                {
                    let row = *row;
                    if !edge.is_cancelled()
                        && let Some(p) = p
                        && let Some((_, track, _)) = layout
                            .controls
                            .iter()
                            .find(|(t, _, _)| *t == Target::Slider(row))
                    {
                        state.move_slider(row, p[0], *track);
                    }
                    state.stop_parameter_motion();
                    let _ = state.pointer.process(edge, target);
                    continue;
                }
                if edge.action() == AppAction::Point
                    && down
                    && !state.confirm_back
                    && layout.popup.is_none()
                {
                    if !matches!(target, Some(Target::PlayParameter(_))) {
                        state.stop_parameter_motion();
                    }
                    if let (Some(Target::Slider(row)), Some(p)) = (target, p) {
                        state.parameter_drag = Some((row, state.document.clone()));
                        state.focus = None;
                        if let Some((_, track, _)) = layout
                            .controls
                            .iter()
                            .find(|(t, _, _)| *t == Target::Slider(row))
                        {
                            state.move_slider(row, p[0], *track);
                        }
                    }
                    if !matches!(
                        target,
                        Some(
                            Target::ParameterRange(..)
                                | Target::Type(_)
                                | Target::Backspace
                                | Target::Enter
                                | Target::Alphabet
                                | Target::Keypad
                        )
                    ) {
                        state.range_edit = None;
                    }
                }
                if edge.action() == AppAction::Point {
                    if down
                        && let (Some(Target::Field(i)), Some(p)) = (target, p)
                        && let Some(caret) = caret_at(&state, &layout, i, p)
                    {
                        let extending = state.shifts.iter().any(|s| *s);
                        let formula = &mut state.document.fields[i];
                        let anchor = if extending {
                            formula.anchor.unwrap_or(formula.caret)
                        } else {
                            caret
                        };
                        formula.caret = caret;
                        formula.select_all = false;
                        formula.anchor = extending.then_some(anchor);
                        state.focus = Some(i);
                        state.formula_drag = Some((i, anchor, p));
                    }
                    if !down
                        && let Some((i, anchor, origin)) = state.formula_drag.take()
                        && let Some(p) = p
                        && (p[0] - origin[0]).hypot(p[1] - origin[1]) > 3.0
                    {
                        if !edge.is_cancelled()
                            && let Some(caret) = caret_at(&state, &layout, i, p)
                        {
                            state.document.fields[i].caret = caret;
                            state.document.fields[i].anchor = Some(anchor);
                        }
                        let _ = state.pointer.process(edge, target);
                        continue;
                    }
                    if !down && state.suppress_release {
                        state.suppress_release = false;
                        let _ = state.pointer.process(edge, None);
                        continue;
                    }
                    if down && let (Some(Target::Visible(i)), Some(position)) = (target, p) {
                        state.icon_hold = Some((i, 0.0, position));
                    } else if !down {
                        state.icon_hold = None;
                    }
                }
                let outcome = state.pointer.process(edge, target);
                if !layout.usable() {
                    continue;
                }
                if let Some(PointerButtonEvent::Clicked { target, .. }) = outcome.event() {
                    state.activate(target);
                    if let Target::Field(i) = target
                        && let Some(p) = p
                    {
                        let rect = layout.fields[i];
                        if let Some(caret) = state.layouts[i].hit(
                            p[0] - rect.x - 12.0 + state.offsets[i][0],
                            p[1] - rect.y - 8.0 + state.offsets[i][1],
                        ) {
                            state.document.fields[i].caret = caret;
                            state.document.fields[i].select_all = false;
                            if !state.shifts.iter().any(|s| *s) {
                                state.document.fields[i].anchor = None;
                            }
                        }
                    }
                    // Modal changes own the remainder of this event batch.
                    if matches!(
                        target,
                        Target::Back | Target::CancelBack | Target::ConfirmBack
                    ) {
                        break;
                    }
                    layout = Layout::for_state(current, &state);
                    if !matches!(target, Target::Visible(_) | Target::Field(_)) {
                        layout = reveal_focus(&mut state, current);
                    }
                    fields(&mut state, &fonts, &layout);
                } else if !state.confirm_back && layout.popup.is_none() {
                    if state.spatial.visible() {
                        if edge.action() == AppAction::Point {
                            if down
                                && target.is_none()
                                && let Some(p) = p.filter(|p| layout.canvas.contains(*p))
                            {
                                state.spatial.drag = Some((p, state.spatial.target));
                                state.focus = None;
                            } else if !down {
                                if !edge.is_cancelled()
                                    && let Some(p) = p
                                {
                                    state.spatial.orbit(p);
                                }
                                state.spatial.drag = None;
                            }
                        }
                        continue;
                    }
                    if edge.action() == AppAction::Point {
                        if down
                            && target.is_none()
                            && let Some(p) = p
                                .filter(|p| layout.canvas.contains(*p))
                                .and_then(|p| state.camera.unproject(p, layout.canvas))
                        {
                            state.begin_canvas(p);
                        } else if !down {
                            if !edge.is_cancelled()
                                && let Some(p) = p
                                    .filter(|p| layout.canvas.contains(*p))
                                    .and_then(|p| state.camera.unproject(p, layout.canvas))
                            {
                                state.finish_canvas(p);
                            } else {
                                state.drag = None;
                                state.canvas_pressed = false;
                            }
                        }
                    }
                    if edge.action() == AppAction::Pan {
                        if down && let Some(p) = p.filter(|p| layout.canvas.contains(*p)) {
                            state.camera_target = None;
                            state.pan = Some((p, state.camera));
                            state.pointer.cancel();
                            state.drag = None;
                            state.canvas_pressed = false;
                        } else if !down {
                            state.pan = None;
                        }
                    }
                }
            }
        }
    }
    if let Some(p) = input.pointer().map(|p| xy(p.position())) {
        state.spatial.orbit(p);
        if let Some((row, _)) = state.parameter_drag.as_ref() {
            let row = *row;
            if let Some((_, track, _)) = layout
                .controls
                .iter()
                .find(|(t, _, _)| *t == Target::Slider(row))
            {
                state.move_slider(row, p[0], *track);
            }
        }
        if let Some((i, anchor, origin)) = state.formula_drag
            && (p[0] - origin[0]).hypot(p[1] - origin[1]) > 3.0
            && let Some(caret) = caret_at(&state, &layout, i, p)
        {
            state.document.fields[i].caret = caret;
            state.document.fields[i].anchor = Some(anchor);
        }
        state.hovered = layout.hit(p, state.confirm_back);
        if let Some((origin, camera)) = state.pan {
            let mut candidate = camera;
            candidate.x -= f64::from(p[0] - origin[0]) / camera.scale;
            candidate.y += f64::from(p[1] - origin[1]) / camera.scale;
            if candidate.x.is_finite() && candidate.y.is_finite() && candidate != state.camera {
                state.camera = candidate;
                state.dirty = true;
            }
        }
        if let Some(point) = state.camera.unproject(p, layout.canvas)
            && let Some(drag) = &mut state.drag
        {
            drag.candidate = point;
        }
    } else {
        state.hovered = None;
    }
    if let Some((index, elapsed, origin)) = state.icon_hold {
        if state.hovered == Some(Target::Visible(index))
            && input.pointer().is_some_and(|p| {
                let p = xy(p.position());
                (p[0] - origin[0]).hypot(p[1] - origin[1]) < 6.0
            })
        {
            let elapsed = elapsed + time.seconds_f32();
            if elapsed >= 0.45 {
                state.style_popup = Some(index);
                state.functions = false;
                state.icon_hold = None;
                state.suppress_release = true;
                state.pointer.cancel();
                layout = Layout::for_state(current, &state);
            } else {
                state.icon_hold = Some((index, elapsed, origin));
            }
        } else {
            state.icon_hold = None;
        }
    }
    window.request_cursor(match state.hovered {
        Some(Target::TransitionGuard) => CursorShape::Default,
        Some(Target::Field(_) | Target::Draft) => CursorShape::Text,
        Some(_) => CursorShape::Pointer,
        None => CursorShape::Default,
    });
    state.blink = (state.blink + time.seconds_f32()).rem_euclid(1.0);
    if layout.usable() && state.advance_editing(time.seconds_f32()) {
        layout = reveal_focus(&mut state, current);
    }
    fields(&mut state, &fonts, &layout);
    layout = Layout::for_state(current, &state);
    fields(&mut state, &fonts, &layout);
    state.poll();
    state.advance_parameters(f64::from(time.seconds_f32()));
    state.advance_camera(f64::from(time.seconds_f32()));
    state.submit(&layout);
    state.animate_ui(time.seconds_f32());
    state.advance_integral_reveal(time.seconds_f32());
    Ok(())
}
fn reveal_focus(state: &mut MathState, viewport: LogicalViewport) -> Layout {
    let layout = Layout::for_state(viewport, state);
    if let Some(rect) = state.focus.and_then(|i| {
        let row = layout.rows.get(i)?;
        if row.h <= layout.list.h {
            Some(row)
        } else {
            layout.fields.get(i)
        }
    }) {
        let delta = if rect.y < layout.list.y {
            rect.y - layout.list.y
        } else {
            (rect.y + rect.h - layout.list.y - layout.list.h).max(0.0)
        };
        state.sidebar_scroll = (state.sidebar_scroll + delta).clamp(0.0, layout.max_scroll);
    }
    Layout::for_state(viewport, state)
}
fn xy(p: LogicalScreenPosition) -> [f32; 2] {
    let p = p.to_vec2();
    [p.x(), p.y()]
}
fn caret_at(state: &MathState, layout: &Layout, i: usize, p: [f32; 2]) -> Option<Caret> {
    let rect = layout.fields.get(i)?;
    state.layouts[i].hit(
        p[0] - rect.x - 12.0 + state.offsets[i][0],
        p[1] - rect.y - 8.0 + state.offsets[i][1],
    )
}

pub(super) fn key_press(state: &mut MathState, key: PhysicalKeyCode) {
    use PhysicalKeyCode::*;
    if state.confirm_back {
        if key == Escape {
            state.confirm_back = false;
        }
        return;
    }
    if state.functions || state.style_popup.is_some() {
        if key == Escape {
            state.activate(Target::ClosePopup);
        }
        return;
    }
    let ctrl = state.controls.iter().any(|b| *b);
    let shift = state.shifts.iter().any(|b| *b);
    if state.range_edit.is_some() {
        if key == Escape {
            state.range_edit = None;
        } else if key == Enter {
            state.commit_range();
        } else if let Some((_, _, text)) = state.range_edit.as_mut() {
            if ctrl && key == KeyA {
                text.clear();
            } else if key == Backspace {
                text.pop();
            } else if !ctrl
                && let Some(ch) = character(key, shift)
                && (ch.is_ascii_digit() || "+-.e".contains(ch))
            {
                text.push(ch);
            }
        }
        return;
    }
    state.stop_parameter_motion();
    if ctrl && matches!(key, KeyZ | KeyY) {
        state.history(key == KeyY || shift);
        return;
    }
    if key == Escape {
        if state.functions || state.style_popup.is_some() {
            state.activate(Target::ClosePopup);
            return;
        }
        state.cancel_gesture();
        state.link = None;
        state.focus = None;
        return;
    }
    if key == Enter {
        state.activate(Target::Enter);
        return;
    }
    let Some(index) = state.focus else {
        if key == Home {
            state.activate(Target::Home);
        }
        return;
    };
    if ctrl && key == KeyA {
        state.document.fields[index].select_all = true;
        state.document.fields[index].anchor = None;
        return;
    }
    if ctrl && matches!(key, KeyC | KeyX | KeyV) {
        state.activate(match key {
            KeyC => Target::Copy,
            KeyX => Target::Cut,
            _ => Target::Paste,
        });
        return;
    }
    if ctrl {
        return;
    }
    if key == Backspace && state.erase_empty_expression() {
        return;
    }
    let before = state.document.clone();
    let formula = &mut state.document.fields[index];
    let anchor = formula.anchor.unwrap_or(formula.caret);
    match key {
        ArrowLeft => formula.move_horizontal(false, shift),
        ArrowRight => formula.move_horizontal(true, shift),
        ArrowUp | ArrowDown => {
            if let Some(caret) = state.layouts[index].vertical(formula.caret, key == ArrowUp) {
                formula.caret = caret;
                formula.select_all = false;
                formula.anchor = shift.then_some(anchor);
            } else if !shift {
                state.adjacent_expression(key == ArrowUp);
            }
        }
        Home => {
            formula.caret = Caret { row: 0, index: 0 };
            formula.select_all = false;
            formula.anchor = shift.then_some(anchor);
        }
        End => {
            formula.caret = Caret {
                row: 0,
                index: formula.rows[0].len(),
            };
            formula.select_all = false;
            formula.anchor = shift.then_some(anchor);
        }
        Tab => {
            if formula.caret.row == 0 {
                let order: Vec<_> = (0..state.document.fields.len()).collect();
                let current = order.iter().position(|i| *i == index).unwrap_or(0);
                state.focus = Some(
                    order[if shift {
                        (current + order.len() - 1) % order.len()
                    } else {
                        (current + 1) % order.len()
                    }],
                );
            } else {
                formula.next_slot(shift);
            }
        }
        Backspace => formula.erase(false),
        Delete => formula.erase(true),
        _ => {
            if let Some(character) = character(key, shift) {
                formula.type_char(character);
            }
        }
    }
    // Caret changes are not semantic edits and must not restart quadrature.
    if before
        .fields
        .iter()
        .zip(&state.document.fields)
        .any(|(a, b)| a.rows != b.rows)
    {
        state.remember(before);
    }
    state.blink = 0.0;
}
pub(super) fn character(key: PhysicalKeyCode, shift: bool) -> Option<char> {
    use PhysicalKeyCode::*;
    if let Some(index) = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ]
    .iter()
    .position(|k| *k == key)
    {
        return char::from_u32(u32::from(b'a') + index as u32);
    }
    Some(match (key, shift) {
        (Digit9, true) => '(',
        (Digit0, true) => ')',
        (Digit6, true) => '^',
        (Digit8, true) => '*',
        (Equal, true) => '+',
        (Equal, false) => '=',
        (Comma, false) => ',',
        (Comma, true) => '<',
        (Backslash, true) => '|',
        (Minus, _) => '-',
        (Slash, _) => '/',
        (Period, false) => '.',
        (Period, true) => '>',
        (Space, _) => ' ',
        (Digit0, false) => '0',
        (Digit1, false) => '1',
        (Digit2, false) => '2',
        (Digit3, false) => '3',
        (Digit4, false) => '4',
        (Digit5, false) => '5',
        (Digit6, false) => '6',
        (Digit7, false) => '7',
        (Digit8, false) => '8',
        (Digit9, false) => '9',
        _ => return None,
    })
}
