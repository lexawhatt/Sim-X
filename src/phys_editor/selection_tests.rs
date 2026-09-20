//! End-to-end selection gestures use real ordered input without a window or GPU.

use std::time::Duration;

use sim_logic::prelude::*;

use super::{
    document::{Document, EditError, Link, Object, ObjectKind, PhysicsEnvironment, Point},
    input::EditorAction,
    layout::Layout,
    selection::SelectionGesture,
    state::{Drag, EditorState, Mode, Tool},
};

type Runner = HeadlessRunner<EditorAction>;

fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}

fn state(runner: &Runner) -> &EditorState {
    runner.resource::<EditorState>().unwrap()
}

fn frame_at(runner: &mut Runner, size: LogicalViewport, events: &[InputEvent]) -> LogicResult {
    match runner.advance_frame(FrameRequest::new(Duration::from_millis(16), events, size)) {
        FrameOutcome::Advanced(report) => {
            assert!(report.failure().is_none(), "{:?}", report.failure());
            assert_eq!(report.fixed_ticks_attempted(), 0);
            assert_eq!(report.spawned(), 0);
            assert_eq!(report.despawned(), 0);
            Ok(())
        }
        FrameOutcome::Rejected(error) => Err(error.into()),
    }
}

fn frame(runner: &mut Runner, events: &[InputEvent]) -> LogicResult {
    frame_at(runner, viewport(), events)
}

fn key(key: PhysicalKeyCode, pressed: bool) -> InputEvent {
    InputEvent::key(
        key,
        if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
    )
}

fn tap(runner: &mut Runner, code: PhysicalKeyCode) -> LogicResult {
    frame(runner, &[key(code, true), key(code, false)])
}

fn shortcut(runner: &mut Runner, code: PhysicalKeyCode) -> LogicResult {
    frame(
        runner,
        &[
            key(PhysicalKeyCode::ControlLeft, true),
            key(code, true),
            key(code, false),
            key(PhysicalKeyCode::ControlLeft, false),
        ],
    )
}

fn press() -> InputEvent {
    InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed)
}

fn release() -> InputEvent {
    InputEvent::mouse_button(MouseButton::Left, ButtonState::Released)
}

fn motion_at(runner: &Runner, point: Point, size: LogicalViewport) -> InputEvent {
    let editor = state(runner);
    let canvas = Layout::for_state(size, editor).canvas;
    let screen = editor.camera.project(point, canvas);
    InputEvent::pointer_moved(PointerSample::new(screen, size).unwrap())
}

fn motion(runner: &Runner, point: Point) -> InputEvent {
    motion_at(runner, point, viewport())
}

fn click(runner: &mut Runner, point: Point) -> LogicResult {
    let moved = motion(runner, point);
    frame(runner, &[moved, press(), release()])
}

fn three_bodies() -> LogicResult<Runner> {
    let (application, initial) = crate::build_phys_editor_application()?;
    let mut runner = application.build_headless(initial)?;
    for x in [-2.0, 0.0, 3.0] {
        click(&mut runner, Point::new(x, 0.0))?;
    }
    assert_eq!(state(&runner).document.objects().len(), 3);
    tap(&mut runner, PhysicalKeyCode::Digit1)?;
    assert_eq!(state(&runner).tool, Tool::Select);
    assert_eq!(selection(&runner), vec![3]);
    Ok(runner)
}

fn linked_bodies() -> LogicResult<Runner> {
    let mut runner = three_bodies()?;
    tap(&mut runner, PhysicalKeyCode::Digit4)?;
    click(&mut runner, Point::new(-2.0, 0.0))?;
    click(&mut runner, Point::new(0.0, 0.0))?;
    click(&mut runner, Point::new(0.0, 0.0))?;
    click(&mut runner, Point::new(3.0, 0.0))?;
    assert_eq!(state(&runner).document.links().len(), 2);
    tap(&mut runner, PhysicalKeyCode::Digit1)?;
    Ok(runner)
}

fn selection(runner: &Runner) -> Vec<u64> {
    state(runner).selection.iter().copied().collect()
}

fn begin(runner: &mut Runner, modifier: PhysicalKeyCode, point: Point) -> LogicResult {
    let moved = motion(runner, point);
    frame(runner, &[key(modifier, true), moved, press()])
}

