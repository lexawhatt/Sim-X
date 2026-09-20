//! Independent SI-unit checks for body-local springs and angular dynamics.
//!
//! Expected torques follow r cross F and angular momentum includes both orbital
//! and spin terms. Contact expectations use the generalized normal effective mass.
//! Sources: OpenStax University Physics 10.7/11.2 and Erin Catto's 2006
//! Sequential Impulses notes, https://box2d.org/files/ErinCatto_SequentialImpulses_GDC2006.pdf.

use sim_physics::{
    BodyDesc, BodyId, ColliderDesc, CollisionShape, LinkDesc, LinkId, Mobility, PhysicsSettings,
    RotationDesc, SolverConfig, Vec2, World,
};

fn id(value: u64) -> BodyId {
    BodyId::new(value).unwrap()
}

fn body(value: u64, position: Vec2, velocity: Vec2, mass: f64) -> BodyDesc {
    BodyDesc {
        id: id(value),
        position_m: position,
        velocity_m_s: velocity,
        mass_kg: mass,
        mobility: Mobility::Dynamic,
    }
}

fn fixed(value: u64, position: Vec2) -> BodyDesc {
    BodyDesc {
        mobility: Mobility::Fixed,
        ..body(value, position, Vec2::ZERO, 1.0)
    }
}

fn rotation(value: u64, angle: f64, omega: f64, inertia: f64) -> RotationDesc {
    RotationDesc {
        body: id(value),
        angle_rad: angle,
        angular_velocity_rad_s: omega,
        inertia_kg_m2: inertia,
    }
}

fn settings() -> PhysicsSettings {
    PhysicsSettings {
        gravity_m_s2: Vec2::ZERO,
        linear_drag_per_s: 0.0,
    }
}

fn solver(dt: f64) -> SolverConfig {
    SolverConfig {
        fixed_dt_s: dt,
        constraint_iterations: 128,
        ..SolverConfig::default()
    }
}

fn spring(local_a: Vec2, local_b: Vec2, rest: f64, stiffness: f64) -> LinkDesc {
    LinkDesc::AttachedSpring {
        id: LinkId::new(1).unwrap(),
        a: id(1),
        b: id(2),
        local_a_m: local_a,
        local_b_m: local_b,
        rest_length_m: rest,
        stiffness_n_m: stiffness,
    }
}

fn rotate(point: Vec2, angle: f64) -> Vec2 {
    let (sine, cosine) = angle.sin_cos();
    Vec2::new(
        cosine * point.x - sine * point.y,
        sine * point.x + cosine * point.y,
    )
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:e}, expected={expected:e}, tolerance={tolerance:e}"
    );
}

fn anchored_spring(local: Vec2, anchor: Vec2, angle: f64, inertia: f64) -> World {
    World::with_rigid_bodies(
        settings(),
        solver(1e-6),
        &[body(1, Vec2::ZERO, Vec2::ZERO, 2.0), fixed(2, anchor)],
        &[spring(local, Vec2::ZERO, 1.0, 10.0)],
        &[],
        &[rotation(1, angle, 0.0, inertia)],
    )
    .unwrap()
}

#[test]
fn uniform_inertia_is_a_disk_or_rectangular_lamina_not_a_sphere() {
    let source = body(1, Vec2::ZERO, Vec2::ZERO, 6.0);
    let disk =
        RotationDesc::uniform(source, CollisionShape::Circle { radius_m: 2.0 }, 0.3, -0.5).unwrap();
    near(disk.inertia_kg_m2, 12.0, 1e-12);
    near(disk.angle_rad, 0.3, 0.0);
    near(disk.angular_velocity_rad_s, -0.5, 0.0);
    let rectangle = RotationDesc::uniform(
        source,
        CollisionShape::Box {
            half_extents_m: Vec2::new(2.0, 3.0),
            angle_rad: 0.7,
        },
        0.0,
        0.0,
    )
    .unwrap();
    near(rectangle.inertia_kg_m2, 26.0, 1e-12);
}

