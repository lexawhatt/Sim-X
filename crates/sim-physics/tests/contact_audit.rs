//! Independent public-API contact regressions and analytic counterexamples.
//!
//! Expected impacts come from linear momentum and Newton restitution, not the
//! implementation's internal impulse values. No angular/friction claim is made.

use sim_physics::{
    BodyDesc, BodyId, ColliderDesc, CollisionShape, LinkDesc, LinkId, Mobility, PhysicsSettings,
    RESTITUTION_SPEED_THRESHOLD_M_S, SolverConfig, Vec2, World,
};

fn body_id(value: u64) -> BodyId {
    BodyId::new(value).unwrap()
}

fn particle(value: u64, position: Vec2, velocity: Vec2, mass: f64) -> BodyDesc {
    BodyDesc {
        id: body_id(value),
        position_m: position,
        velocity_m_s: velocity,
        mass_kg: mass,
        mobility: Mobility::Dynamic,
    }
}

fn fixed(value: u64, position: Vec2) -> BodyDesc {
    BodyDesc {
        mobility: Mobility::Fixed,
        ..particle(value, position, Vec2::ZERO, 1.0)
    }
}

fn circle(value: u64, radius: f64, restitution: f64) -> ColliderDesc {
    ColliderDesc {
        body: body_id(value),
        shape: CollisionShape::Circle { radius_m: radius },
        restitution,
    }
}

fn rectangle(value: u64, half_extents: Vec2, angle: f64, restitution: f64) -> ColliderDesc {
    ColliderDesc {
        body: body_id(value),
        shape: CollisionShape::Box {
            half_extents_m: half_extents,
            angle_rad: angle,
        },
        restitution,
    }
}

fn no_gravity() -> PhysicsSettings {
    PhysicsSettings {
        gravity_m_s2: Vec2::ZERO,
        linear_drag_per_s: 0.0,
    }
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:e}, expected={expected:e}, tolerance={tolerance:e}"
    );
}

fn kinetic(bodies: &[BodyDesc]) -> f64 {
    bodies
        .iter()
        .filter(|body| body.mobility == Mobility::Dynamic)
        .map(|body| 0.5 * body.mass_kg * body.velocity_m_s.dot(body.velocity_m_s))
        .sum()
}

#[test]
fn initially_touching_unequal_masses_follow_momentum_and_restitution() {
    let bodies = [
        particle(1, Vec2::new(-0.5, 0.0), Vec2::new(3.0, 0.0), 2.0),
        particle(2, Vec2::new(0.5, 0.0), Vec2::new(-1.0, 0.0), 3.0),
    ];
    let mut world = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &bodies,
        &[],
        &[circle(1, 0.5, 0.2), circle(2, 0.5, 0.5)],
    )
    .unwrap();
    world.step(&[]).unwrap();
    let a = world.body(body_id(1)).unwrap();
    let b = world.body(body_id(2)).unwrap();
    // Centre velocity is 0.6 m/s; relative rebound speed is 0.5 * 4 m/s.
    near(a.velocity_m_s.x, -0.6, 1e-9);
    near(b.velocity_m_s.x, 1.4, 1e-9);
    near(2.0 * a.velocity_m_s.x + 3.0 * b.velocity_m_s.x, 3.0, 1e-9);
    near(kinetic(world.bodies()), 3.3, 1e-9);
    near(kinetic(&bodies) - kinetic(world.bodies()), 7.2, 1e-9);
    assert!((b.position_m - a.position_m).length() >= 1.0 - 1e-8);
}

#[test]
fn a_rotated_fixed_box_reflects_normal_velocity_without_tangential_friction() {
    let angle = std::f64::consts::PI / 6.0;
    let tangent = Vec2::new(angle.cos(), angle.sin());
    let normal = Vec2::new(-angle.sin(), angle.cos());
    let bodies = [
        fixed(1, Vec2::ZERO),
        particle(2, normal, tangent * 2.0 - normal * 3.0, 2.0),
    ];
    let mut world = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &bodies,
        &[],
        &[
            rectangle(1, Vec2::new(5.0, 0.5), angle, 0.0),
            circle(2, 0.5, 1.0),
        ],
    )
    .unwrap();
    world.step(&[]).unwrap();
    let ball = world.body(body_id(2)).unwrap();
    near(ball.velocity_m_s.dot(normal), 3.0, 1e-9);
    near(ball.velocity_m_s.dot(tangent), 2.0, 1e-9);
    near(kinetic(world.bodies()), 13.0, 1e-9);
    assert_eq!(world.body(body_id(1)).unwrap(), &bodies[0]);
}

