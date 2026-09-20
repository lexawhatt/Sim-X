//! Angular API, support stability, bounded work and diagnostic contracts.

use sim_physics::{
    BodyDesc, BodyId, ColliderDesc, CollisionShape, Error, LinkDesc, LinkId, MAX_BODIES, Mobility,
    PhysicsSettings, RotationDesc, SolverConfig, Vec2, World,
};

fn body(value: u64, position: Vec2, mobility: Mobility) -> BodyDesc {
    BodyDesc {
        id: BodyId::new(value).unwrap(),
        position_m: position,
        velocity_m_s: Vec2::ZERO,
        mass_kg: 2.0,
        mobility,
    }
}

fn rotation(source: BodyDesc, inertia: f64) -> RotationDesc {
    RotationDesc {
        body: source.id,
        angle_rad: 0.0,
        angular_velocity_rad_s: 0.0,
        inertia_kg_m2: inertia,
    }
}

#[test]
fn tall_face_contact_does_not_apply_the_same_stale_penetration_twice() {
    let floor = body(1, Vec2::new(0.0, -0.5), Mobility::Fixed);
    let tower = body(2, Vec2::new(0.0, 2.0), Mobility::Dynamic);
    let colliders = [
        ColliderDesc {
            body: floor.id,
            shape: CollisionShape::Box {
                half_extents_m: Vec2::new(5.0, 0.5),
                angle_rad: 0.0,
            },
            restitution: 0.0,
        },
        ColliderDesc {
            body: tower.id,
            shape: CollisionShape::Box {
                half_extents_m: Vec2::new(0.1, 2.0),
                angle_rad: 0.0,
            },
            restitution: 0.0,
        },
    ];
    let mut world = World::with_rigid_bodies(
        PhysicsSettings::default(),
        SolverConfig {
            constraint_iterations: 128,
            ..SolverConfig::default()
        },
        &[floor, tower],
        &[],
        &colliders,
        &[RotationDesc::uniform(tower, colliders[1].shape, 0.0, 0.0).unwrap()],
    )
    .unwrap();
    for index in 0..240 {
        let report = world.step(&[]).unwrap();
        assert_eq!(report.contacts.len(), 1);
        assert!(
            (world.body(tower.id).unwrap().position_m.y - 2.0).abs() < 1e-7,
            "step {index}: {report:?}"
        );
        assert!(world.body(tower.id).unwrap().velocity_m_s.length() < 1e-7);
        assert!(world.rotation(tower.id).unwrap().angle_rad.abs() < 1e-7);
        assert!(report.contacts[0].points.len() <= 2);
    }
}

#[test]
fn free_fast_spin_is_not_limited_by_an_unrelated_contact_step_policy() {
    let source = body(1, Vec2::ZERO, Mobility::Dynamic);
    let mut spin = rotation(source, 3.0);
    spin.angular_velocity_rad_s = 240.0;
    let mut world = World::with_rigid_bodies(
        PhysicsSettings {
            gravity_m_s2: Vec2::ZERO,
            linear_drag_per_s: 2.0,
        },
        SolverConfig::default(),
        &[source],
        &[],
        &[],
        &[spin],
    )
    .unwrap();
    let report = world.step(&[]).unwrap();
    assert_eq!(world.rotation(source.id).unwrap().angle_rad, 1.0);
    assert_eq!(
        world.rotation(source.id).unwrap().angular_velocity_rad_s,
        240.0
    );
    assert_eq!(report.bodies[0].kinetic_energy_j, 86_400.0);
    assert_eq!(report.rotations[0].kinetic_energy_j, 86_400.0);
    assert_eq!(report.energy.kinetic_j, 86_400.0);
    assert_eq!(report.energy.rotational_kinetic_j, 86_400.0);
    assert_eq!(report.energy.drag_dissipated_j, 0.0);
}

#[test]
fn angular_budget_precedes_traversal_and_missing_components_are_explicit() {
    let source = body(1, Vec2::ZERO, Mobility::Dynamic);
    let fixed = body(2, Vec2::new(3.0, 0.0), Mobility::Fixed);
    let invalid = RotationDesc {
        inertia_kg_m2: f64::NAN,
        ..rotation(source, 1.0)
    };
    assert_eq!(
        World::with_rigid_bodies(
            PhysicsSettings::default(),
            SolverConfig::default(),
            &[source],
            &[],
            &[],
            &vec![invalid; MAX_BODIES + 1],
        )
        .unwrap_err(),
        Error::RotationBudget
    );
    let link = LinkDesc::AttachedSpring {
        id: LinkId::new(1).unwrap(),
        a: source.id,
        b: fixed.id,
        local_a_m: Vec2::new(1.0, 0.0),
        local_b_m: Vec2::ZERO,
        rest_length_m: 2.0,
        stiffness_n_m: 1.0,
    };
    assert_eq!(
        World::new(
            PhysicsSettings::default(),
            SolverConfig::default(),
            &[source, fixed],
            &[link],
        )
        .unwrap_err(),
        Error::MissingRotation(source.id)
    );
}

#[test]
fn offset_spring_angular_frequency_and_travel_have_separate_atomic_guards() {
    let source = body(1, Vec2::ZERO, Mobility::Dynamic);
    let fixed = body(2, Vec2::new(3.0, 0.0), Mobility::Fixed);
    let link = LinkDesc::AttachedSpring {
        id: LinkId::new(1).unwrap(),
        a: source.id,
        b: fixed.id,
        local_a_m: Vec2::new(1.0, 0.0),
        local_b_m: Vec2::ZERO,
        rest_length_m: 2.0,
        stiffness_n_m: 1.0,
    };
    assert_eq!(
        World::with_rigid_bodies(
            PhysicsSettings::default(),
            SolverConfig::default(),
            &[source, fixed],
            &[link],
            &[],
            &[rotation(source, 1e-15)],
        )
        .unwrap_err(),
        Error::SpringStepTooLarge(link.id())
    );
    let mut spin = rotation(source, 1.0);
    spin.angular_velocity_rad_s = 240.0;
    let mut world = World::with_rigid_bodies(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &[source, fixed],
        &[link],
        &[],
        &[spin],
    )
    .unwrap();
    let before = world.snapshot();
    assert_eq!(
        world.step(&[]),
        Err(Error::AngularMotionTooLarge(source.id))
    );
    assert_eq!(world.snapshot(), before);
}
