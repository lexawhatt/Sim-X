//! Regression tests for the authoring/run boundary and real fixed-step playback.

use std::time::Duration;

use sim_logic::prelude::*;

use super::{
    catalog::{CatalogTarget, EntryId},
    catalog_layout::CatalogLayout,
    document::{Document, LinkKind, ObjectKind, PhysicsEnvironment, Point},
    input::EditorAction,
    layout::{Layout, Rect},
    session::{MAX_STEPS_PER_FRAME, PlaybackSpeed, RunSession},
    state::{Control, EditorState, Mode, Tool},
    view::VisualSlot,
};

fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}

fn runner() -> LogicResult<HeadlessRunner<EditorAction>> {
    let (app, initial) = crate::build_phys_editor_application()?;
    Ok(app.build_headless(initial)?)
}

fn state(runner: &HeadlessRunner<EditorAction>) -> &EditorState {
    runner.resource::<EditorState>().unwrap()
}

fn run(runner: &HeadlessRunner<EditorAction>) -> &RunSession {
    state(runner).run.as_ref().unwrap()
}

fn frame(runner: &mut HeadlessRunner<EditorAction>, events: &[InputEvent]) -> LogicResult {
    let request = FrameRequest::new(Duration::from_millis(16), events, viewport());
    match runner.advance_frame(request) {
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

fn center(rect: Rect) -> LogicalScreenPosition {
    LogicalScreenPosition::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn click_events(point: LogicalScreenPosition) -> [InputEvent; 3] {
    [
        InputEvent::pointer_moved(PointerSample::new(point, viewport()).unwrap()),
        InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
    ]
}

fn click_at(
    runner: &mut HeadlessRunner<EditorAction>,
    point: LogicalScreenPosition,
) -> LogicResult {
    frame(runner, &click_events(point))
}

fn click(runner: &mut HeadlessRunner<EditorAction>, control: Control) -> LogicResult {
    if matches!(control, Control::Tool(Tool::Rod | Tool::Spring)) {
        return frame(
            runner,
            &key(if control == Control::Tool(Tool::Rod) {
                PhysicalKeyCode::Digit4
            } else {
                PhysicalKeyCode::Digit5
            }),
        );
    }
    if control == Control::Pendulum {
        let editor = state(runner);
        let layout = CatalogLayout::new(&Layout::for_state(viewport(), editor), &editor.catalog);
        click_at(runner, center(layout.opener()))?;
        for _ in 0..30 {
            frame(runner, &[])?;
        }
        let editor = state(runner);
        let layout = CatalogLayout::new(&Layout::for_state(viewport(), editor), &editor.catalog);
        click_at(
            runner,
            center(layout.category(super::catalog::Category::Assemblies)),
        )?;
        let editor = state(runner);
        let layout = CatalogLayout::new(&Layout::for_state(viewport(), editor), &editor.catalog);
        click_at(
            runner,
            center(
                layout
                    .target(CatalogTarget::Entry(EntryId::Pendulum), &editor.catalog)
                    .unwrap(),
            ),
        )?;
        for _ in 0..30 {
            frame(runner, &[])?;
        }
        return click_world(runner, Point::new(0.0, 1.5));
    }
    let layout = Layout::for_state(viewport(), state(runner));
    click_at(runner, center(layout.control(control).unwrap()))
}

fn click_world(runner: &mut HeadlessRunner<EditorAction>, point: Point) -> LogicResult {
    let editor = state(runner);
    let layout = Layout::for_state(viewport(), editor);
    click_at(runner, editor.camera.project(point, layout.canvas))
}

fn key(key: PhysicalKeyCode) -> [InputEvent; 2] {
    [
        InputEvent::key(key, ButtonState::Pressed),
        InputEvent::key(key, ButtonState::Released),
    ]
}

fn shortcut(key_code: PhysicalKeyCode) -> [InputEvent; 4] {
    let [down, up] = key(key_code);
    [
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Pressed),
        down,
        up,
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Released),
    ]
}

fn assert_same_document(actual: &Document, expected: &Document) {
    assert_eq!(actual.objects(), expected.objects());
    assert_eq!(actual.links(), expected.links());
    assert_eq!(actual.environment(), expected.environment());
    assert_eq!(actual.can_undo(), expected.can_undo());
    assert_eq!(actual.can_redo(), expected.can_redo());
}

#[test]
fn collider_adapter_copies_real_dimensions_mobility_and_material_without_anchors() {
    let mut document = Document::default();
    let rectangle = document.add(ObjectKind::Box, Point::default()).unwrap();
    document.set_size(rectangle, 4.0).unwrap();
    document.set_height(rectangle, 2.0).unwrap();
    document.rotate(rectangle, 30.0).unwrap();
    document.set_fixed(rectangle, true).unwrap();
    document.set_restitution(rectangle, 0.6).unwrap();
    let ball = document
        .add(ObjectKind::Ball, Point::new(8.0, 0.0))
        .unwrap();
    document.set_size(ball, 3.0).unwrap();
    // A marker may occupy a collider's centre: it has no collision capability.
    let anchor = document.add(ObjectKind::Anchor, Point::default()).unwrap();
    let session = RunSession::new(&document).unwrap();
    let world = session.world();
    assert_eq!(world.bodies().len(), 3);
    assert_eq!(world.colliders().len(), 2);
    assert_eq!(world.colliders()[0].restitution, 0.6);
    assert_eq!(
        world.colliders()[0].shape,
        sim_physics::CollisionShape::Box {
            half_extents_m: sim_physics::Vec2::new(2.0, 1.0),
            angle_rad: 0.0,
        }
    );
    let rotation = world
        .rotation(sim_physics::BodyId::new(rectangle).unwrap())
        .unwrap();
    assert_eq!(rotation.angle_rad, 30.0_f64.to_radians());
    assert!((rotation.inertia_kg_m2 - 5.0 / 3.0).abs() < 1e-12);
    assert_eq!(
        world.colliders()[1].shape,
        sim_physics::CollisionShape::Circle { radius_m: 1.5 }
    );
    for (id, mobility) in [
        (rectangle, sim_physics::Mobility::Fixed),
        (ball, sim_physics::Mobility::Dynamic),
        (anchor, sim_physics::Mobility::Fixed),
    ] {
        assert_eq!(
            world
                .body(sim_physics::BodyId::new(id).unwrap())
                .unwrap()
                .mobility,
            mobility
        );
    }
    let snapshot = world.snapshot();
    document.set_size(ball, 4.0).unwrap();
    document.set_restitution(rectangle, 0.0).unwrap();
    assert_eq!(
        session.world().snapshot(),
        snapshot,
        "Run owns its physical descriptions"
    );
}

#[test]
fn bounce_lab_is_an_ordinary_atomic_composition_and_restitution_changes_rebound() {
    let mut document = Document::default();
    let selected = document.add_bounce_lab(Point::default()).unwrap();
    let authored = document.clone();
    assert_eq!(document.objects().len(), 3);
    assert!(document.links().is_empty());
    assert_eq!(document.objects()[2].id, selected);
    assert!(document.objects()[0].fixed);
    assert!(document.undo());
    assert!(document.objects().is_empty());
    assert!(!document.can_undo());
    assert!(document.redo());
    assert_eq!(document.objects(), authored.objects());

    let mut session = RunSession::new(&document).unwrap();
    session.advance(0.0);
    let dt = session.world().solver().fixed_dt_s;
    let mut rebounded = [false; 2];
    let mut peaks = [f64::NEG_INFINITY; 2];
    let mut contact_seen = false;
    for _ in 0..720 {
        session.advance(dt);
        assert!(session.failure.is_none(), "{:?}", session.failure);
        let report = session.world().last_report().unwrap();
        contact_seen |= !report.contacts.is_empty();
        for index in 0..2 {
            let body = &report.bodies[index + 1];
            rebounded[index] |= body.velocity_m_s.y > 0.01;
            if rebounded[index] {
                peaks[index] = peaks[index].max(body.position_m.y);
            }
            assert!(
                body.position_m.y >= -2.250001,
                "penetrated the authored platform"
            );
        }
        assert_eq!(
            report.bodies[0].position_m,
            sim_physics::Vec2::new(0.0, -3.0)
        );
    }
    assert!(contact_seen);
    assert!(rebounded.into_iter().all(|bounced| bounced));
    assert!(
        peaks[1] > peaks[0] + 0.8,
        "different restitution must change the actual trajectory: {peaks:?}"
    );
    assert_same_document(&document, &authored);
}

#[test]
fn intersecting_authored_colliders_reject_run_without_rewriting_the_document() {
    let mut document = Document::default();
    document.add(ObjectKind::Ball, Point::default()).unwrap();
    document
        .add(ObjectKind::Box, Point::new(0.25, 0.0))
        .unwrap();
    let before = document.clone();
    assert!(matches!(
        RunSession::new(&document),
        Err(sim_physics::Error::InitialOverlap(_, _))
    ));
    assert_same_document(&document, &before);
}

#[test]
fn inspector_collision_settings_are_undoable_and_fixed_geometry_does_not_fall() -> LogicResult {
    let mut runner = runner()?;
    let editor = state(&runner);
    let catalog = CatalogLayout::new(&Layout::for_state(viewport(), editor), &editor.catalog);
    let target = center(
        catalog
            .target(CatalogTarget::Quick(EntryId::Box), &editor.catalog)
            .unwrap(),
    );
    click_at(&mut runner, target)?;
    click_world(&mut runner, Point::default())?;
    let id = state(&runner).selected.unwrap();
    click(&mut runner, Control::SizeUp)?;
    click(&mut runner, Control::HeightDown)?;
    click(&mut runner, Control::RotateRight)?;
    click(&mut runner, Control::Fixed)?;
    click(&mut runner, Control::RestitutionUp)?;
    let object = state(&runner).document.object(id).unwrap();
    assert_eq!(object.size_m, 2.0);
    assert_eq!(object.height_m, 0.5);
    assert_eq!(object.rotation_deg, 15.0);
    assert!(object.fixed);
    assert!((object.restitution - 0.45).abs() < 1e-12);
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
    assert_eq!(
        state(&runner).document.object(id).unwrap().restitution,
        0.35
    );
    let authored = state(&runner).document.clone();
    click(&mut runner, Control::Run)?;
    for _ in 0..100 {
        frame(&mut runner, &[])?;
    }
    assert!(run(&runner).failure.is_none());
    assert_eq!(run(&runner).position(id), Some(Point::default()));
    assert_eq!(run(&runner).world().colliders().len(), 1);
    click(&mut runner, Control::Back)?;
    assert_same_document(&state(&runner).document, &authored);
    Ok(())
}

#[test]
fn bounce_lab_quick_access_places_no_objects_until_canvas_confirmation() -> LogicResult {
    let mut runner = runner()?;
    let editor = state(&runner);
    let catalog = CatalogLayout::new(&Layout::for_state(viewport(), editor), &editor.catalog);
    let target = center(
        catalog
            .target(CatalogTarget::Quick(EntryId::BounceLab), &editor.catalog)
            .unwrap(),
    );
    click_at(&mut runner, target)?;
    assert!(state(&runner).document.objects().is_empty());
    click_world(&mut runner, Point::new(0.0, 1.0))?;
    assert_eq!(state(&runner).document.objects().len(), 3);
    let authored = state(&runner).document.clone();
    for _ in 0..30 {
        frame(&mut runner, &[])?;
    }
    assert!(state(&runner).run.is_none());
    assert_same_document(&state(&runner).document, &authored);
    click(&mut runner, Control::Run)?;
    for _ in 0..90 {
        frame(&mut runner, &[])?;
    }
    assert!(run(&runner).failure.is_none(), "{:?}", run(&runner).failure);
    assert_ne!(
        run(&runner).position(authored.objects()[1].id),
        Some(authored.objects()[1].position)
    );
    assert_same_document(&state(&runner).document, &authored);
    Ok(())
}

#[test]
fn rejected_overlap_start_stays_in_editor_and_preserves_undo() -> LogicResult {
    let mut runner = runner()?;
    click_world(&mut runner, Point::default())?;
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyD))?;
    assert_eq!(state(&runner).document.objects().len(), 2);
    let before = state(&runner).document.clone();
    click(&mut runner, Control::Run)?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert!(state(&runner).run.is_none());
    assert!(state(&runner).notice.is_some());
    assert_same_document(&state(&runner).document, &before);
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    Ok(())
}

