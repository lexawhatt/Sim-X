use super::*;
use crate::math_editor::{
    formula::Caret,
    state,
    view::layout::{self, Target},
};

fn blank_click(runner: &mut HeadlessRunner<crate::actions::AppAction>) -> LogicResult {
    let state = runner.resource::<state::MathState>().unwrap();
    let layout = layout::Layout::for_state(viewport(), state);
    let bottom = layout.rows.last().unwrap().y + layout.rows.last().unwrap().h;
    let p = [
        layout.sidebar - 20.0,
        bottom + (layout.list.y + layout.list.h - bottom) * 0.8,
    ];
    assert_eq!(layout.hit(p, false), Some(Target::Draft));
    click_at(runner, p)
}

fn click_at(runner: &mut HeadlessRunner<crate::actions::AppAction>, p: [f32; 2]) -> LogicResult {
    frame(
        runner,
        &[
            pointer(p),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )
}

#[test]
fn blank_sidebar_creates_focused_draft_and_reuses_empty_row() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    blank_click(&mut runner)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        1
    );
    type_text(&mut runner, "x")?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(layout::Layout::for_state(viewport(), s).draft.is_some());
    blank_click(&mut runner)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields.len(), 2);
    assert_eq!(s.focus, Some(1));
    assert_eq!(s.document.fields[0].source().unwrap(), "x");
    assert!(layout::Layout::for_state(viewport(), s).draft.is_none());
    for _ in 0..3 {
        blank_click(&mut runner)?;
        frame(&mut runner, &tap(PhysicalKeyCode::Enter))?;
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        2
    );
    type_text(&mut runner, "x+1")?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[1]
            .source()
            .unwrap(),
        "x+1"
    );
    Ok(())
}

#[test]
fn blank_sidebar_requires_matching_release_and_respects_popups() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x")?;
    let s = runner.resource::<state::MathState>().unwrap();
    let l = layout::Layout::for_state(viewport(), s);
    let blank = [100.0, l.list.y + l.list.h - 20.0];
    let canvas = l.canvas.center();
    frame(
        &mut runner,
        &[
            pointer(blank),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    frame(
        &mut runner,
        &[
            pointer(canvas),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        1
    );
    frame(
        &mut runner,
        &[
            pointer(blank),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    frame(&mut runner, &[InputEvent::FocusLost])?;
    frame(
        &mut runner,
        &[
            pointer(blank),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        1
    );
    click(&mut runner, Target::Functions)?;
    click_at(&mut runner, blank)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(!s.functions);
    assert_eq!(s.document.fields.len(), 1);
    click_at(&mut runner, canvas)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        1
    );
    blank_click(&mut runner)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        2
    );
    Ok(())
}

#[test]
fn enter_inserts_immediately_after_current_row_without_reordering_metadata() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x")?;
    frame(&mut runner, &tap(Enter))?;
    type_text(&mut runner, "x+2")?;
    let old_style = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .styles[1];
    click(&mut runner, Target::Field(0))?;
    frame(&mut runner, &tap(Enter))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.focus, Some(1));
    assert_eq!(s.document.fields.len(), 3);
    assert!(s.document.fields[1].rows[0].is_empty());
    assert_eq!(s.document.fields[2].source().unwrap(), "x+2");
    assert!(s.document.styles[2] == old_style);
    assert_eq!(s.calculate.len(), 3);
    assert_eq!(s.offsets.len(), 3);
    assert_eq!(s.layouts.len(), 3);
    click(&mut runner, Target::Field(0))?;
    frame(&mut runner, &tap(Enter))?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        3
    );
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().focus,
        Some(1)
    );
    type_text(&mut runner, "7")?;
    until(&mut runner, |s| !s.busy && !s.dirty)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .plot
            .as_ref()
            .unwrap()
            .rows[1]
            .scalar,
        Some(7.0)
    );
    Ok(())
}

#[test]
fn empty_backspace_returns_to_previous_end_and_undo_restores_row() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x+12")?;
    frame(&mut runner, &tap(Enter))?;
    frame(&mut runner, &tap(Backspace))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields.len(), 1);
    assert_eq!(s.focus, Some(0));
    assert_eq!(s.document.fields[0].caret, Caret { row: 0, index: 4 });
    assert_eq!(s.document.fields[0].source().unwrap(), "x+12");
    click(&mut runner, Target::Undo)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        2
    );
    click(&mut runner, Target::Redo)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        1
    );
    select_all(&mut runner)?;
    frame(&mut runner, &tap(Backspace))?;
    frame(&mut runner, &tap(Backspace))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields.len(), 1);
    assert_eq!(s.focus, Some(0));
    assert!(s.document.fields[0].rows[0].is_empty());
    Ok(())
}

