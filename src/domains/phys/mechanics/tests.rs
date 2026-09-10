use crate::foundation::{
    EntityId, Kilograms, Meters, MetersPerSecond, Newtons, StepIndex, TimeStep,
};

use super::{
    ArithmeticOperation, BodySpec, Force2, ForceContribution, ForceSourceId, MAX_BODIES,
    MAX_EVENTS_PER_STEP, MAX_FORCE_CONTRIBUTIONS_PER_STEP, MechanicsError, MechanicsEventKind,
    MechanicsWorld, Mobility, Position2, Velocity2,
};

fn meters(value: f64) -> Meters {
    Meters::new(value).expect("test distance must be finite")
}

fn velocity(value: f64) -> MetersPerSecond {
    MetersPerSecond::new(value).expect("test velocity must be finite")
}

fn newtons(value: f64) -> Newtons {
    Newtons::new(value).expect("test force must be finite")
}

fn kilograms(value: f64) -> Kilograms {
    Kilograms::new(value).expect("test mass must be positive and finite")
}

fn dt(value: f64) -> TimeStep {
    TimeStep::new(value).expect("test step must be positive and finite")
}

fn force_source(raw: u64) -> ForceSourceId {
    ForceSourceId::new(raw).expect("test source ID must be non-zero")
}

fn dynamic_body(position: Position2, velocity: Velocity2, mass: f64) -> BodySpec {
    BodySpec::new(position, velocity, kilograms(mass), Mobility::Dynamic)
}

fn contribution(target: EntityId, source_id: u64, x: f64, y: f64) -> ForceContribution {
    ForceContribution::new(
        target,
        force_source(source_id),
        Force2::new(newtons(x), newtons(y)),
    )
}

#[test]
fn zero_force_preserves_velocity_and_advances_position() {
    let mut world = MechanicsWorld::new();
    let id = world
        .create_body(dynamic_body(
            Position2::new(meters(1.0), meters(2.0)),
            Velocity2::new(velocity(3.0), velocity(-4.0)),
            2.0,
        ))
        .expect("body creation should succeed");

    let report = world.step(dt(0.5), &[]).expect("step should succeed");
    let body = report
        .snapshot
        .bodies
        .iter()
        .find(|body| body.id == id)
        .expect("created body must be present");

    assert_eq!(body.velocity.x().get(), 3.0);
    assert_eq!(body.velocity.y().get(), -4.0);
    assert_eq!(body.position.x().get(), 2.5);
    assert_eq!(body.position.y().get(), 0.0);
    assert_eq!(body.acceleration, super::Acceleration2::ZERO);
    assert_eq!(body.net_force, Force2::ZERO);
}

#[test]
fn equal_force_on_double_mass_produces_half_acceleration() {
    let mut world = MechanicsWorld::new();
    let light = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 2.0))
        .expect("light body");
    let heavy = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 4.0))
        .expect("heavy body");
    let forces = [
        contribution(light, 1, 8.0, -4.0),
        contribution(heavy, 1, 8.0, -4.0),
    ];

    let snapshot = world
        .step(dt(0.5), &forces)
        .expect("step should succeed")
        .snapshot;
    let light = snapshot
        .bodies
        .iter()
        .find(|body| body.id == light)
        .expect("light snapshot");
    let heavy = snapshot
        .bodies
        .iter()
        .find(|body| body.id == heavy)
        .expect("heavy snapshot");

    assert_eq!(light.acceleration.x().get(), 4.0);
    assert_eq!(light.acceleration.y().get(), -2.0);
    assert_eq!(heavy.acceleration.x().get(), 2.0);
    assert_eq!(heavy.acceleration.y().get(), -1.0);
    assert_eq!(light.velocity.x().get(), 2.0);
    assert_eq!(heavy.velocity.x().get(), 1.0);
}