fn move_to(runner: &mut Runner, point: Point) -> LogicResult {
    let moved = motion(runner, point);
    frame(runner, &[moved])
}

fn finish(runner: &mut Runner, modifier: PhysicalKeyCode, point: Point) -> LogicResult {
    let moved = motion(runner, point);
    frame(runner, &[moved, release(), key(modifier, false)])
}

fn select_first_two(runner: &mut Runner) -> LogicResult {
    begin(runner, PhysicalKeyCode::ShiftLeft, Point::new(-3.0, -1.0))?;
    finish(runner, PhysicalKeyCode::ShiftLeft, Point::new(1.0, 1.0))?;
    assert_eq!(selection(runner), vec![1, 2]);
    Ok(())
}

#[derive(Debug, PartialEq)]
struct AuthoredSnapshot {
    objects: Vec<Object>,
    links: Vec<Link>,
    environment: PhysicsEnvironment,
}

fn authored(document: &Document) -> AuthoredSnapshot {
    AuthoredSnapshot {
        objects: document.objects().to_vec(),
        links: document.links().to_vec(),
        environment: document.environment(),
    }
}

fn history(document: &Document) -> (Vec<AuthoredSnapshot>, Vec<AuthoredSnapshot>) {
    let mut undo_document = document.clone();
    let mut redo_document = document.clone();
    let mut undo = Vec::new();
    let mut redo = Vec::new();
    while undo_document.undo() {
        undo.push(authored(&undo_document));
    }
    while redo_document.redo() {
        redo.push(authored(&redo_document));
    }
    (undo, redo)
}

fn assert_unchanged(before: &Document, after: &Document) {
    assert_eq!(authored(before), authored(after));
    assert_eq!(history(before), history(after));
}

#[test]
fn shift_rectangle_commits_selection_only_on_release_without_document_history() -> LogicResult {
    let mut runner = three_bodies()?;
    let document = state(&runner).document.clone();
    begin(
        &mut runner,
        PhysicalKeyCode::ShiftLeft,
        Point::new(-3.0, -1.0),
    )?;
    assert!(matches!(
        state(&runner).selection_gesture,
        Some(SelectionGesture::Rectangle { .. })
    ));
    move_to(&mut runner, Point::new(1.0, 1.0))?;
    assert_eq!(selection(&runner), vec![3]);
    assert_unchanged(&document, &state(&runner).document);
    finish(
        &mut runner,
        PhysicalKeyCode::ShiftLeft,
        Point::new(1.0, 1.0),
    )?;
    assert_eq!(selection(&runner), vec![1, 2]);
    assert!(state(&runner).selection_gesture.is_none());
    assert_unchanged(&document, &state(&runner).document);
    shortcut(&mut runner, PhysicalKeyCode::KeyZ)?;
    assert_eq!(state(&runner).document.objects().len(), 2);
    Ok(())
}

#[test]
fn control_lasso_closes_implicitly_with_its_release_endpoint() -> LogicResult {
    let mut runner = three_bodies()?;
    let document = state(&runner).document.clone();
    begin(
        &mut runner,
        PhysicalKeyCode::ControlLeft,
        Point::new(-3.0, -1.0),
    )?;
    assert!(matches!(
        state(&runner).selection_gesture,
        Some(SelectionGesture::Lasso { .. })
    ));
    move_to(&mut runner, Point::new(1.0, -1.0))?;
    move_to(&mut runner, Point::new(1.0, 1.0))?;
    assert_eq!(selection(&runner), vec![3]);
    assert_unchanged(&document, &state(&runner).document);
    // The final vertex only arrives with release, not a previous held frame.
    finish(
        &mut runner,
        PhysicalKeyCode::ControlLeft,
        Point::new(-3.0, 1.0),
    )?;
    assert_eq!(selection(&runner), vec![1, 2]);
    assert!(state(&runner).selection_gesture.is_none());
    assert_unchanged(&document, &state(&runner).document);
    Ok(())
}