#[test]
fn prepared_pendulum_runs_only_in_an_independent_view_instance() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Control::Pendulum)?;
    let authored = state(&runner).document.clone();
    assert_eq!(authored.objects().len(), 2);
    assert_eq!(authored.links().len(), 1);
    assert!(matches!(
        authored.links()[0].kind,
        LinkKind::Rod { length_m: 3.0 }
    ));
    let bob = state(&runner).selected.unwrap();
    assert_eq!(authored.object(bob).unwrap().kind, ObjectKind::Ball);
    for _ in 0..20 {
        frame(&mut runner, &[])?;
        assert!(state(&runner).run.is_none());
        assert_same_document(&state(&runner).document, &authored);
    }
    click(&mut runner, Control::Run)?;
    assert_eq!(state(&runner).mode, Mode::Preview);
    assert_eq!(
        run(&runner).world().step_index(),
        0,
        "entrance time is not simulated"
    );
    assert_eq!(run(&runner).world().bodies().len(), 2);
    assert_eq!(run(&runner).world().links().len(), 1);
    for _ in 0..40 {
        frame(&mut runner, &[])?;
    }
    assert!(run(&runner).failure.is_none());
    assert!(run(&runner).world().step_index() > 0);
    assert_ne!(
        run(&runner).position(bob).unwrap(),
        authored.object(bob).unwrap().position
    );
    assert_same_document(&state(&runner).document, &authored);
    click(&mut runner, Control::Back)?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert!(state(&runner).run.is_none());
    assert_same_document(&state(&runner).document, &authored);
    click(&mut runner, Control::Run)?;
    assert_eq!(run(&runner).world().step_index(), 0);
    assert_eq!(
        run(&runner).position(bob).unwrap(),
        authored.object(bob).unwrap().position
    );
    Ok(())
}