#[test]
fn contribution_slice_order_cannot_change_reduction_or_events() {
    fn run(order: &[usize]) -> super::StepReport {
        let mut world = MechanicsWorld::new();
        let target = world
            .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
            .expect("body");
        let canonical = [
            contribution(target, 1, 1.0e16, 0.0),
            contribution(target, 2, -1.0e16, 0.0),
            contribution(target, 3, 1.0, 0.0),
        ];
        let permuted: Vec<_> = order.iter().map(|&index| canonical[index]).collect();
        world.step(dt(1.0), &permuted).expect("step")
    }

    let first = run(&[0, 1, 2]);
    let second = run(&[2, 0, 1]);
    let third = run(&[1, 2, 0]);

    assert_eq!(first, second);
    assert_eq!(second, third);
    assert_eq!(first.snapshot.bodies[0].net_force.x().get(), 1.0);
}

#[test]
fn source_id_assignment_cannot_change_physical_sum_or_success() {
    fn run(assignments: &[(u64, f64)]) -> super::MechanicsSnapshot {
        let mut world = MechanicsWorld::new();
        let target = world
            .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
            .expect("body");
        let forces: Vec<_> = assignments
            .iter()
            .map(|&(source_id, value)| contribution(target, source_id, value, 0.0))
            .collect();
        world.step(dt(1.0), &forces).expect("step").snapshot
    }

    let first = run(&[(1, 1.0e16), (2, 1.0), (3, -1.0e16)]);
    let reassigned = run(&[(3, 1.0e16), (1, 1.0), (2, -1.0e16)]);
    assert_eq!(first, reassigned);
    assert_eq!(first.bodies[0].net_force.x().get(), 1.0);

    let maximum_first = run(&[(1, f64::MAX), (2, f64::MAX), (3, -f64::MAX)]);
    let maximum_reassigned = run(&[(3, f64::MAX), (1, f64::MAX), (2, -f64::MAX)]);
    assert_eq!(maximum_first, maximum_reassigned);
    assert_eq!(maximum_first.bodies[0].net_force.x().get(), f64::MAX);
}

#[test]
fn exact_maximum_cancellation_has_no_intermediate_overflow() {
    let mut world = MechanicsWorld::new();
    let target = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("body");
    let forces = [
        contribution(target, 1, f64::MAX, 0.0),
        contribution(target, 2, f64::MAX, 0.0),
        contribution(target, 3, -f64::MAX, 0.0),
        contribution(target, 4, -f64::MAX, 0.0),
    ];

    let snapshot = world
        .step(dt(1.0), &forces)
        .expect("exact cancellation")
        .snapshot;

    assert_eq!(snapshot.bodies[0].net_force, Force2::ZERO);
    assert_eq!(snapshot.bodies[0].acceleration, super::Acceleration2::ZERO);
}