#[test]
fn rectangle_uses_event_time_release_not_later_pointer_motion_in_same_frame() -> LogicResult {
    let mut runner = three_bodies()?;
    begin(
        &mut runner,
        PhysicalKeyCode::ShiftLeft,
        Point::new(-3.0, -1.0),
    )?;
    let release_position = motion(&runner, Point::new(1.0, 1.0));
    let late_position = motion(&runner, Point::new(5.0, 2.0));
    frame(
        &mut runner,
        &[
            release_position,
            release(),
            late_position,
            key(PhysicalKeyCode::ShiftLeft, false),
        ],
    )?;
    assert_eq!(selection(&runner), vec![1, 2]);
    Ok(())
}

#[test]
fn cancelled_rectangle_preserves_previous_selection_and_entire_document_history() -> LogicResult {
    for cancellation in [0, 1, 2, 3] {
        let mut runner = three_bodies()?;
        let document = state(&runner).document.clone();
        begin(
            &mut runner,
            PhysicalKeyCode::ShiftLeft,
            Point::new(-3.0, -1.0),
        )?;
        move_to(&mut runner, Point::new(1.0, 1.0))?;
        let size = match cancellation {
            0 => {
                tap(&mut runner, PhysicalKeyCode::Escape)?;
                viewport()
            }
            1 => {
                frame(&mut runner, &[InputEvent::FocusLost])?;
                viewport()
            }
            2 => {
                let size = LogicalViewport::new(1100.0, 740.0)?;
                frame_at(&mut runner, size, &[])?;
                size
            }
            _ => {
                frame(&mut runner, &[InputEvent::PointerLeft])?;
                viewport()
            }
        };
        assert!(state(&runner).selection_gesture.is_none());
        assert_eq!(selection(&runner), vec![3]);
        let moved = motion_at(&runner, Point::new(1.0, 1.0), size);
        frame_at(
            &mut runner,
            size,
            &[moved, release(), key(PhysicalKeyCode::ShiftLeft, false)],
        )?;
        assert_eq!(selection(&runner), vec![3]);
        assert_unchanged(&document, &state(&runner).document);
    }
    Ok(())
}

#[test]
fn group_drag_is_one_atomic_edit_and_keeps_internal_rods() -> LogicResult {
    let mut runner = linked_bodies()?;
    select_first_two(&mut runner)?;
    let document = state(&runner).document.clone();
    let start = motion(&runner, Point::new(-2.0, 0.0));
    frame(&mut runner, &[start, press()])?;
    assert!(matches!(
        state(&runner).drag,
        Some(Drag::MoveSelection { .. })
    ));
    move_to(&mut runner, Point::new(-1.0, 1.0))?;
    assert_unchanged(&document, &state(&runner).document);
    // A later pointer event must not inflate the committed translation.
    let released_at = motion(&runner, Point::new(-1.0, 1.0));
    let late_motion = motion(&runner, Point::new(2.0, 2.0));
    frame(&mut runner, &[released_at, release(), late_motion])?;
    let moved = state(&runner).document.clone();
    for id in [1, 2] {
        let old = document.object(id).unwrap();
        let new = moved.object(id).unwrap();
        assert_eq!(
            new.position,
            Point::new(old.position.x + 1.0, old.position.y + 1.0)
        );
    }
    assert_eq!(document.object(3), moved.object(3));
    assert_eq!(moved.links()[0], document.links()[0]);
    assert_ne!(moved.links()[1], document.links()[1]);
    assert_eq!(selection(&runner), vec![1, 2]);
    shortcut(&mut runner, PhysicalKeyCode::KeyZ)?;
    assert_eq!(authored(&state(&runner).document), authored(&document));
    shortcut(&mut runner, PhysicalKeyCode::KeyY)?;
    assert_eq!(authored(&state(&runner).document), authored(&moved));
    Ok(())
}

#[test]
fn group_delete_cascades_internal_and_external_links_in_one_undo() -> LogicResult {
    let mut runner = linked_bodies()?;
    select_first_two(&mut runner)?;
    let document = state(&runner).document.clone();
    tap(&mut runner, PhysicalKeyCode::Delete)?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    assert_eq!(state(&runner).document.objects()[0].id, 3);
    assert!(state(&runner).document.links().is_empty());
    assert!(selection(&runner).is_empty());
    shortcut(&mut runner, PhysicalKeyCode::KeyZ)?;
    assert_eq!(authored(&state(&runner).document), authored(&document));
    shortcut(&mut runner, PhysicalKeyCode::KeyY)?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    assert!(state(&runner).document.links().is_empty());
    Ok(())
}