#[test]
fn playback_speeds_change_work_not_fixed_dt_and_pause_discards_elapsed() {
    let mut document = Document::default();
    document.add(ObjectKind::Ball, Point::default()).unwrap();
    let mut session = RunSession::new(&document).unwrap();
    let dt = session.world().solver().fixed_dt_s;
    session.advance(100.0);
    assert_eq!(session.world().step_index(), 0);
    assert_eq!(session.dropped_simulation_s, 0.0);
    session.advance(dt * 8.0);
    assert_eq!(session.world().step_index(), 8);
    session.pause(true);
    let paused = session.world().snapshot();
    session.advance(10.0);
    assert_eq!(session.world().snapshot(), paused);
    session.pause(false);
    session.advance(10.0);
    assert_eq!(session.world().snapshot(), paused);
    for (speed, expected_steps) in [
        (PlaybackSpeed::Quarter, 2),
        (PlaybackSpeed::Normal, 8),
        (PlaybackSpeed::Fast, 32),
    ] {
        let before = session.world().step_index();
        session.set_speed(speed);
        session.advance(10.0);
        assert_eq!(session.world().step_index(), before);
        session.advance(dt * 8.0);
        assert_eq!(session.world().step_index() - before, expected_steps);
        assert_eq!(session.world().solver().fixed_dt_s, dt);
        assert!(session.failure.is_none());
    }
    assert_eq!(session.dropped_simulation_s, 0.0);
}

