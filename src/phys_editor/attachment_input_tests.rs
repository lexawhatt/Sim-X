//! Input ownership and body-local endpoint authoring without a native window.

use std::time::Duration;

use sim_logic::prelude::*;

use super::{
    attachment::{Attachment, authored_point},
    document::{LinkKind, ObjectKind, Point},
    input::EditorAction,
    layout::Layout,
    state::{EditorState, Mode, Tool},
};

fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}

fn state(runner: &HeadlessRunner<EditorAction>) -> &EditorState {
    runner.resource::<EditorState>().unwrap()
}

fn runner() -> LogicResult<HeadlessRunner<EditorAction>> {
    fn populate(mut state: ResMut<EditorState>) {
        let body = state
            .document
            .add(ObjectKind::Box, Point::new(-2.0, 0.0))
            .unwrap();
        state.document.set_size(body, 2.0).unwrap();
        state.document.rotate(body, 30.0).unwrap();
        state
            .document
            .add(ObjectKind::Ball, Point::new(2.0, 0.0))
            .unwrap();
        state.tool = Tool::Spring;
    }
    let (mut app, initial) = crate::build_phys_editor_application()?;
    app.add_system(Stage::Startup, populate);
    Ok(app.build_headless(initial)?)
}

fn frame(runner: &mut HeadlessRunner<EditorAction>, events: &[InputEvent]) -> LogicResult {
    match runner.advance_frame(FrameRequest::new(
        Duration::from_millis(16),
        events,
        viewport(),
    )) {
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

fn motion(runner: &HeadlessRunner<EditorAction>, point: Point) -> InputEvent {
    let state = state(runner);
    let projected = state
        .camera
        .project(point, Layout::for_state(viewport(), state).canvas);
    InputEvent::pointer_moved(PointerSample::new(projected, viewport()).unwrap())
}

fn click(runner: &mut HeadlessRunner<EditorAction>, point: Point) -> LogicResult {
    let moved = motion(runner, point);
    frame(
        runner,
        &[
            moved,
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )
}

fn key(runner: &mut HeadlessRunner<EditorAction>, key: PhysicalKeyCode) -> LogicResult {
    frame(
        runner,
        &[
            InputEvent::key(key, ButtonState::Pressed),
            InputEvent::key(key, ButtonState::Released),
        ],
    )
}

#[test]
fn rotated_corner_and_circle_clicks_save_real_points_and_rest_length() -> LogicResult {
    let mut runner = runner()?;
    let object = state(&runner).document.object(1).unwrap();
    let local = Point::new(1.0, 0.5);
    let first = authored_point(object, Point::new(1.03, 0.53));
    click(&mut runner, first)?;
    assert_eq!(
        state(&runner).link_start,
        Some(Attachment {
            body: 1,
            local_m: local
        })
    );
    click(&mut runner, Point::new(1.5, 0.0))?;
    let editor = state(&runner);
    let link = editor.document.links()[0];
    assert_eq!(link.a_local_m, local);
    assert!((link.b_local_m.x + 0.5).abs() < 1e-6);
    assert!(link.b_local_m.y.abs() < 1e-6);
    let from = authored_point(editor.document.object(link.a).unwrap(), link.a_local_m);
    let to = authored_point(editor.document.object(link.b).unwrap(), link.b_local_m);
    let LinkKind::Spring { rest_length_m, .. } = link.kind else {
        panic!("Expected a spring")
    };
    assert!((rest_length_m - (to.x - from.x).hypot(to.y - from.y)).abs() < 1e-12);
    assert!(
        (rest_length_m - 4.0).abs() > 0.5,
        "Rest length must not use body centres"
    );
    assert!(editor.run.is_none());
    assert_eq!(editor.mode, Mode::Editor);
    Ok(())
}

#[test]
fn focus_loss_pointer_leave_escape_and_mode_change_cancel_pending_endpoint() -> LogicResult {
    for cancellation in [InputEvent::FocusLost, InputEvent::PointerLeft] {
        let mut runner = runner()?;
        click(&mut runner, Point::new(-2.0, 0.0))?;
        assert!(state(&runner).link_start.is_some());
        frame(&mut runner, &[cancellation])?;
        assert!(state(&runner).link_start.is_none());
        click(&mut runner, Point::new(2.0, 0.0))?;
        assert!(state(&runner).document.links().is_empty());
        assert!(state(&runner).link_start.is_some());
    }
    let mut runner = runner()?;
    click(&mut runner, Point::new(-2.0, 0.0))?;
    key(&mut runner, PhysicalKeyCode::Escape)?;
    assert!(state(&runner).link_start.is_none());
    click(&mut runner, Point::new(-2.0, 0.0))?;
    key(&mut runner, PhysicalKeyCode::F5)?;
    assert_eq!(state(&runner).mode, Mode::Preview);
    assert!(state(&runner).link_start.is_none());
    assert!(state(&runner).document.links().is_empty());
    key(&mut runner, PhysicalKeyCode::Escape)?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    click(&mut runner, Point::new(2.0, 0.0))?;
    assert!(state(&runner).document.links().is_empty());
    Ok(())
}

#[test]
fn endpoint_requires_press_and_release_on_same_body_and_release_owns_point() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Point::new(-2.0, 0.0))?;
    let first = state(&runner).link_start;
    let second = motion(&runner, Point::new(2.0, 0.0));
    frame(
        &mut runner,
        &[
            second,
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    let wrong_body = motion(&runner, Point::new(-2.0, 0.0));
    frame(
        &mut runner,
        &[
            wrong_body,
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )?;
    assert_eq!(state(&runner).link_start, first);
    assert!(state(&runner).document.links().is_empty());
    let down = motion(&runner, Point::new(2.0, 0.0));
    let release = motion(&runner, Point::new(2.5, 0.0));
    let later = motion(&runner, Point::new(2.0, -0.5));
    frame(
        &mut runner,
        &[
            down,
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            release,
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
            later,
        ],
    )?;
    let point = state(&runner).document.links()[0].b_local_m;
    assert!((point.x - 0.5).abs() < 1e-6);
    assert!(
        point.y.abs() < 1e-6,
        "The post-release cursor cannot change an accepted attachment"
    );
    Ok(())
}

#[test]
fn freshly_authored_rotated_spring_has_no_invented_roundoff_strain() {
    use super::{
        document::{Document, PhysicsEnvironment},
        session::RunSession,
    };
    for angle in [30.0, 37.0, 89.9, 271.3] {
        let mut document = Document::default();
        document
            .set_environment(PhysicsEnvironment {
                gravity_m_s2: Point::default(),
                linear_drag_per_s: 0.0,
            })
            .unwrap();
        let anchor = document
            .add(ObjectKind::Anchor, Point::new(-8.0, 4.0))
            .unwrap();
        let body = document
            .add(ObjectKind::Box, Point::new(3.125, -1.25))
            .unwrap();
        document.set_size(body, 4.0).unwrap();
        document.set_height(body, 2.0).unwrap();
        document.rotate(body, angle).unwrap();
        document
            .add_spring_at(
                Attachment::center(anchor),
                Attachment {
                    body,
                    local_m: Point::new(-2.0, 1.0),
                },
            )
            .unwrap();
        let mut run = RunSession::new(&document).unwrap();
        let before = run.world().snapshot();
        run.advance(0.0);
        for _ in 0..8 {
            run.advance(run.world().solver().fixed_dt_s);
            assert!(run.failure.is_none(), "angle={angle}: {:?}", run.failure);
        }
        let after = run.world().snapshot();
        assert_eq!(
            after.bodies, before.bodies,
            "A freshly authored spring starts at exact rest"
        );
        assert_eq!(after.rotations, before.rotations);
    }
}