#[test]
fn off_center_hook_obeys_torque_sign_and_inverse_inertia() {
    for (side, inertia) in [(1.0, 2.0), (-1.0, 2.0), (1.0, 4.0)] {
        let mut world = anchored_spring(Vec2::new(side, 0.0), Vec2::new(side, 2.0), 0.0, inertia);
        world.step(&[]).unwrap();
        let moving = world.body(id(1)).unwrap();
        let spin = world.rotation(id(1)).unwrap();
        // Extension 1m * 10N/m -> (0,10)N. The first kick/drift moves by O(dt^2).
        near(moving.velocity_m_s.y / 1e-6, 5.0, 1e-8);
        near(moving.velocity_m_s.x / 1e-6, 0.0, 1e-8);
        near(
            spin.angular_velocity_rad_s / 1e-6,
            side * 10.0 / inertia,
            1e-8,
        );
        near(spin.angle_rad, side * 5.0 / inertia * 1e-12, 1e-20);
        assert_eq!(world.body(id(2)).unwrap().position_m, Vec2::new(side, 2.0));
    }
}

#[test]
fn a_force_through_the_center_does_not_create_spin() {
    let mut world = anchored_spring(Vec2::new(1.0, 0.0), Vec2::new(3.0, 0.0), 0.0, 2.0);
    for _ in 0..100 {
        world.step(&[]).unwrap();
        near(
            world.rotation(id(1)).unwrap().angular_velocity_rad_s,
            0.0,
            0.0,
        );
        near(world.rotation(id(1)).unwrap().angle_rad, 0.0, 0.0);
    }
    assert!(world.body(id(1)).unwrap().velocity_m_s.x > 0.0);
}

#[test]
fn rotating_the_whole_configuration_rotates_force_but_not_torque() {
    let quarter_turn = std::f64::consts::FRAC_PI_2;
    let mut ordinary = anchored_spring(Vec2::new(1.0, 0.0), Vec2::new(1.0, 2.0), 0.0, 2.0);
    let mut turned = anchored_spring(
        Vec2::new(1.0, 0.0),
        rotate(Vec2::new(1.0, 2.0), quarter_turn),
        quarter_turn,
        2.0,
    );
    ordinary.step(&[]).unwrap();
    turned.step(&[]).unwrap();
    let expected = rotate(ordinary.body(id(1)).unwrap().velocity_m_s, quarter_turn);
    let actual = turned.body(id(1)).unwrap().velocity_m_s;
    near(actual.x, expected.x, 1e-14);
    near(actual.y, expected.y, 1e-14);
    near(
        turned.rotation(id(1)).unwrap().angular_velocity_rad_s,
        ordinary.rotation(id(1)).unwrap().angular_velocity_rad_s,
        1e-14,
    );
}

#[test]
fn a_rotated_local_corner_sets_rest_length_not_center_distance() {
    let origin = Vec2::new(3.0, 4.0);
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1.0 / 240.0),
        &[
            body(1, origin, Vec2::ZERO, 2.0),
            fixed(2, Vec2::new(2.0, 7.0)),
        ],
        &[spring(Vec2::new(1.0, 1.0), Vec2::ZERO, 2.0, 10.0)],
        &[],
        &[rotation(1, std::f64::consts::FRAC_PI_2, 0.0, 2.0)],
    )
    .unwrap();
    // (1,1) rotated 90 degrees is (-1,1): hook is (2,5), two metres below anchor.
    for _ in 0..240 {
        let report = world.step(&[]).unwrap();
        near(report.links[0].distance_m, 2.0, 1e-12);
        near(report.links[0].elastic_energy_j, 0.0, 1e-20);
        near(world.body(id(1)).unwrap().position_m.x, origin.x, 1e-12);
        near(world.body(id(1)).unwrap().position_m.y, origin.y, 1e-12);
        near(
            world.rotation(id(1)).unwrap().angular_velocity_rad_s,
            0.0,
            1e-12,
        );
    }
}