#[test]
fn catch_up_work_is_capped_with_explicit_reported_time_loss() {
    let mut session = RunSession::new(&Document::default()).unwrap();
    session.advance(0.0);
    session.advance(2.0);
    assert_eq!(session.world().step_index(), MAX_STEPS_PER_FRAME as u64);
    assert!(session.dropped_simulation_s > 1.0);
    assert!((session.world().elapsed_s() + session.dropped_simulation_s - 2.0).abs() < 1e-12);
    let before = session.world().step_index();
    session.advance(0.0);
    assert_eq!(
        session.world().step_index(),
        before,
        "no hidden catch-up debt remains"
    );
}

#[test]
fn editor_environment_is_undoable_and_modal_blocks_authoring_and_run() -> LogicResult {
    let mut runner = runner()?;
    click_world(&mut runner, Point::default())?;
    let authored = state(&runner).document.clone();
    let background = Layout::new(viewport(), Mode::Editor);
    click(&mut runner, Control::Environment)?;
    assert!(state(&runner).environment_open);
    for control in [Control::Run, Control::Delete] {
        click_at(&mut runner, center(background.control(control).unwrap()))?;
    }
    for key_code in [
        PhysicalKeyCode::F5,
        PhysicalKeyCode::Delete,
        PhysicalKeyCode::Digit2,
    ] {
        frame(&mut runner, &key(key_code))?;
    }
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
    click_world(&mut runner, Point::new(-4.0, 2.0))?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert_same_document(&state(&runner).document, &authored);
    click(&mut runner, Control::GravityUp)?;
    assert_eq!(
        state(&runner).document.environment().gravity_m_s2.y,
        -8.80665
    );
    click(&mut runner, Control::CloseEnvironment)?;
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
    assert_eq!(
        state(&runner).document.environment(),
        authored.environment()
    );
    assert_eq!(state(&runner).document.objects(), authored.objects());
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyY))?;
    assert_eq!(
        state(&runner).document.environment().gravity_m_s2.y,
        -8.80665
    );
    Ok(())
}