#[test]
fn vertical_arrows_prefer_formula_slots_then_adjacent_expressions() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "1/2")?;
    frame(&mut runner, &tap(Enter))?;
    type_text(&mut runner, "x")?;
    frame(&mut runner, &tap(ArrowUp))?;
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().focus,
        Some(0)
    );
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .caret
            .row,
        0
    );
    frame(&mut runner, &tap(ArrowDown))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.focus, Some(0));
    assert_ne!(s.document.fields[0].caret.row, 0);
    frame(&mut runner, &tap(ArrowDown))?;
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().focus,
        Some(1)
    );
    frame(&mut runner, &tap(ArrowDown))?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        2
    );
    Ok(())
}

#[test]
fn integral_enter_calculates_and_shift_enter_creates_next_row() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "int(x)")?;
    frame(&mut runner, &tap(Enter))?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        1
    );
    assert!(runner.resource::<state::MathState>().unwrap().calculate[0]);
    until(&mut runner, |s| !s.busy && !s.dirty)?;
    frame(
        &mut runner,
        &[
            key(ShiftLeft, ButtonState::Pressed),
            key(Enter, ButtonState::Pressed),
            key(Enter, ButtonState::Released),
            key(ShiftLeft, ButtonState::Released),
        ],
    )?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields.len(), 2);
    assert_eq!(s.focus, Some(1));
    assert!(s.calculate[0]);
    Ok(())
}

#[test]
fn inserted_expression_scrolls_into_view_and_tabs_between_rows() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    for _ in 0..8 {
        type_text(&mut runner, "x")?;
        frame(&mut runner, &tap(Enter))?;
        let s = runner.resource::<state::MathState>().unwrap();
        let l = layout::Layout::for_state(viewport(), s);
        let r = l.fields[s.focus.unwrap()];
        assert!(r.y >= l.list.y - 0.01 && r.y + r.h <= l.list.y + l.list.h + 0.01);
    }
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .sidebar_scroll
            > 0.0
    );
    frame(&mut runner, &tap(Tab))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.focus, Some(0));
    assert_eq!(s.sidebar_scroll, 0.0);
    Ok(())
}

#[test]
fn popup_keyboard_cannot_edit_or_create_background_rows() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x")?;
    click(&mut runner, Target::Functions)?;
    for code in [KeyA, Backspace, Enter] {
        frame(&mut runner, &tap(code))?;
    }
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(s.functions);
    assert_eq!(s.document.fields.len(), 1);
    assert_eq!(s.document.fields[0].source().unwrap(), "x");
    frame(&mut runner, &tap(Escape))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(!s.functions);
    assert_eq!(s.focus, Some(0));
    let l = layout::Layout::for_state(viewport(), s);
    let icon = l
        .controls
        .iter()
        .find(|(t, _, _)| *t == Target::Visible(0))
        .unwrap()
        .1;
    frame(
        &mut runner,
        &[
            pointer(icon.center()),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    for _ in 0..32 {
        frame(&mut runner, &[])?;
    }
    frame(
        &mut runner,
        &[InputEvent::mouse_button(
            MouseButton::Left,
            ButtonState::Released,
        )],
    )?;
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().style_popup,
        Some(0)
    );
    for code in [KeyA, Backspace, Enter] {
        frame(&mut runner, &tap(code))?;
    }
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields.len(), 1);
    assert_eq!(s.document.fields[0].source().unwrap(), "x");
    frame(&mut runner, &tap(Escape))?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .style_popup
            .is_none()
    );
    Ok(())
}

#[test]
fn unusable_window_cannot_repeat_keys_into_hidden_rows() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x")?;
    let narrow = LogicalViewport::new(700.0, 600.0)?;
    for i in 0..45 {
        let events = if i == 0 {
            vec![key(KeyA, ButtonState::Pressed)]
        } else {
            vec![]
        };
        match runner.advance_frame(FrameRequest::new(
            std::time::Duration::from_millis(16),
            &events,
            narrow,
        )) {
            FrameOutcome::Advanced(report) => {
                assert!(report.failure().is_none(), "{:?}", report.failure())
            }
            FrameOutcome::Rejected(error) => return Err(error.into()),
        }
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .unwrap(),
        "x"
    );
    frame(&mut runner, &[])?;
    for _ in 0..35 {
        frame(&mut runner, &[])?;
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .unwrap(),
        "x"
    );
    frame(&mut runner, &[key(KeyA, ButtonState::Released)])?;
    Ok(())
}
