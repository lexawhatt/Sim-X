//! End-to-end attachment conversion, authoring isolation and retained markers.

use super::{
    attachment::Attachment,
    document::{Document, ObjectKind, PhysicsEnvironment, Point},
    session::RunSession,
    state::EditorState,
    view::{self, VisualSlot},
};
use sim_logic::prelude::*;
use std::time::Duration;

fn scene() -> (Document, u64, u64) {
    let mut doc = Document::default();
    doc.set_environment(PhysicsEnvironment {
        gravity_m_s2: Point::default(),
        linear_drag_per_s: 0.0,
    })
    .unwrap();
    let anchor = doc.add(ObjectKind::Anchor, Point::new(-4.0, 2.0)).unwrap();
    let body = doc.add(ObjectKind::Box, Point::new(1.0, 0.0)).unwrap();
    doc.set_size(body, 2.0).unwrap();
    doc.add_spring_at(
        Attachment::center(anchor),
        Attachment {
            body,
            local_m: Point::new(-1.0, 0.5),
        },
    )
    .unwrap();
    (doc, anchor, body)
}

#[test]
fn adapter_passes_local_endpoints_rest_length_and_uniform_inertia() {
    let (doc, anchor, body) = scene();
    let session = RunSession::new(&doc).unwrap();
    let [
        sim_physics::LinkDesc::AttachedSpring {
            a,
            b,
            local_a_m,
            local_b_m,
            rest_length_m,
            ..
        },
    ] = session.world().links()
    else {
        panic!("adapter discarded attached-spring capability");
    };
    assert_eq!(a.get(), anchor);
    assert_eq!(b.get(), body);
    assert_eq!(*local_a_m, sim_physics::Vec2::ZERO);
    assert_eq!(*local_b_m, sim_physics::Vec2::new(-1.0, 0.5));
    assert!((*rest_length_m - 4.0_f64.hypot(1.5)).abs() < 1e-12);
    let rotation = session.world().rotation(*b).unwrap();
    assert!((rotation.inertia_kg_m2 - 5.0 / 12.0).abs() < 1e-12);
    assert!(
        session.world().rotation(*a).is_none(),
        "point markers have no rotational shape"
    );
}

#[test]
fn stretching_corner_spring_rotates_only_the_independent_run() {
    let (mut doc, anchor, body) = scene();
    doc.move_object(anchor, Point::new(-5.0, 2.0)).unwrap();
    let before = doc.clone();
    let mut session = RunSession::new(&doc).unwrap();
    session.advance(0.0);
    for _ in 0..120 {
        session.advance(session.world().solver().fixed_dt_s);
        assert!(session.failure.is_none(), "{:?}", session.failure);
    }
    assert!(session.angle_deg(body).unwrap().abs() > 0.01);
    assert_ne!(session.position(body), Some(Point::new(1.0, 0.0)));
    assert_eq!(doc.objects(), before.objects());
    assert_eq!(doc.links(), before.links());
    let restarted = RunSession::new(&doc).unwrap();
    assert_eq!(restarted.angle_deg(body), Some(0.0));
    assert_eq!(restarted.position(body), Some(Point::new(1.0, 0.0)));
}

#[test]
fn physical_resize_changes_inertia_and_corner_in_next_run_not_current_run() {
    let (mut doc, _, body) = scene();
    let old = RunSession::new(&doc).unwrap();
    let snapshot = old.world().snapshot();
    doc.set_size(body, 4.0).unwrap();
    doc.set_height(body, 2.0).unwrap();
    let new = RunSession::new(&doc).unwrap();
    let id = sim_physics::BodyId::new(body).unwrap();
    assert!((new.world().rotation(id).unwrap().inertia_kg_m2 - 5.0 / 3.0).abs() < 1e-12);
    assert_eq!(doc.links()[0].b_local_m, Point::new(-2.0, 1.0));
    assert_eq!(old.world().snapshot(), snapshot);
    assert!(doc.undo());
    assert!(doc.undo());
    assert_eq!(doc.links()[0].b_local_m, Point::new(-1.0, 0.5));
    assert_eq!(RunSession::new(&doc).unwrap().world().snapshot(), snapshot);
}

#[test]
fn unstrained_rotated_corner_does_not_invent_a_roundoff_force_on_run() {
    for angle in [15.0, 37.0, 90.0, 139.0, 345.0] {
        let (mut doc, anchor, body) = scene();
        // Replace the original relationship so the final rotated geometry
        // itself defines zero strain, rather than intentionally stretching it.
        doc.remove(anchor).unwrap();
        doc.move_object(body, Point::new(4.0, -2.0)).unwrap();
        doc.set_size(body, 4.0).unwrap();
        doc.set_height(body, 2.0).unwrap();
        doc.rotate(body, angle).unwrap();
        let anchor = doc.add(ObjectKind::Anchor, Point::new(-4.0, 3.0)).unwrap();
        doc.add_spring_at(
            Attachment::center(anchor),
            Attachment {
                body,
                local_m: Point::new(2.0, 1.0),
            },
        )
        .unwrap();
        let mut session = RunSession::new(&doc).unwrap();
        let before = session.world().snapshot();
        session.advance(0.0);
        for _ in 0..10 {
            session.advance(session.world().solver().fixed_dt_s);
            assert!(
                session.failure.is_none(),
                "angle {angle}: {:?}",
                session.failure
            );
        }
        assert_eq!(session.world().bodies(), before.bodies);
        assert_eq!(session.world().rotations(), before.rotations);
    }
}

#[test]
fn marker_entities_are_retained_and_layered_above_bodies_below_panels() -> LogicResult {
    let (mut app, initial) = crate::build_phys_editor_application()?;
    app.add_frame_system(|mut state: ResMut<EditorState>| {
        if state.document.objects().is_empty() {
            state.document = scene().0;
        }
    });
    let mut runner = app.build_headless(initial)?;
    for _ in 0..3 {
        let request = FrameRequest::new(
            Duration::from_millis(16),
            &[],
            LogicalViewport::new(1280.0, 800.0)?,
        );
        let FrameOutcome::Advanced(report) = runner.advance_frame(request) else {
            panic!("frame rejected");
        };
        assert!(report.failure().is_none(), "{:?}", report.failure());
        assert_eq!(report.spawned(), 0);
        assert_eq!(report.despawned(), 0);
        assert_eq!(report.fixed_ticks_attempted(), 0);
    }
    let mut anchors = 0;
    let mut visible = 0;
    for (entity, slot) in runner.components::<VisualSlot>() {
        if let VisualSlot::LinkAnchor { .. } = slot {
            anchors += 1;
            let circle = runner.component::<ScreenCircleVisual>(entity)?;
            if circle.clip() != ScreenClip::Empty {
                visible += 1;
                assert!(circle.draw_order_depth() > 2.0 && circle.draw_order_depth() < 3.0);
            }
        }
    }
    assert_eq!(anchors, super::document::MAX_LINKS * 2);
    assert_eq!(visible, 2);
    assert!(view::CIRCLE_COUNT >= anchors + super::document::MAX_OBJECTS);
    Ok(())
}