#[test]
fn contribution_budget_is_checked_before_processing_and_is_atomic() {
    let mut world = MechanicsWorld::new();
    let target = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("body");
    let before = world.snapshot();
    let forces: Vec<_> = (1..=(MAX_FORCE_CONTRIBUTIONS_PER_STEP + 1))
        .map(|index| contribution(target, index as u64, 0.0, 0.0))
        .collect();

    assert_eq!(
        world.step(dt(1.0), &forces),
        Err(MechanicsError::ContributionBudgetExceeded {
            submitted: MAX_FORCE_CONTRIBUTIONS_PER_STEP + 1,
            maximum: MAX_FORCE_CONTRIBUTIONS_PER_STEP,
        })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn maximum_contribution_input_produces_a_bounded_event_stream() {
    let mut world = MechanicsWorld::new();
    let target = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("body");
    let forces: Vec<_> = (1..=MAX_FORCE_CONTRIBUTIONS_PER_STEP)
        .map(|index| contribution(target, index as u64, 0.0, 0.0))
        .collect();

    let report = world.step(dt(1.0), &forces).expect("bounded step");
    assert_eq!(report.events.len(), MAX_FORCE_CONTRIBUTIONS_PER_STEP + 2);
    assert!(report.events.len() <= MAX_EVENTS_PER_STEP);
}

#[test]
fn duplicate_force_source_is_canonical_and_atomic() {
    let mut world = MechanicsWorld::new();
    let first = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("first body");
    let second = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("second body");
    world
        .step(dt(0.25), &[contribution(first, 9, 2.0, 0.0)])
        .expect("baseline step");
    let before = world.snapshot();

    let forces = [
        contribution(second, 4, 1.0, 0.0),
        contribution(second, 4, 2.0, 0.0),
        contribution(first, 2, 1.0, 0.0),
        contribution(first, 2, 2.0, 0.0),
    ];
    assert_eq!(
        world.step(dt(0.25), &forces),
        Err(MechanicsError::DuplicateForceSource {
            target: first,
            source: force_source(2),
        })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn missing_target_rejects_the_complete_step() {
    let mut world = MechanicsWorld::new();
    let existing = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("body");
    let missing = EntityId::new(existing.get() + 10).expect("non-zero missing ID");
    let before = world.snapshot();

    assert_eq!(
        world.step(dt(1.0), &[contribution(missing, 1, 1.0, 0.0)]),
        Err(MechanicsError::MissingForceTarget { target: missing })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn fixed_target_rejects_force_and_does_not_move() {
    let mut world = MechanicsWorld::new();
    let fixed = world
        .create_body(BodySpec::new(
            Position2::new(meters(5.0), meters(-3.0)),
            Velocity2::ZERO,
            kilograms(1.0),
            Mobility::Fixed,
        ))
        .expect("fixed body");
    let before = world.snapshot();

    assert_eq!(
        world.step(dt(1.0), &[contribution(fixed, 1, 4.0, 0.0)]),
        Err(MechanicsError::ForceTargetIsFixed { target: fixed })
    );
    assert_eq!(world.snapshot(), before);

    let committed = world.step(dt(1.0), &[]).expect("empty fixed step");
    assert_eq!(committed.snapshot.bodies[0].position.x().get(), 5.0);
    assert_eq!(committed.snapshot.bodies[0].velocity.x().get(), 0.0);
    assert_eq!(committed.snapshot.bodies[0].net_force, Force2::ZERO);
}

#[test]
fn fixed_body_with_velocity_is_rejected_before_allocation() {
    let mut world = MechanicsWorld::new();
    let before = world.snapshot();
    let invalid = BodySpec::new(
        Position2::ZERO,
        Velocity2::new(velocity(1.0), velocity(0.0)),
        kilograms(1.0),
        Mobility::Fixed,
    );

    assert_eq!(
        world.create_body(invalid),
        Err(MechanicsError::FixedBodyHasVelocity)
    );
    assert_eq!(world.snapshot(), before);
    let first = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("valid body after rejection");
    assert_eq!(first.get(), 1);
}

#[test]
fn intermediate_overflow_rejects_step_atomically() {
    let mut world = MechanicsWorld::new();
    world
        .create_body(dynamic_body(
            Position2::ZERO,
            Velocity2::new(velocity(f64::MAX), velocity(0.0)),
            1.0,
        ))
        .expect("body");
    let before = world.snapshot();

    assert_eq!(
        world.step(dt(2.0), &[]),
        Err(MechanicsError::NonFiniteArithmetic {
            operation: ArithmeticOperation::PositionDelta,
            entity: Some(before.bodies[0].id),
        })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn elapsed_time_increment_absorbed_by_f64_rejects_step_atomically() {
    let mut world = MechanicsWorld::new();
    world
        .step(dt(2_f64.powi(53)), &[])
        .expect("large first step");
    let before = world.snapshot();

    assert_eq!(
        world.step(dt(1.0), &[]),
        Err(MechanicsError::PrecisionLoss {
            operation: ArithmeticOperation::ElapsedTime,
            entity: None,
        })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn position_increment_absorbed_by_f64_rejects_step_atomically() {
    let mut world = MechanicsWorld::new();
    let entity = world
        .create_body(dynamic_body(
            Position2::new(meters(2_f64.powi(53)), meters(0.0)),
            Velocity2::new(velocity(1.0), velocity(0.0)),
            1.0,
        ))
        .expect("body");
    let before = world.snapshot();

    assert_eq!(
        world.step(dt(1.0), &[]),
        Err(MechanicsError::PrecisionLoss {
            operation: ArithmeticOperation::Position,
            entity: Some(entity),
        })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn velocity_increment_absorbed_by_f64_rejects_step_atomically() {
    let mut world = MechanicsWorld::new();
    let entity = world
        .create_body(dynamic_body(
            Position2::ZERO,
            Velocity2::new(velocity(2_f64.powi(53)), velocity(0.0)),
            1.0,
        ))
        .expect("body");
    let before = world.snapshot();

    assert_eq!(
        world.step(dt(1.0), &[contribution(entity, 1, 1.0, 0.0)]),
        Err(MechanicsError::PrecisionLoss {
            operation: ArithmeticOperation::Velocity,
            entity: Some(entity),
        })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn force_reduction_overflow_rejects_step_atomically() {
    let mut world = MechanicsWorld::new();
    let target = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("body");
    let before = world.snapshot();
    let forces = [
        contribution(target, 1, f64::MAX, 0.0),
        contribution(target, 2, f64::MAX, 0.0),
    ];

    assert_eq!(
        world.step(dt(1.0), &forces),
        Err(MechanicsError::NonFiniteArithmetic {
            operation: ArithmeticOperation::ForceReduction,
            entity: Some(target),
        })
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn snapshots_are_sorted_bounded_and_non_mutating() {
    let mut world = MechanicsWorld::new();
    for _ in 0..MAX_BODIES {
        world
            .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
            .expect("body within bound");
    }
    let first = world.snapshot();
    let second = world.snapshot();

    assert_eq!(first, second);
    assert_eq!(first.bodies.len(), MAX_BODIES);
    assert!(first.bodies.windows(2).all(|pair| pair[0].id < pair[1].id));
    assert_eq!(first.step, StepIndex::ZERO);
    assert_eq!(
        world.create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0)),
        Err(MechanicsError::EntityCapacityReached {
            capacity: MAX_BODIES,
        })
    );
    assert_eq!(world.snapshot(), first);
}

#[test]
fn events_are_post_commit_and_canonically_ordered() {
    let mut world = MechanicsWorld::new();
    let first = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 1.0))
        .expect("first body");
    let second = world
        .create_body(dynamic_body(Position2::ZERO, Velocity2::ZERO, 2.0))
        .expect("second body");
    let forces = [
        contribution(second, 8, 4.0, 0.0),
        contribution(first, 9, 3.0, 0.0),
        contribution(first, 2, 1.0, 0.0),
    ];

    let report = world.step(dt(0.5), &forces).expect("step");
    assert_eq!(report.snapshot, world.snapshot());
    assert_eq!(report.events.len(), 6);
    assert!(matches!(
        report.events[0].kind,
        MechanicsEventKind::ForceAccepted { target, source, .. }
            if target == first && source == force_source(2)
    ));
    assert!(matches!(
        report.events[1].kind,
        MechanicsEventKind::ForceAccepted { target, source, .. }
            if target == first && source == force_source(9)
    ));
    assert!(matches!(
        report.events[2].kind,
        MechanicsEventKind::ForceAccepted { target, .. } if target == second
    ));
    assert!(matches!(
        report.events[3].kind,
        MechanicsEventKind::BodyIntegrated { entity, .. } if entity == first
    ));
    assert!(matches!(
        report.events[4].kind,
        MechanicsEventKind::BodyIntegrated { entity, .. } if entity == second
    ));
    assert!(matches!(
        report.events[5].kind,
        MechanicsEventKind::StepCommitted {
            dynamic_bodies: 2,
            accepted_forces: 3,
        }
    ));
    assert!(report.events.iter().all(|event| {
        event.step == report.snapshot.step
            && event.revision == report.snapshot.revision
            && event.elapsed == report.snapshot.elapsed
    }));
}

#[test]
fn identical_worlds_and_inputs_produce_identical_reports() {
    fn run() -> super::StepReport {
        let mut world = MechanicsWorld::new();
        let body = world
            .create_body(dynamic_body(
                Position2::new(meters(-2.0), meters(3.0)),
                Velocity2::new(velocity(0.25), velocity(-0.5)),
                3.0,
            ))
            .expect("body");
        world
            .step(
                dt(0.125),
                &[
                    contribution(body, 20, -9.0, 2.0),
                    contribution(body, 4, 1.5, -8.0),
                ],
            )
            .expect("step")
    }

    assert_eq!(run(), run());
}

#[test]
fn zero_force_source_identity_is_rejected() {
    assert!(ForceSourceId::new(0).is_err());
}