#[test]
fn a_free_rotor_has_constant_spin_and_rotational_kinetic_energy() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1.0 / 240.0),
        &[body(1, Vec2::ZERO, Vec2::ZERO, 2.0)],
        &[],
        &[],
        &[rotation(1, 0.2, 0.7, 3.0)],
    )
    .unwrap();
    for _ in 0..480 {
        let report = world.step(&[]).unwrap();
        near(
            world.rotation(id(1)).unwrap().angular_velocity_rad_s,
            0.7,
            0.0,
        );
        near(report.energy.kinetic_j, 0.735, 1e-12);
        near(report.energy.total_j, 0.735, 1e-12);
    }
    near(world.rotation(id(1)).unwrap().angle_rad, 1.6, 1e-12);
}

fn coupled_bodies(dt: f64) -> World {
    World::with_rigid_bodies(
        settings(),
        solver(dt),
        &[
            body(1, Vec2::new(-2.0, 0.0), Vec2::new(0.3, 0.1), 2.0),
            body(2, Vec2::new(2.0, 0.5), Vec2::new(-0.2, -0.07), 3.0),
        ],
        &[spring(Vec2::new(0.4, 0.2), Vec2::new(-0.5, 0.1), 2.6, 4.0)],
        &[],
        &[rotation(1, 0.3, 0.2, 0.7), rotation(2, -0.4, -0.1, 1.1)],
    )
    .unwrap()
}

fn momentum(world: &World) -> Vec2 {
    world.bodies().iter().fold(Vec2::ZERO, |sum, body| {
        sum + body.velocity_m_s * body.mass_kg
    })
}

fn angular_momentum(world: &World) -> f64 {
    world
        .bodies()
        .iter()
        .map(|body| {
            let spin = world.rotation(body.id).unwrap();
            cross(body.position_m, body.velocity_m_s * body.mass_kg)
                + spin.inertia_kg_m2 * spin.angular_velocity_rad_s
        })
        .sum()
}

fn coupled_energy(world: &World) -> f64 {
    let kinetic: f64 = world
        .bodies()
        .iter()
        .map(|body| {
            let spin = world.rotation(body.id).unwrap();
            0.5 * body.mass_kg * body.velocity_m_s.dot(body.velocity_m_s)
                + 0.5 * spin.inertia_kg_m2 * spin.angular_velocity_rad_s.powi(2)
        })
        .sum();
    let a = world.body(id(1)).unwrap();
    let b = world.body(id(2)).unwrap();
    let hook_a = a.position_m
        + rotate(
            Vec2::new(0.4, 0.2),
            world.rotation(id(1)).unwrap().angle_rad,
        );
    let hook_b = b.position_m
        + rotate(
            Vec2::new(-0.5, 0.1),
            world.rotation(id(2)).unwrap().angle_rad,
        );
    kinetic + 0.5 * 4.0 * ((hook_b - hook_a).length() - 2.6).powi(2)
}

#[test]
fn isolated_attached_spring_conserves_orbital_plus_spin_angular_momentum() {
    let mut world = coupled_bodies(1.0 / 480.0);
    let initial_momentum = momentum(&world);
    let initial_angular = angular_momentum(&world);
    for _ in 0..960 {
        let report = world.step(&[]).unwrap();
        let current = momentum(&world);
        near(current.x, initial_momentum.x, 3e-11);
        near(current.y, initial_momentum.y, 3e-11);
        near(angular_momentum(&world), initial_angular, 3e-10);
        near(report.energy.total_j, coupled_energy(&world), 2e-12);
    }
}

#[test]
fn conservative_angular_spring_energy_error_is_bounded_and_second_order() {
    let mut errors = Vec::new();
    for rate in [240_u32, 480] {
        let mut world = coupled_bodies(1.0 / f64::from(rate));
        let initial = coupled_energy(&world);
        let mut maximum_error = 0.0_f64;
        for _ in 0..2 * rate {
            world.step(&[]).unwrap();
            maximum_error = maximum_error.max((coupled_energy(&world) - initial).abs());
        }
        assert!(
            maximum_error < 1e-4 * initial,
            "energy drift={maximum_error:e}"
        );
        errors.push(maximum_error);
    }
    assert!(
        errors[0] > 1e-10,
        "test needs resolved nonzero integration error"
    );
    assert!(
        errors[1] < 0.3 * errors[0],
        "halving the step did not recover second-order energy accuracy: {errors:?}"
    );
}

