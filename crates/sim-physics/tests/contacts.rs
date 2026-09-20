//! Collider contracts, bounded work, telemetry, and model-exclusion regressions.

use sim_physics::*;

mod support;
use support::*;

fn disk(number: u64, restitution: f64) -> ColliderDesc {
    ColliderDesc {
        body: id(number),
        shape: CollisionShape::Circle { radius_m: 0.5 },
        restitution,
    }
}

fn rectangle(number: u64, half: Vec2, angle: f64) -> ColliderDesc {
    ColliderDesc {
        body: id(number),
        shape: CollisionShape::Box {
            half_extents_m: half,
            angle_rad: angle,
        },
        restitution: 0.0,
    }
}

#[test]
fn collider_input_limits_precede_invalid_contents_and_constructor_canonicalizes() {
    let body = body(1, 0.0, 0.0, 1.0);
    let mut invalid = disk(99, f64::NAN);
    assert_eq!(
        World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &[body],
            &[],
            &vec![invalid; MAX_COLLIDERS + 1]
        )
        .unwrap_err(),
        Error::ColliderBudget
    );
    for shape in [
        CollisionShape::Circle { radius_m: 0.0 },
        CollisionShape::Circle {
            radius_m: f64::INFINITY,
        },
        CollisionShape::Box {
            half_extents_m: Vec2::new(1.0, -1.0),
            angle_rad: 0.0,
        },
        CollisionShape::Box {
            half_extents_m: Vec2::new(1.0, 1.0),
            angle_rad: f64::NAN,
        },
    ] {
        invalid = ColliderDesc {
            body: id(1),
            shape,
            restitution: 0.0,
        };
        assert_eq!(
            World::with_colliders(
                no_gravity(),
                SolverConfig::default(),
                &[body],
                &[],
                &[invalid]
            )
            .unwrap_err(),
            Error::InvalidCollider(id(1))
        );
    }
    assert_eq!(
        World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &[body],
            &[],
            &[disk(1, 0.0), disk(1, 0.0)]
        )
        .unwrap_err(),
        Error::DuplicateCollider(id(1))
    );
    assert_eq!(
        World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &[body],
            &[],
            &[disk(2, 0.0)]
        )
        .unwrap_err(),
        Error::MissingBody(id(2))
    );
    for restitution in [-0.01, 1.01, f64::NAN] {
        assert_eq!(
            World::with_colliders(
                no_gravity(),
                SolverConfig::default(),
                &[body],
                &[],
                &[disk(1, restitution)]
            )
            .unwrap_err(),
            Error::InvalidCollider(id(1))
        );
    }
}

#[test]
fn initial_dynamic_overlap_rejects_but_static_geometry_may_intersect() {
    let colliders = [disk(2, 0.0), disk(1, 0.0)];
    assert_eq!(
        World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &[body(2, 0.5, 0.0, 1.0), body(1, 0.0, 0.0, 1.0)],
            &[],
            &colliders
        )
        .unwrap_err(),
        Error::InitialOverlap(id(1), id(2))
    );
    let mut world = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &[fixed(2, 0.0, 0.0), fixed(1, 0.0, 0.0)],
        &[],
        &colliders,
    )
    .unwrap();
    assert_eq!(world.colliders(), &[disk(1, 0.0), disk(2, 0.0)]);
    assert_eq!(world.snapshot().colliders, world.colliders());
    assert!(world.step(&[]).unwrap().contacts.is_empty());
}

#[test]
fn rods_do_not_secretly_disable_endpoint_collisions() {
    let bodies = [body(1, 0.0, 0.0, 1.0), body(2, 0.5, 0.0, 1.0)];
    assert_eq!(
        World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &bodies,
            &[rod(1, 1, 2, 0.5)],
            &[disk(1, 0.0), disk(2, 0.0)]
        )
        .unwrap_err(),
        Error::InitialOverlap(id(1), id(2))
    );
}

fn repeated_floor(height: f64) -> (Vec<BodyDesc>, Vec<ColliderDesc>) {
    let mut bodies = vec![
        body(1, -3.0, height, 1.0),
        body(2, 0.0, height, 1.0),
        body(3, 3.0, height, 1.0),
    ];
    let mut colliders = vec![disk(1, 0.0), disk(2, 0.0), disk(3, 0.0)];
    // Intersecting fixed geometry is legal; each dynamic contact still consumes
    // its own bounded pair entry, regardless of identical supporting shapes.
    for number in 4..=MAX_BODIES as u64 {
        bodies.push(fixed(number, 0.0, -1.0));
        colliders.push(rectangle(number, Vec2::new(10.0, 0.5), 0.0));
    }
    (bodies, colliders)
}

#[test]
fn active_contact_budget_is_enforced_on_construction_and_before_step_commit() {
    let (bodies, colliders) = repeated_floor(0.0);
    assert_eq!(
        World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &bodies,
            &[],
            &colliders
        )
        .unwrap_err(),
        Error::ContactBudget
    );
    let (bodies, colliders) = repeated_floor(0.001);
    let mut world = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &bodies,
        &[],
        &colliders,
    )
    .unwrap();
    world.step(&[]).unwrap();
    let before = world.snapshot();
    let forces: Vec<_> = (1..=3)
        .map(|number| force(number, 1, 0.0, -1000.0))
        .collect();
    assert_eq!(world.step(&forces).unwrap_err(), Error::ContactBudget);
    assert_eq!(world.snapshot(), before);
}