#[test]
fn a_resting_elastic_ball_does_not_turn_gravity_into_repeated_bounces() {
    let mut world = World::with_colliders(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &[
            fixed(1, Vec2::ZERO),
            particle(2, Vec2::new(0.0, 1.0), Vec2::ZERO, 2.0),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(5.0, 0.5), 0.0, 0.0),
            circle(2, 0.5, 1.0),
        ],
    )
    .unwrap();
    for _ in 0..1200 {
        world.step(&[]).unwrap();
        let ball = world.body(body_id(2)).unwrap();
        near(ball.position_m.y, 1.0, 1e-7);
        near(ball.velocity_m_s.y, 0.0, 1e-7);
    }
}

#[test]
fn small_frictionless_stack_supports_weight_without_residual_penetration() {
    let solver = SolverConfig {
        constraint_iterations: 128,
        ..SolverConfig::default()
    };
    let mut world = World::with_colliders(
        PhysicsSettings::default(),
        solver,
        &[
            fixed(1, Vec2::ZERO),
            particle(2, Vec2::new(0.0, 1.0), Vec2::ZERO, 1.0),
            particle(3, Vec2::new(0.0, 2.0), Vec2::ZERO, 1.0),
            particle(4, Vec2::new(0.0, 3.0), Vec2::ZERO, 1.0),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(5.0, 0.5), 0.0, 0.0),
            circle(2, 0.5, 0.0),
            circle(3, 0.5, 0.0),
            circle(4, 0.5, 0.0),
        ],
    )
    .unwrap();
    for _ in 0..240 {
        world.step(&[]).unwrap();
        let mut previous_y = 0.0;
        for value in 2..=4 {
            let body = world.body(body_id(value)).unwrap();
            assert!(body.position_m.y - previous_y >= 1.0 - 1e-7);
            near(body.velocity_m_s.y, 0.0, 1e-7);
            previous_y = body.position_m.y;
        }
    }
}

#[test]
fn pendulum_wall_contact_preserves_rod_length_and_tangency_together() {
    let solver = SolverConfig {
        constraint_iterations: 128,
        ..SolverConfig::default()
    };
    let bodies = [
        fixed(1, Vec2::ZERO),
        particle(2, Vec2::new(0.0, -2.0), Vec2::new(1.0, 0.0), 1.0),
        fixed(3, Vec2::new(0.6, -2.0)),
    ];
    let links = [LinkDesc::Rod {
        id: LinkId::new(1).unwrap(),
        a: body_id(1),
        b: body_id(2),
        length_m: 2.0,
    }];
    let mut world = World::with_colliders(
        PhysicsSettings::default(),
        solver,
        &bodies,
        &links,
        &[
            circle(2, 0.5, 1.0),
            rectangle(3, Vec2::new(0.1, 3.0), 0.0, 0.0),
        ],
    )
    .unwrap();
    for _ in 0..240 {
        world.step(&[]).unwrap();
        let bob = world.body(body_id(2)).unwrap();
        near(bob.position_m.length(), 2.0, 2e-9);
        near(bob.position_m.dot(bob.velocity_m_s) / 2.0, 0.0, 2e-9);
        assert!(bob.position_m.x <= 1e-8, "bob penetrated the wall");
    }
}

fn assert_step_rejected_atomically(world: &mut World) {
    let before = world.snapshot();
    assert!(
        world.step(&[]).is_err(),
        "unsupported swept collision was silently accepted"
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn glancing_circle_sweep_cannot_cross_a_thin_overlap_lens_unnoticed() {
    let solver = SolverConfig::default();
    let velocity = Vec2::new(0.04 / solver.fixed_dt_s, 0.0);
    let mut world = World::with_colliders(
        no_gravity(),
        solver,
        &[
            fixed(1, Vec2::ZERO),
            particle(2, Vec2::new(-0.02, 0.9999), velocity, 1.0),
        ],
        &[],
        &[circle(1, 0.5, 0.0), circle(2, 0.5, 0.0)],
    )
    .unwrap();
    // Both endpoints are separated, travel is only 0.04m, but closest approach
    // is 0.9999m < sum radii. A feature-size movement cap alone misses this.
    assert_step_rejected_atomically(&mut world);
}

#[test]
fn swept_locked_boxes_cannot_clip_through_a_corner_between_separated_endpoints() {
    let solver = SolverConfig::default();
    let angle = 0.3_f64;
    let rotate = |point: Vec2| {
        Vec2::new(
            point.x * angle.cos() - point.y * angle.sin(),
            point.x * angle.sin() + point.y * angle.cos(),
        )
    };
    let start = rotate(Vec2::new(-1.02, 0.9798));
    let displacement = rotate(Vec2::new(0.04, 0.04));
    let mut world = World::with_colliders(
        no_gravity(),
        solver,
        &[
            fixed(1, Vec2::ZERO),
            particle(2, start, displacement * (1.0 / solver.fixed_dt_s), 1.0),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(0.5, 0.5), angle, 0.0),
            rectangle(2, Vec2::new(0.5, 0.5), angle, 0.0),
        ],
    )
    .unwrap();
    // In shared local axes, X separation ends at t=.5 but Y separation only
    // starts at t=.505; the overlapping interval is small and strictly real.
    assert_step_rejected_atomically(&mut world);
}

fn rounded_corner_sweep(clearance: f64) -> World {
    let solver = SolverConfig::default();
    let component = std::f64::consts::FRAC_1_SQRT_2;
    let radial = Vec2::new(component, component);
    let tangent = Vec2::new(-component, component);
    let middle = Vec2::new(1.0, 1.0) + radial * (0.5 + clearance);
    World::with_colliders(
        no_gravity(),
        solver,
        &[
            fixed(1, Vec2::ZERO),
            particle(
                2,
                middle - tangent * 0.02,
                tangent * (0.04 / solver.fixed_dt_s),
                1.0,
            ),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(1.0, 1.0), 0.0, 0.0),
            circle(2, 0.5, 0.0),
        ],
    )
    .unwrap()
}