#[test]
fn opening_and_closing_environment_quarantines_queued_tail_actions() -> LogicResult {
    let mut runner = runner()?;
    click_world(&mut runner, Point::default())?;
    let authored = state(&runner).document.clone();
    let [open, open_up] = key(PhysicalKeyCode::KeyE);
    let [escape, escape_up] = key(PhysicalKeyCode::Escape);
    let [delete, delete_up] = key(PhysicalKeyCode::Delete);
    frame(
        &mut runner,
        &[open, open_up, escape, escape_up, delete, delete_up],
    )?;
    assert!(state(&runner).environment_open);
    assert_same_document(&state(&runner).document, &authored);
    let [escape, escape_up] = key(PhysicalKeyCode::Escape);
    let [delete, delete_up] = key(PhysicalKeyCode::Delete);
    let [run, run_up] = key(PhysicalKeyCode::F5);
    frame(
        &mut runner,
        &[escape, escape_up, delete, delete_up, run, run_up],
    )?;
    assert!(!state(&runner).environment_open);
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert_same_document(&state(&runner).document, &authored);
    Ok(())
}

#[test]
fn live_environment_changes_every_free_body_without_rewriting_authoring() -> LogicResult {
    let mut runner = runner()?;
    click_world(&mut runner, Point::new(-2.0, 1.0))?;
    click_world(&mut runner, Point::new(2.0, 1.0))?;
    let authored = state(&runner).document.clone();
    click(&mut runner, Control::Run)?;
    frame(&mut runner, &[])?;
    click(&mut runner, Control::Environment)?;
    let before = run(&runner).world().snapshot();
    let background = Layout::new(viewport(), Mode::Preview);
    click_at(
        &mut runner,
        center(background.control(Control::Back).unwrap()),
    )?;
    frame(&mut runner, &key(PhysicalKeyCode::Delete))?;
    for _ in 0..8 {
        frame(&mut runner, &[])?;
        assert_eq!(run(&runner).world().snapshot(), before);
    }
    assert_eq!(state(&runner).mode, Mode::Preview);
    click(&mut runner, Control::ZeroGravity)?;
    click(&mut runner, Control::GravityRight)?;
    assert_eq!(
        run(&runner).world().settings_revision(),
        before.settings_revision + 2
    );
    assert_eq!(run(&runner).world().elapsed_s(), before.elapsed_s);
    assert_eq!(run(&runner).world().bodies(), before.bodies);
    assert_eq!(
        run(&runner).environment().gravity_m_s2,
        Point::new(1.0, 0.0)
    );
    assert_same_document(&state(&runner).document, &authored);
    click(&mut runner, Control::CloseEnvironment)?;
    assert_eq!(run(&runner).world().elapsed_s(), before.elapsed_s);
    frame(&mut runner, &[])?;
    let report = run(&runner).world().last_report().unwrap();
    assert_eq!(report.bodies.len(), 2);
    for body in &report.bodies {
        assert!((body.acceleration_m_s2.x - 1.0).abs() < 1e-10);
        assert!(body.acceleration_m_s2.y.abs() < 1e-10);
        assert!(body.velocity_m_s.x > 0.0);
    }
    assert_same_document(&state(&runner).document, &authored);
    Ok(())
}