#[test]
fn rigid_body_and_link_input_order_does_not_change_the_replay() {
    let mut ordinary = coupled_bodies(1.0 / 240.0);
    let mut bodies = ordinary.bodies().to_vec();
    let mut rotations = ordinary.rotations().to_vec();
    bodies.reverse();
    rotations.reverse();
    let mut reordered = World::with_rigid_bodies(
        settings(),
        ordinary.solver(),
        &bodies,
        ordinary.links(),
        &[],
        &rotations,
    )
    .unwrap();
    assert_eq!(ordinary.snapshot(), reordered.snapshot());
    for _ in 0..120 {
        ordinary.step(&[]).unwrap();
        reordered.step(&[]).unwrap();
        assert_eq!(ordinary.snapshot(), reordered.snapshot());
    }
}

#[test]
fn invalid_rotation_components_are_rejected_before_a_world_exists() {
    let bodies = [body(1, Vec2::ZERO, Vec2::ZERO, 2.0)];
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            World::with_rigid_bodies(
                settings(),
                solver(1.0 / 240.0),
                &bodies,
                &[],
                &[],
                &[rotation(1, 0.0, 0.0, bad)],
            )
            .is_err(),
            "invalid inertia was accepted: {bad}"
        );
    }
    for bad in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e6 + 1.0,
        -1e6 - 1.0,
    ] {
        for invalid in [rotation(1, bad, 0.0, 1.0), rotation(1, 0.0, bad, 1.0)] {
            assert!(
                World::with_rigid_bodies(
                    settings(),
                    solver(1.0 / 240.0),
                    &bodies,
                    &[],
                    &[],
                    &[invalid],
                )
                .is_err()
            );
        }
    }
    for rotations in [
        vec![rotation(99, 0.0, 0.0, 1.0)],
        vec![rotation(1, 0.0, 0.0, 1.0), rotation(1, 0.0, 0.0, 1.0)],
    ] {
        assert!(
            World::with_rigid_bodies(
                settings(),
                solver(1.0 / 240.0),
                &bodies,
                &[],
                &[],
                &rotations,
            )
            .is_err()
        );
    }
    assert!(
        World::with_rigid_bodies(
            settings(),
            solver(1.0 / 240.0),
            &[fixed(1, Vec2::ZERO)],
            &[],
            &[],
            &[rotation(1, 0.0, 1.0, 1.0)],
        )
        .is_err()
    );
}

#[test]
fn nonfinite_local_attachments_do_not_create_a_partial_graph() {
    let bodies = [
        body(1, Vec2::ZERO, Vec2::ZERO, 2.0),
        fixed(2, Vec2::new(0.0, 2.0)),
    ];
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for local in [Vec2::new(bad, 0.0), Vec2::new(0.0, bad)] {
            assert!(
                World::with_rigid_bodies(
                    settings(),
                    solver(1.0 / 240.0),
                    &bodies,
                    &[spring(local, Vec2::ZERO, 1.0, 10.0)],
                    &[],
                    &[rotation(1, 0.0, 0.0, 2.0)],
                )
                .is_err()
            );
        }
    }
}

fn circle(value: u64, radius: f64, restitution: f64) -> ColliderDesc {
    ColliderDesc {
        body: id(value),
        shape: CollisionShape::Circle { radius_m: radius },
        restitution,
    }
}

fn rectangle(value: u64, half: Vec2, angle: f64, restitution: f64) -> ColliderDesc {
    ColliderDesc {
        body: id(value),
        shape: CollisionShape::Box {
            half_extents_m: half,
            angle_rad: angle,
        },
        restitution,
    }
}

