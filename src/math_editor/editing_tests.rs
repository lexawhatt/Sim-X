use super::*;
use crate::math_editor::layout::Target;

#[test]
fn typing_undo_groups_stop_at_navigation_pause_and_templates() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "123")?;
    assert_eq!(runner.resource::<state::MathState>().unwrap().undo.len(), 1);
    click(&mut runner, Target::Undo)?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .rows[0]
            .is_empty()
    );
    click(&mut runner, Target::Redo)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .unwrap(),
        "123"
    );
    // A caret move is a new editing intention even when it returns to the same stop.
    frame(&mut runner, &tap(ArrowLeft))?;
    frame(&mut runner, &tap(ArrowRight))?;
    type_text(&mut runner, "4")?;
    assert_eq!(runner.resource::<state::MathState>().unwrap().undo.len(), 2);
    for _ in 0..50 {
        frame(&mut runner, &[])?;
    }
    type_text(&mut runner, "5")?;
    assert_eq!(runner.resource::<state::MathState>().unwrap().undo.len(), 3);
    type_text(&mut runner, "^2")?;
    click(&mut runner, Target::Undo)?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .is_err()
    );
    click(&mut runner, Target::Undo)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .unwrap(),
        "12345"
    );
    Ok(())
}

#[test]
fn held_edit_keys_repeat_with_delay_then_stop_on_release_and_focus_loss() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    frame(&mut runner, &[key(KeyX, ButtonState::Pressed)])?;
    for _ in 0..15 {
        frame(&mut runner, &[])?;
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .rows[0]
            .len(),
        1
    );
    for _ in 0..30 {
        frame(&mut runner, &[])?;
    }
    let count = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .fields[0]
        .rows[0]
        .len();
    assert!(count > 3);
    frame(&mut runner, &[key(KeyX, ButtonState::Released)])?;
    for _ in 0..35 {
        frame(&mut runner, &[])?;
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .rows[0]
            .len(),
        count
    );
    frame(&mut runner, &[key(Backspace, ButtonState::Pressed)])?;
    for _ in 0..32 {
        frame(&mut runner, &[])?;
    }
    let remaining = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .fields[0]
        .rows[0]
        .len();
    assert!(remaining < count - 1);
    frame(&mut runner, &[InputEvent::FocusLost])?;
    for _ in 0..40 {
        frame(&mut runner, &[])?;
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .rows[0]
            .len(),
        remaining
    );
    Ok(())
}

#[test]
fn held_enter_and_shortcuts_do_not_repeat_and_click_revokes_old_hold() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x")?;
    frame(&mut runner, &[key(Enter, ButtonState::Pressed)])?;
    for _ in 0..65 {
        frame(&mut runner, &[])?;
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
    frame(&mut runner, &[key(Enter, ButtonState::Released)])?;
    frame(&mut runner, &[key(KeyY, ButtonState::Pressed)])?;
    click(&mut runner, Target::Field(0))?;
    let before = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .clone();
    for _ in 0..60 {
        frame(&mut runner, &[])?;
    }
    assert!(runner.resource::<state::MathState>().unwrap().document == before);
    frame(&mut runner, &[key(KeyY, ButtonState::Released)])?;
    frame(
        &mut runner,
        &[
            key(ControlLeft, ButtonState::Pressed),
            key(KeyA, ButtonState::Pressed),
        ],
    )?;
    for _ in 0..40 {
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
    Ok(())
}

#[test]
fn destructive_hold_stops_when_empty_row_removal_changes_focus() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "123")?;
    frame(&mut runner, &tap(Enter))?;
    type_text(&mut runner, "9")?;
    frame(&mut runner, &[key(Backspace, ButtonState::Pressed)])?;
    for _ in 0..90 {
        frame(&mut runner, &[])?;
    }
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields.len(), 1);
    assert_eq!(s.document.fields[0].source().unwrap(), "123");
    frame(&mut runner, &[key(Backspace, ButtonState::Released)])?;
    frame(&mut runner, &tap(Backspace))?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .unwrap(),
        "12"
    );
    // The initial press on an already empty row must not rearm on its new focus.
    frame(&mut runner, &tap(Enter))?;
    frame(&mut runner, &[key(Backspace, ButtonState::Pressed)])?;
    for _ in 0..60 {
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
        "12"
    );
    Ok(())
}

#[test]
fn workspace_starts_without_keyboard_but_physical_typing_and_reopening_work() -> LogicResult {
    let (app, world) = app::build_math_editor_application()?;
    let mut runner = app.build_headless(world)?;
    frame(&mut runner, &[])?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(!s.keypad);
    let without_keys = layout::Layout::for_state(viewport(), s).canvas.h;
    type_text(&mut runner, "root(3,-8)")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .plot
            .as_ref()
            .unwrap()
            .rows[0]
            .scalar,
        Some(-2.0)
    );
    click(&mut runner, Target::Keypad)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(s.keypad);
    assert!(layout::Layout::for_state(viewport(), s).canvas.h < without_keys);
    frame(&mut runner, &tap(PhysicalKeyCode::Enter))?;
    click(&mut runner, Target::NthRoot)?;
    type_text(&mut runner, "5,-32)")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .plot
            .as_ref()
            .unwrap()
            .rows[1]
            .scalar,
        Some(-2.0)
    );
    Ok(())
}

#[test]
fn root_entry_and_draft_cpu_preview() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "root(3,x)")?;
    frame(&mut runner, &tap(PhysicalKeyCode::Enter))?;
    type_text(&mut runner, "root(5,-32)")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    let s = runner.resource::<state::MathState>().unwrap();
    let l = layout::Layout::for_state(viewport(), s);
    let fonts = runner.resource::<assets::Fonts>().unwrap();
    let scene = view::scene(s, &l, fonts);
    assert!(l.draft.is_some());
    assert!(scene.texts.iter().any(|t| t.text.contains("Click to add")));
    super::super::visual_probe::export(&scene, layout::Rect::new(0.0, 0.0, l.width, l.height));
    Ok(())
}

#[test]
fn construction_labels_prefer_clear_space_and_have_readable_background() -> LogicResult {
    let runner = runner()?;
    let state = runner.resource::<state::MathState>().unwrap();
    let l = layout::Layout::for_state(viewport(), state);
    let fonts = runner.resource::<assets::Fonts>().unwrap();
    let p = l.canvas.center();
    let mut scene = drawing::Scene::default();
    let a = [p[0], p[1]];
    let b = [p[0] + 80.0, p[1] - 80.0];
    scene.line(a, b, drawing::GEOMETRY, 2.0, l.canvas, 2.0);
    let mut labels = super::super::point_labels::PointLabels::new(&scene, &l, fonts);
    labels.draw(&mut scene, 9, p);
    let label = scene.texts.iter().find(|t| t.text == "P10").unwrap();
    let bounds = layout::Rect::new(
        label.position[0] - 3.0,
        label.position[1] - 16.0,
        fonts.width(2, "P10") + 6.0,
        21.0,
    );
    assert!(drawing::clip_line(a, b, bounds).is_none());
    assert!(scene.panels.iter().any(|panel| panel.rect == bounds));
    Ok(())
}