#[test]
fn rod_shortcut_uses_two_endpoints_cancel_duplicate_validation_and_cascade_undo() -> LogicResult {
    let mut runner = runner()?;
    let left = Point::new(-2.0, 1.0);
    let right = Point::new(2.0, 1.0);
    click_world(&mut runner, left)?;
    click_world(&mut runner, right)?;
    frame(&mut runner, &key(PhysicalKeyCode::Digit4))?;
    assert_eq!(state(&runner).tool, Tool::Rod);
    click_world(&mut runner, left)?;
    assert!(state(&runner).link_start.is_some());
    assert!(state(&runner).document.links().is_empty());
    frame(&mut runner, &key(PhysicalKeyCode::Escape))?;
    assert!(state(&runner).link_start.is_none());
    click_world(&mut runner, left)?;
    click_world(&mut runner, right)?;
    assert_eq!(state(&runner).document.links().len(), 1);
    assert_eq!(
        state(&runner).document.links()[0].kind,
        LinkKind::Rod { length_m: 4.0 }
    );
    let graph = state(&runner).document.clone();
    click_world(&mut runner, right)?;
    click_world(&mut runner, left)?;
    assert_same_document(&state(&runner).document, &graph);
    assert!(
        state(&runner)
            .status
            .contains("already have a relationship")
    );
    frame(&mut runner, &key(PhysicalKeyCode::Delete))?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    assert!(state(&runner).document.links().is_empty());
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
    assert_eq!(state(&runner).document.objects(), graph.objects());
    assert_eq!(state(&runner).document.links(), graph.links());
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
    assert_eq!(state(&runner).document.objects().len(), 2);
    assert!(
        state(&runner).document.links().is_empty(),
        "rejected duplicate did not add history"
    );
    Ok(())
}

#[test]
fn spring_shortcut_creates_one_typed_link_with_bounded_visual_segments() -> LogicResult {
    let mut runner = runner()?;
    let left = Point::new(-2.0, 0.0);
    let right = Point::new(2.0, 0.0);
    click_world(&mut runner, left)?;
    click_world(&mut runner, right)?;
    frame(&mut runner, &key(PhysicalKeyCode::Digit5))?;
    assert_eq!(state(&runner).tool, Tool::Spring);
    click_world(&mut runner, left)?;
    click_world(&mut runner, right)?;
    assert_eq!(
        state(&runner).document.links()[0].kind,
        LinkKind::Spring {
            rest_length_m: 4.0,
            stiffness_n_m: 20.0
        }
    );
    let mut visible_edges = 0;
    for (entity, slot) in runner.components::<VisualSlot>() {
        if matches!(slot, VisualSlot::LinkEdge { index: 0, .. })
            && runner.component::<ScreenLineVisual>(entity)?.clip() != ScreenClip::Empty
        {
            visible_edges += 1;
        }
    }
    assert_eq!(visible_edges, 8);
    Ok(())
}