#[test]
fn collider_local_rotation_composes_with_body_rotation() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-5),
        &[
            fixed(1, Vec2::ZERO),
            body(2, Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0), 1.0),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(3.0, 0.5), std::f64::consts::PI / 6.0, 0.0),
            circle(2, 0.5, 1.0),
        ],
        &[
            rotation(1, std::f64::consts::PI / 3.0, 0.0, 1.0),
            rotation(2, 0.0, 0.0, 0.125),
        ],
    )
    .unwrap();
    world.step(&[]).unwrap();
    near(world.body(id(2)).unwrap().velocity_m_s.x, 1.0, 1e-8);
    near(world.body(id(2)).unwrap().velocity_m_s.y, 0.0, 1e-8);
    near(
        world.rotation(id(2)).unwrap().angular_velocity_rad_s,
        0.0,
        1e-8,
    );
}

#[test]
fn off_center_circle_box_impact_exchanges_spin_and_orbital_momentum() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-6),
        &[
            body(1, Vec2::ZERO, Vec2::ZERO, 2.0),
            body(2, Vec2::new(1.5, 0.5), Vec2::new(-1.0, 0.0), 1.0),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(1.0, 1.0), 0.0, 1.0),
            circle(2, 0.5, 1.0),
        ],
        &[
            rotation(1, 0.0, 0.0, 4.0 / 3.0),
            rotation(2, 0.0, 0.0, 0.125),
        ],
    )
    .unwrap();
    let initial_angular = angular_momentum(&world);
    let report = world.step(&[]).unwrap();
    // K=1/2+1+(0.5^2)/(4/3)=27/16; elastic impulse J=2/K=32/27.
    let impulse = 32.0 / 27.0;
    near(
        world.body(id(1)).unwrap().velocity_m_s.x,
        -impulse / 2.0,
        2e-6,
    );
    near(
        world.body(id(2)).unwrap().velocity_m_s.x,
        -1.0 + impulse,
        2e-6,
    );
    near(
        world.rotation(id(1)).unwrap().angular_velocity_rad_s,
        0.5 * impulse / (4.0 / 3.0),
        2e-6,
    );
    near(
        world.rotation(id(2)).unwrap().angular_velocity_rad_s,
        0.0,
        2e-6,
    );
    near(momentum(&world).x, -1.0, 1e-9);
    near(angular_momentum(&world), initial_angular, 2e-6);
    near(report.energy.kinetic_j, 0.5, 2e-6);
}

#[test]
fn a_flat_box_resting_on_a_plane_does_not_acquire_spurious_spin() {
    let mut world = World::with_rigid_bodies(
        PhysicsSettings::default(),
        solver(1.0 / 240.0),
        &[
            fixed(1, Vec2::ZERO),
            body(2, Vec2::new(0.0, 1.0), Vec2::ZERO, 2.0),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(5.0, 0.5), 0.0, 0.0),
            rectangle(2, Vec2::new(1.0, 0.5), 0.0, 0.0),
        ],
        &[rotation(2, 0.0, 0.0, 5.0 / 6.0)],
    )
    .unwrap();
    for _ in 0..240 {
        world.step(&[]).unwrap();
        near(world.body(id(2)).unwrap().position_m.y, 1.0, 1e-7);
        near(world.body(id(2)).unwrap().velocity_m_s.y, 0.0, 1e-7);
        near(world.rotation(id(2)).unwrap().angle_rad, 0.0, 1e-7);
        near(
            world.rotation(id(2)).unwrap().angular_velocity_rad_s,
            0.0,
            1e-7,
        );
    }
}