#[test]
fn swept_circle_box_corner_detects_real_penetration_but_allows_a_near_miss() {
    assert_step_rejected_atomically(&mut rounded_corner_sweep(-0.0001));
    let mut near_miss = rounded_corner_sweep(0.0001);
    let velocity = near_miss.body(body_id(2)).unwrap().velocity_m_s;
    near_miss.step(&[]).unwrap();
    assert_eq!(near_miss.body(body_id(2)).unwrap().velocity_m_s, velocity);
}

#[test]
fn fast_head_on_pass_through_is_rejected_without_committing_telemetry() {
    let solver = SolverConfig::default();
    let mut world = World::with_colliders(
        no_gravity(),
        solver,
        &[
            fixed(1, Vec2::ZERO),
            particle(
                2,
                Vec2::new(-2.0, 0.0),
                Vec2::new(4.0 / solver.fixed_dt_s, 0.0),
                1.0,
            ),
        ],
        &[],
        &[circle(1, 0.5, 0.0), circle(2, 0.5, 0.0)],
    )
    .unwrap();
    assert_step_rejected_atomically(&mut world);
}

#[test]
fn separated_colliders_do_not_change_the_existing_rattle_trajectory() {
    let angle = 0.3_f64;
    let bodies = [
        fixed(1, Vec2::ZERO),
        particle(
            2,
            Vec2::new(2.0 * angle.sin(), -2.0 * angle.cos()),
            Vec2::ZERO,
            1.0,
        ),
    ];
    let links = [LinkDesc::Rod {
        id: LinkId::new(1).unwrap(),
        a: body_id(1),
        b: body_id(2),
        length_m: 2.0,
    }];
    let mut reference = World::new(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &bodies,
        &links,
    )
    .unwrap();
    let mut with_shapes = World::with_colliders(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &bodies,
        &links,
        &[circle(1, 0.05, 0.0), circle(2, 0.05, 0.0)],
    )
    .unwrap();
    for _ in 0..480 {
        reference.step(&[]).unwrap();
        with_shapes.step(&[]).unwrap();
        assert_eq!(with_shapes.bodies(), reference.bodies());
        assert_eq!(with_shapes.elapsed_s(), reference.elapsed_s());
        assert_eq!(
            with_shapes.last_report().unwrap().energy,
            reference.last_report().unwrap().energy
        );
    }
}

#[test]
fn caller_storage_order_does_not_change_contact_results() {
    let bodies = [
        particle(1, Vec2::new(-0.5, 0.0), Vec2::new(3.0, 0.0), 2.0),
        particle(2, Vec2::new(0.5, 0.0), Vec2::new(-1.0, 0.0), 3.0),
    ];
    let colliders = [circle(1, 0.5, 0.2), circle(2, 0.5, 0.5)];
    let mut a = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &bodies,
        &[],
        &colliders,
    )
    .unwrap();
    let mut b = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &[bodies[1], bodies[0]],
        &[],
        &[colliders[1], colliders[0]],
    )
    .unwrap();
    for _ in 0..20 {
        a.step(&[]).unwrap();
        b.step(&[]).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
    }
}