#[test]
fn unsupported_spring_frequency_rejects_run_without_discarding_scene() -> LogicResult {
    fn populate(mut state: ResMut<EditorState>) {
        let a = state
            .document
            .add(ObjectKind::Ball, Point::default())
            .unwrap();
        let b = state
            .document
            .add(ObjectKind::Ball, Point::new(1.0, 0.0))
            .unwrap();
        state.document.set_mass(a, 0.001).unwrap();
        state.document.set_mass(b, 0.001).unwrap();
        state.document.add_spring(a, b).unwrap();
    }
    let (mut app, initial) = crate::build_phys_editor_application()?;
    app.add_system(Stage::Startup, populate);
    let mut runner = app.build_headless(initial)?;
    let authored = state(&runner).document.clone();
    assert!(matches!(
        RunSession::new(&authored),
        Err(sim_physics::Error::SpringStepTooLarge(_))
    ));
    click(&mut runner, Control::Run)?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert!(state(&runner).run.is_none());
    assert!(
        state(&runner)
            .notice
            .as_deref()
            .unwrap()
            .contains("Cannot start physics")
    );
    assert_same_document(&state(&runner).document, &authored);
    Ok(())
}

#[test]
fn failed_physics_step_pauses_session_and_preserves_last_committed_world() {
    let mut document = Document::default();
    let a = document.add(ObjectKind::Anchor, Point::default()).unwrap();
    let b = document
        .add(ObjectKind::Ball, Point::new(1e-6, 0.0))
        .unwrap();
    document.add_rod(a, b).unwrap();
    document
        .set_environment(PhysicsEnvironment {
            gravity_m_s2: Point::new(0.0, -10_000.0),
            linear_drag_per_s: 0.0,
        })
        .unwrap();
    let mut session = RunSession::new(&document).unwrap();
    session.advance(0.0);
    let before = session.world().snapshot();
    session.advance(session.world().solver().fixed_dt_s);
    assert!(session.paused);
    assert!(session.failure.is_some());
    assert_eq!(session.world().snapshot(), before);
    session.pause(false);
    session.advance(10.0);
    assert_eq!(
        session.world().snapshot(),
        before,
        "a failed run cannot silently resume"
    );
}

#[test]
fn invalid_frame_duration_becomes_a_visible_atomic_session_failure() {
    for invalid in [f64::NAN, f64::INFINITY, -0.001] {
        let mut session = RunSession::new(&Document::default()).unwrap();
        session.advance(0.0);
        let before = session.world().snapshot();
        session.advance(invalid);
        assert!(session.paused);
        assert!(
            session
                .failure
                .as_deref()
                .unwrap()
                .contains("Invalid frame duration")
        );
        assert_eq!(session.world().snapshot(), before);
    }
}

#[test]
fn playback_focus_loss_pauses_until_explicit_resume_without_catching_up() -> LogicResult {
    let mut runner = runner()?;
    click_world(&mut runner, Point::default())?;
    click(&mut runner, Control::Run)?;
    frame(&mut runner, &[])?;
    let before = run(&runner).world().snapshot();
    frame(&mut runner, &[InputEvent::FocusLost])?;
    assert!(run(&runner).paused);
    for _ in 0..10 {
        frame(&mut runner, &[])?;
        assert_eq!(run(&runner).world().snapshot(), before);
    }
    click(&mut runner, Control::Pause)?;
    assert!(!run(&runner).paused);
    assert_eq!(run(&runner).world().snapshot(), before);
    frame(&mut runner, &[])?;
    assert!(run(&runner).world().step_index() > before.step_index);
    assert_eq!(run(&runner).dropped_simulation_s, 0.0);
    Ok(())
}