#[test]
fn group_selection_is_select_only_and_cannot_pass_through_run_or_modals() -> LogicResult {
    for owner in [0, 1, 2, 3] {
        for modifier in [PhysicalKeyCode::ShiftLeft, PhysicalKeyCode::ControlLeft] {
            let mut runner = three_bodies()?;
            match owner {
                0 => tap(&mut runner, PhysicalKeyCode::Digit2)?,
                1 => tap(&mut runner, PhysicalKeyCode::F5)?,
                2 => tap(&mut runner, PhysicalKeyCode::KeyE)?,
                _ => shortcut(&mut runner, PhysicalKeyCode::KeyK)?,
            }
            match owner {
                0 => assert_eq!(state(&runner).tool, Tool::Build),
                1 => assert_eq!(state(&runner).mode, Mode::Preview),
                2 => assert!(state(&runner).environment_open),
                _ => assert!(state(&runner).catalog.open),
            }
            let document = state(&runner).document.clone();
            begin(&mut runner, modifier, Point::new(-3.0, -1.0))?;
            move_to(&mut runner, Point::new(1.0, 1.0))?;
            assert!(state(&runner).selection_gesture.is_none());
            assert!(!matches!(
                state(&runner).drag,
                Some(Drag::MoveSelection { .. })
            ));
            assert_unchanged(&document, &state(&runner).document);
            // Build can author normally; cancel its placement instead of releasing.
            if owner == 0 {
                tap(&mut runner, PhysicalKeyCode::Escape)?;
            }
            finish(&mut runner, modifier, Point::new(1.0, 1.0))?;
            assert!(state(&runner).selection_gesture.is_none());
            assert_unchanged(&document, &state(&runner).document);
        }
    }
    Ok(())
}

#[test]
fn group_translation_rejection_keeps_all_objects_links_and_undo_redo_history() {
    let mut document = Document::default();
    let first = document.add(ObjectKind::Ball, Point::default()).unwrap();
    let boundary = document
        .add(ObjectKind::Ball, Point::new(1_000_000.0, 0.0))
        .unwrap();
    document.set_mass(first, 2.0).unwrap();
    assert!(document.undo());
    assert!(document.can_redo());
    let before = document.clone();
    assert_eq!(
        document.translate_many(&[first, boundary], Point::new(1.0, 0.0)),
        Err(EditError::InvalidPosition)
    );
    assert_unchanged(&before, &document);
    assert_eq!(
        document.translate_many(&[first, u64::MAX], Point::new(1.0, 0.0)),
        Err(EditError::MissingObject(u64::MAX))
    );
    assert_unchanged(&before, &document);
    assert_eq!(
        document.translate_many(&[first], Point::new(f64::NAN, 0.0)),
        Err(EditError::InvalidPosition)
    );
    assert_unchanged(&before, &document);
}

#[test]
fn group_translation_rejects_coincident_external_link_atomically() {
    let mut document = Document::default();
    let first = document.add(ObjectKind::Ball, Point::default()).unwrap();
    let second = document
        .add(ObjectKind::Ball, Point::new(1.0, 0.0))
        .unwrap();
    let third = document
        .add(ObjectKind::Ball, Point::new(3.0, 0.0))
        .unwrap();
    document.add_rod(first, second).unwrap();
    document.add_spring(second, third).unwrap();
    let before = document.clone();
    assert_eq!(
        document.translate_many(&[first, third], Point::new(1.0, 0.0)),
        Err(EditError::InvalidLinkLength)
    );
    assert_unchanged(&before, &document);
}

#[test]
fn duplicate_group_ids_do_not_duplicate_translation_or_deletion() {
    let mut document = Document::default();
    let id = document.add(ObjectKind::Ball, Point::default()).unwrap();
    document
        .translate_many(&[id, id], Point::new(1.0, 0.0))
        .unwrap();
    assert_eq!(document.object(id).unwrap().position, Point::new(1.0, 0.0));
    assert!(document.undo());
    assert_eq!(document.object(id).unwrap().position, Point::default());
    document.remove_many(&[id, id]).unwrap();
    assert!(document.objects().is_empty());
    assert!(document.undo());
    assert_eq!(document.objects().len(), 1);
}