#[test]
fn a_rotating_bar_cannot_sweep_through_an_obstacle_between_clear_endpoints() {
    let dt = 1.0 / 240.0;
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(dt),
        &[
            body(1, Vec2::ZERO, Vec2::ZERO, 2.0),
            fixed(2, Vec2::new(0.0, 1.5)),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(2.0, 0.05), 0.0, 0.0),
            circle(2, 0.1, 0.0),
        ],
        &[rotation(
            1,
            0.0,
            std::f64::consts::PI / dt,
            2.6683333333333334,
        )],
    )
    .unwrap();
    // Half a turn: the bar is horizontal at both endpoints but vertical halfway.
    let before = world.snapshot();
    assert!(
        world.step(&[]).is_err(),
        "unsupported angular sweep tunneled through a circle"
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn tiny_corner_rotation_cannot_cross_a_grazing_obstacle_below_the_travel_cap() {
    let dt = 1.0 / 240.0;
    let half = Vec2::new(1.0, 1.0);
    let radius = half.length();
    let obstacle = half * ((radius + 0.001 - 5e-8) / radius);
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(dt),
        &[body(1, Vec2::ZERO, Vec2::ZERO, 2.0), fixed(2, obstacle)],
        &[],
        &[rectangle(1, half, 0.0, 0.0), circle(2, 0.001, 0.0)],
        &[rotation(1, -1e-5, 2e-5 / dt, 4.0 / 3.0)],
    )
    .unwrap();
    // The corner travels 0.0000283m, below 0.25 * minimum feature 0.001m.
    // Both endpoints are clear but the midpoint penetrates by 5e-8m. A small
    // angular increment or relative movement cap alone is not a sweep proof.
    let before = world.snapshot();
    assert!(
        world.step(&[]).is_err(),
        "small-angle grazing collision was missed"
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn spring_radial_velocity_uses_hook_velocity_not_center_velocity() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-6),
        &[
            body(1, Vec2::ZERO, Vec2::ZERO, 2.0),
            fixed(2, Vec2::new(1.0, 2.0)),
        ],
        &[spring(Vec2::new(1.0, 0.0), Vec2::ZERO, 2.0, 10.0)],
        &[],
        &[rotation(1, 0.0, 1.0, 2.0)],
    )
    .unwrap();
    let report = world.step(&[]).unwrap();
    let body = world.body(id(1)).unwrap();
    let spin = world.rotation(id(1)).unwrap();
    let arm = rotate(Vec2::new(1.0, 0.0), spin.angle_rad);
    let hook_velocity = body.velocity_m_s + Vec2::new(-arm.y, arm.x) * spin.angular_velocity_rad_s;
    let direction = world.body(id(2)).unwrap().position_m - body.position_m - arm;
    let expected = -direction.dot(hook_velocity) / direction.length();
    near(report.links[0].radial_velocity_m_s, expected, 1e-10);
    near(expected, -1.0, 1e-5);
}

#[test]
fn swallowed_nonzero_angle_drift_is_an_atomic_numeric_error() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-6),
        &[body(1, Vec2::ZERO, Vec2::ZERO, 2.0)],
        &[],
        &[],
        &[rotation(1, 1e6, 1e-12, 2.0)],
    )
    .unwrap();
    let before = world.snapshot();
    assert!(
        world.step(&[]).is_err(),
        "nonzero spin silently stopped rotating"
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn centered_circle_impacts_do_not_modify_frictionless_spin() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-5),
        &[
            body(1, Vec2::new(-0.5, 0.0), Vec2::new(1.0, 0.0), 1.0),
            body(2, Vec2::new(0.5, 0.0), Vec2::new(-1.0, 0.0), 1.0),
        ],
        &[],
        &[circle(1, 0.5, 1.0), circle(2, 0.5, 1.0)],
        &[rotation(1, 0.0, 0.7, 0.125), rotation(2, 0.0, -0.2, 0.125)],
    )
    .unwrap();
    let report = world.step(&[]).unwrap();
    near(world.body(id(1)).unwrap().velocity_m_s.x, -1.0, 1e-9);
    near(world.body(id(2)).unwrap().velocity_m_s.x, 1.0, 1e-9);
    near(
        world.rotation(id(1)).unwrap().angular_velocity_rad_s,
        0.7,
        1e-12,
    );
    near(
        world.rotation(id(2)).unwrap().angular_velocity_rad_s,
        -0.2,
        1e-12,
    );
    near(report.energy.kinetic_j, 1.0 + 0.0625 * (0.49 + 0.04), 1e-10);
}