#[test]
fn support_loads_and_position_cleanup_are_distinct_observations() {
    let mut world = World::with_colliders(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &[body(1, 0.0, 0.0, 2.0), fixed(2, 0.0, -1.0)],
        &[],
        &[disk(1, 1.0), rectangle(2, Vec2::new(3.0, 0.5), 0.0)],
    )
    .unwrap();
    for _ in 0..240 {
        let report = world.step(&[]).unwrap();
        let bob = &report.bodies[0];
        let floor = &report.bodies[1];
        close(bob.position_m.y, 0.0, 1e-12);
        close(bob.velocity_m_s.y, 0.0, 1e-12);
        close(bob.contact_force_n.y, 2.0 * 9.80665, 1e-10);
        close(bob.net_force_n.y, 0.0, 1e-10);
        assert_eq!(bob.contact_impulse_ns, -floor.contact_impulse_ns);
        assert!(bob.contact_position_correction_m.y > 0.0);
        assert!(report.energy.contact_projection_energy_change_j > 0.0);
        assert!(report.energy.contact_solve_kinetic_change_j < 0.0);
        assert_eq!(report.contacts[0].effective_restitution, 0.0);
        assert_eq!(report.energy.drag_dissipated_j, 0.0);
    }
}

#[test]
fn restitution_threshold_is_explicit_and_not_a_silent_material_edit() {
    for (speed, effective) in [
        (RESTITUTION_SPEED_THRESHOLD_M_S * 0.5, 0.0),
        (RESTITUTION_SPEED_THRESHOLD_M_S, 1.0),
    ] {
        let mut moving = body(1, 0.0, 0.0, 1.0);
        moving.velocity_m_s.x = speed;
        let mut world = World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &[moving, fixed(2, 1.0, 0.0)],
            &[],
            &[disk(1, 1.0), disk(2, 0.0)],
        )
        .unwrap();
        let report = world.step(&[]).unwrap();
        assert_eq!(report.contacts[0].restitution, 1.0);
        assert_eq!(report.contacts[0].effective_restitution, effective);
        close(report.bodies[0].velocity_m_s.x, -speed * effective, 1e-14);
    }
}

#[test]
fn locked_box_collision_conserves_linear_momentum_without_rotating_shapes() {
    let mut left = body(1, -0.5, 0.0, 1.0);
    let mut right = body(2, 0.5, 0.0, 2.0);
    left.velocity_m_s = Vec2::new(1.0, 0.3);
    right.velocity_m_s = Vec2::new(-1.0, 0.3);
    let mut colliders = [
        rectangle(1, Vec2::new(0.5, 0.7), 0.0),
        rectangle(2, Vec2::new(0.5, 0.5), 0.0),
    ];
    colliders[0].restitution = 1.0;
    let mut world = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &[left, right],
        &[],
        &colliders,
    )
    .unwrap();
    let report = world.step(&[]).unwrap();
    close(report.bodies[0].velocity_m_s.x, -5.0 / 3.0, 1e-12);
    close(report.bodies[1].velocity_m_s.x, 1.0 / 3.0, 1e-12);
    close(report.bodies[0].velocity_m_s.y, 0.3, 1e-12);
    close(report.bodies[1].velocity_m_s.y, 0.3, 1e-12);
    assert_eq!(world.colliders(), colliders);
    close(report.energy.contact_solve_kinetic_change_j, 0.0, 1e-12);
}

#[test]
fn isolated_collider_has_no_implicit_floor_or_shape_induced_damping() {
    let initial = [body(1, 0.0, 1.0, 1.0)];
    let mut point = World::new(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &initial,
        &[],
    )
    .unwrap();
    let mut finite = World::with_colliders(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &initial,
        &[],
        &[disk(1, 0.8)],
    )
    .unwrap();
    for _ in 0..480 {
        let old = point.step(&[]).unwrap();
        let new = finite.step(&[]).unwrap();
        assert_eq!(point.bodies(), finite.bodies());
        assert_eq!(old, new);
    }
    assert!(finite.bodies()[0].position_m.y < -10.0);
}

#[test]
fn contact_balance_with_drag_matches_actual_body_momentum_change() {
    let mut moving = body(1, 0.0, 0.0, 3.0);
    moving.velocity_m_s = Vec2::new(1.0, 0.2);
    let settings = PhysicsSettings {
        gravity_m_s2: Vec2::ZERO,
        linear_drag_per_s: 0.5,
    };
    let mut world = World::with_colliders(
        settings,
        SolverConfig::default(),
        &[moving, fixed(2, 1.0, 0.0)],
        &[],
        &[disk(1, 0.7), disk(2, 0.0)],
    )
    .unwrap();
    let report = world.step(&[]).unwrap();
    let current = &report.bodies[0];
    let dt = world.solver().fixed_dt_s;
    let momentum = (current.velocity_m_s - moving.velocity_m_s) * moving.mass_kg;
    close(current.net_force_n.x * dt, momentum.x, 1e-12);
    close(current.net_force_n.y * dt, momentum.y, 1e-12);
    assert!(report.energy.drag_dissipated_j > 0.0);
}