#[test]
fn a_symmetric_simultaneous_impact_does_not_create_kinetic_energy() {
    let bodies = [
        particle(1, Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), 1.0),
        particle(2, Vec2::ZERO, Vec2::ZERO, 1.0),
        particle(3, Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0), 1.0),
    ];
    let solver = SolverConfig {
        constraint_iterations: 128,
        ..SolverConfig::default()
    };
    let mut world = World::with_colliders(
        no_gravity(),
        solver,
        &bodies,
        &[],
        &[
            circle(1, 0.5, 1.0),
            circle(2, 0.5, 1.0),
            circle(3, 0.5, 1.0),
        ],
    )
    .unwrap();
    world.step(&[]).unwrap();
    near(world.body(body_id(1)).unwrap().velocity_m_s.x, -1.0, 1e-8);
    near(world.body(body_id(2)).unwrap().velocity_m_s.x, 0.0, 1e-8);
    near(world.body(body_id(3)).unwrap().velocity_m_s.x, 1.0, 1e-8);
    assert!(kinetic(world.bodies()) <= kinetic(&bodies) + 1e-8);
}

#[test]
fn contact_motion_limits_do_not_restrict_fast_free_flight_without_a_nearby_pair() {
    let initial = particle(1, Vec2::ZERO, Vec2::new(1000.0, 0.0), 1.0);
    let mut world = World::with_colliders(
        no_gravity(),
        SolverConfig::default(),
        &[initial, fixed(2, Vec2::new(1000.0, 1000.0))],
        &[],
        &[circle(1, 0.1, 0.0), circle(2, 0.1, 0.0)],
    )
    .unwrap();
    for _ in 0..10 {
        world.step(&[]).unwrap();
    }
    let moving = world.body(body_id(1)).unwrap();
    near(moving.position_m.x, 1000.0 * world.elapsed_s(), 1e-12);
    assert_eq!(moving.velocity_m_s, initial.velocity_m_s);
}

#[test]
fn an_unrelated_high_energy_body_cannot_erase_contact_kinetic_change() {
    let bodies = [
        particle(1, Vec2::new(-0.5, 0.0), Vec2::new(3.0, 0.0), 2.0),
        particle(2, Vec2::new(0.5, 0.0), Vec2::new(-1.0, 0.0), 3.0),
        particle(3, Vec2::new(1e6, 1e6), Vec2::new(1e5, 0.0), 1e12),
    ];
    let colliders = [circle(1, 0.5, 0.2), circle(2, 0.5, 0.5)];
    for count in [2, 3] {
        let mut world = World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &bodies[..count],
            &[],
            &colliders,
        )
        .unwrap();
        let report = world.step(&[]).unwrap();
        // The spectator receives no contact impulse. Its unchanged ~5e21 J
        // cannot turn the independently representable 7.2 J loss into zero.
        near(report.energy.contact_solve_kinetic_change_j, -7.2, 1e-9);
    }
}

#[test]
fn an_unrelated_potential_offset_cannot_erase_projection_energy_accounting() {
    let settings = PhysicsSettings::default();
    let bodies = [
        fixed(1, Vec2::ZERO),
        particle(2, Vec2::new(0.0, 1.0), Vec2::ZERO, 1.0),
        particle(3, Vec2::new(1e6, 1e8), Vec2::ZERO, 1e12),
    ];
    let colliders = [
        rectangle(1, Vec2::new(5.0, 0.5), 0.0, 0.0),
        circle(2, 0.5, 0.0),
    ];
    for count in [2, 3] {
        let mut world = World::with_colliders(
            settings,
            SolverConfig::default(),
            &bodies[..count],
            &[],
            &colliders,
        )
        .unwrap();
        let report = world.step(&[]).unwrap();
        let ball = report
            .bodies
            .iter()
            .find(|body| body.id == body_id(2))
            .unwrap();
        let expected = -settings
            .gravity_m_s2
            .dot(ball.contact_position_correction_m);
        assert!(expected > 1e-5);
        near(
            report.energy.contact_projection_energy_change_j,
            expected,
            1e-12,
        );
    }
}

#[test]
fn restitution_settling_threshold_is_explicit_at_its_boundary() {
    for (speed, effective) in [
        (RESTITUTION_SPEED_THRESHOLD_M_S - 1e-6, 0.0),
        (RESTITUTION_SPEED_THRESHOLD_M_S, 1.0),
        (RESTITUTION_SPEED_THRESHOLD_M_S + 1e-6, 1.0),
    ] {
        let mut world = World::with_colliders(
            no_gravity(),
            SolverConfig::default(),
            &[
                fixed(1, Vec2::ZERO),
                particle(2, Vec2::new(0.0, 1.0), Vec2::new(0.0, -speed), 1.0),
            ],
            &[],
            &[
                rectangle(1, Vec2::new(5.0, 0.5), 0.0, 0.0),
                circle(2, 0.5, 1.0),
            ],
        )
        .unwrap();
        let report = world.step(&[]).unwrap();
        near(
            world.body(body_id(2)).unwrap().velocity_m_s.y,
            speed * effective,
            1e-10,
        );
        assert_eq!(report.contacts.len(), 1);
        assert_eq!(report.contacts[0].restitution, 1.0);
        assert_eq!(report.contacts[0].effective_restitution, effective);
    }
}