#[test]
fn an_unsafe_angular_response_rejects_the_whole_step_atomically() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-6),
        &[
            body(1, Vec2::ZERO, Vec2::ZERO, 2.0),
            fixed(2, Vec2::new(1.0, 2000.0)),
        ],
        &[spring(Vec2::new(1.0, 0.0), Vec2::ZERO, 1.0, 10.0)],
        &[],
        &[rotation(1, 0.0, 0.0, 1e-10)],
    )
    .unwrap();
    // The frequency budget h*sqrt(k/I) ~= 0.316 is valid, but the deliberately
    // enormous authored extension requires an unsupported angular displacement.
    let before = world.snapshot();
    assert!(
        world.step(&[]).is_err(),
        "extreme torque/inertia silently clamped spin"
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn fixed_support_reports_the_applied_spring_moment_without_rotating() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-6),
        &[
            body(1, Vec2::ZERO, Vec2::ZERO, 2.0),
            fixed(2, Vec2::new(0.0, 2.0)),
        ],
        &[spring(Vec2::new(1.0, 0.0), Vec2::new(1.0, 0.0), 1.0, 10.0)],
        &[],
        &[rotation(1, 0.0, 0.0, 2.0), rotation(2, 0.0, 0.0, 2.0)],
    )
    .unwrap();
    let report = world.step(&[]).unwrap();
    let fixed_load = report
        .rotations
        .iter()
        .find(|entry| entry.body == id(2))
        .unwrap();
    near(fixed_load.spring_torque_nm, -10.0, 1e-8);
    near(fixed_load.net_torque_nm, -10.0, 1e-8);
    near(fixed_load.angular_velocity_rad_s, 0.0, 0.0);
    near(fixed_load.angular_acceleration_rad_s2, 0.0, 0.0);
    near(fixed_load.angle_rad, 0.0, 0.0);
    near(fixed_load.kinetic_energy_j, 0.0, 0.0);
}

#[test]
fn unrelated_large_rotational_energy_cannot_erase_contact_energy_loss() {
    let mut world = World::with_rigid_bodies(
        settings(),
        solver(1e-6),
        &[
            body(1, Vec2::ZERO, Vec2::ZERO, 2.0),
            body(2, Vec2::new(1.5, 0.5), Vec2::new(-1.0, 0.0), 1.0),
            body(3, Vec2::new(1e6, 1e6), Vec2::ZERO, 1e12),
        ],
        &[],
        &[
            rectangle(1, Vec2::new(1.0, 1.0), 0.0, 0.0),
            circle(2, 0.5, 0.0),
        ],
        &[
            rotation(1, 0.0, 0.0, 4.0 / 3.0),
            rotation(2, 0.0, 0.0, 0.125),
            rotation(3, 0.0, 1e4, 1e24),
        ],
    )
    .unwrap();
    let report = world.step(&[]).unwrap();
    // Local inelastic loss = -0.5 * closing_speed^2 / effective_inverse_mass.
    // A world-total subtraction would round away this loss next to 5e31J.
    near(
        report.energy.contact_solve_kinetic_change_j,
        -8.0 / 27.0,
        2e-6,
    );
    near(
        world.rotation(id(3)).unwrap().angular_velocity_rad_s,
        1e4,
        0.0,
    );
    let contact = &report.contacts[0];
    assert_eq!(contact.points.len(), 1);
    let a = world.body(id(1)).unwrap();
    let b = world.body(id(2)).unwrap();
    for point in &contact.points {
        near(
            point.angular_impulse_on_a_nms,
            cross(point.position_m - a.position_m, point.impulse_on_a_ns),
            1e-12,
        );
        near(
            point.angular_impulse_on_b_nms,
            cross(point.position_m - b.position_m, -point.impulse_on_a_ns),
            1e-12,
        );
    }
}
