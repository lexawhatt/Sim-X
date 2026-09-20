//! Validation, budgets, determinism and atomic transaction regressions.

mod support;
use sim_physics::*;
use support::*;

#[test]
fn maximum_work_envelopes_and_telemetry_are_bounded() {
    let bodies: Vec<_> = (1..=MAX_BODIES)
        .map(|i| body(i as u64, i as f64, 0.0, 1.0))
        .collect();
    let mut world = World::new(no_gravity(), SolverConfig::default(), &bodies, &[]).unwrap();
    let forces: Vec<_> = (1..=MAX_FORCES)
        .map(|i| force(1, i as u64, 0.001, 0.0))
        .collect();
    let report = world.step(&forces).unwrap();
    assert_eq!(report.bodies.len(), MAX_BODIES);
    assert!(report.links.is_empty());
    close(report.bodies[0].external_force_n.x, 1.024, 1e-15);
    let links: Vec<_> = (1..=MAX_LINKS)
        .map(|i| LinkDesc::Spring {
            id: link_id(i as u64),
            a: id(1),
            b: id(2),
            rest_length_m: 1.0,
            stiffness_n_m: 0.001,
        })
        .collect();
    let mut world =
        World::new(no_gravity(), SolverConfig::default(), &bodies[..2], &links).unwrap();
    let report = world.step(&[]).unwrap();
    assert_eq!(report.links.len(), MAX_LINKS);
}

#[test]
fn failure_on_last_body_keeps_earlier_candidate_changes_private() {
    let bodies = [
        BodyDesc {
            velocity_m_s: Vec2::new(1.0, 0.0),
            ..body(1, 0.0, 0.0, 1.0)
        },
        BodyDesc {
            velocity_m_s: Vec2::new(1.0, 0.0),
            ..body(2, 1e9, 0.0, 1.0)
        },
    ];
    let mut world = World::new(no_gravity(), SolverConfig::default(), &bodies, &[]).unwrap();
    let before = world.snapshot();
    assert!(world.step(&[]).is_err());
    assert_eq!(world.snapshot(), before);
}

#[test]
fn nonzero_gravitational_force_cannot_disappear_by_underflow() {
    let mut world = World::new(
        PhysicsSettings {
            gravity_m_s2: Vec2::new(f64::from_bits(1), 0.0),
            ..no_gravity()
        },
        SolverConfig::default(),
        &[body(1, 0.0, 0.0, 1e-6)],
        &[],
    )
    .unwrap();
    let before = world.snapshot();
    assert_eq!(
        world.step(&[]),
        Err(Error::PrecisionLoss(NumericStage::ForceReduction))
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn reference_recipes_are_regular_validated_mutable_definitions() {
    let mut scene = scenarios::free_fall(1.0, Vec2::ZERO, PhysicsSettings::default()).unwrap();
    let first = scene.build().unwrap();
    scene.settings.gravity_m_s2 = Vec2::ZERO;
    let second = scene.build().unwrap();
    assert_ne!(first.settings(), second.settings());
    scene.bodies[0].mass_kg = 0.0;
    assert!(scene.build().is_err());
    assert!(scenarios::pendulum(1.0, 1.0, f64::NAN, PhysicsSettings::default()).is_err());
    assert!(scenarios::pendulum(-1.0, 1.0, 0.0, PhysicsSettings::default()).is_err());
    assert!(
        scenarios::spring_oscillator(1.0, 4.0, 0.2, no_gravity())
            .unwrap()
            .build()
            .is_ok()
    );
    assert!(scenarios::spring_oscillator(1.0, 4.0, -1.0, no_gravity()).is_err());
}

#[test]
fn seeded_finite_free_particles_match_independent_analytic_updates() {
    // A tiny deterministic generator avoids an ambient RNG and extra runtime
    // dependencies. This is a reproducible property sweep, not statistical proof.
    let mut state = 0x9e3779b97f4a7c15_u64;
    let mut sample = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((state >> 11) as f64) / (1_u64 << 53) as f64
    };
    for _ in 0..200 {
        let mass = 0.1 + sample() * 1000.0;
        let position = Vec2::new(sample() * 20.0 - 10.0, sample() * 20.0 - 10.0);
        let velocity = Vec2::new(sample() * 20.0 - 10.0, sample() * 20.0 - 10.0);
        let gravity = Vec2::new(sample() * 20.0 - 10.0, sample() * 20.0 - 10.0);
        let applied = Vec2::new(sample() * 20.0 - 10.0, sample() * 20.0 - 10.0);
        let initial = BodyDesc {
            id: id(1),
            position_m: position,
            velocity_m_s: velocity,
            mass_kg: mass,
            mobility: Mobility::Dynamic,
        };
        let mut world = World::new(
            PhysicsSettings {
                gravity_m_s2: gravity,
                ..no_gravity()
            },
            SolverConfig::default(),
            &[initial],
            &[],
        )
        .unwrap();
        world.step(&[force(1, 1, applied.x, applied.y)]).unwrap();
        let dt = world.solver().fixed_dt_s;
        let acceleration = gravity + applied * (1.0 / mass);
        close(
            (world.bodies()[0].velocity_m_s - (velocity + acceleration * dt)).length(),
            0.0,
            5e-14,
        );
        close(
            (world.bodies()[0].position_m
                - (position + velocity * dt + acceleration * (0.5 * dt * dt)))
                .length(),
            0.0,
            5e-14,
        );
    }
}

#[test]
fn source_renaming_and_input_permutations_do_not_change_physics() {
    let base = World::new(
        no_gravity(),
        SolverConfig::default(),
        &[body(1, 0.0, 0.0, 1.0)],
        &[],
    )
    .unwrap();
    let mut first = base.clone();
    let mut second = base.clone();
    let mut third = base;
    first
        .step(&[
            force(1, 1, 1e16, 0.0),
            force(1, 2, 1.0, 0.0),
            force(1, 3, -1e16, 0.0),
        ])
        .unwrap();
    second
        .step(&[
            force(1, 3, 1e16, 0.0),
            force(1, 1, -1e16, 0.0),
            force(1, 2, 1.0, 0.0),
        ])
        .unwrap();
    third
        .step(&[
            force(1, 1, f64::MAX, 0.0),
            force(1, 2, 1.0, 0.0),
            force(1, 3, -f64::MAX, 0.0),
        ])
        .unwrap();
    assert_eq!(first.snapshot(), second.snapshot());
    assert_eq!(first.snapshot(), third.snapshot());
    assert_eq!(
        first.last_report().unwrap().bodies[0].external_force_n.x,
        1.0
    );
}

#[test]
fn replay_and_constructor_order_are_identical() {
    let bodies = [
        fixed(1, 0.0, 0.0),
        body(2, 1.0, 0.0, 2.0),
        body(3, 2.0, 0.0, 1.0),
    ];
    let links = [
        rod(1, 1, 2, 1.0),
        LinkDesc::Spring {
            id: link_id(2),
            a: id(2),
            b: id(3),
            rest_length_m: 1.0,
            stiffness_n_m: 10.0,
        },
    ];
    let mut first = World::new(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &bodies,
        &links,
    )
    .unwrap();
    let mut second = World::new(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &[bodies[2], bodies[0], bodies[1]],
        &[links[1], links[0]],
    )
    .unwrap();
    for _ in 0..1000 {
        assert_eq!(first.step(&[]), second.step(&[]));
    }
    assert_eq!(first.snapshot(), second.snapshot());
}

#[test]
fn setting_change_is_atomic_and_affects_every_dynamic_body_next_step() {
    let mut world = World::new(
        no_gravity(),
        SolverConfig::default(),
        &[body(1, 0.0, 0.0, 1.0), body(2, 0.0, 1.0, 2.0)],
        &[],
    )
    .unwrap();
    world.step(&[]).unwrap();
    let before = world.snapshot();
    assert_eq!(world.set_settings(no_gravity()), Ok(false));
    assert_eq!(world.snapshot(), before);
    assert_eq!(
        world.set_settings(PhysicsSettings {
            gravity_m_s2: Vec2::new(f64::NAN, 0.0),
            ..no_gravity()
        }),
        Err(Error::InvalidSettings)
    );
    assert_eq!(world.snapshot(), before);
    assert_eq!(
        world.set_settings(PhysicsSettings {
            gravity_m_s2: Vec2::new(0.0, -2.0),
            ..no_gravity()
        }),
        Ok(true)
    );
    assert_eq!(world.step_index(), 1);
    assert_eq!(world.elapsed_s(), before.elapsed_s);
    assert!(world.last_report().is_none());
    let report = world.step(&[]).unwrap();
    assert_eq!(report.settings_revision, 1);
    for body in report.bodies {
        close(body.acceleration_m_s2.y, -2.0, 1e-12);
    }
}

#[test]
fn budgets_precede_invalid_contents_and_steps_are_atomic() {
    let invalid = BodyDesc {
        mass_kg: f64::NAN,
        ..body(1, 0.0, 0.0, 1.0)
    };
    assert!(matches!(
        World::new(
            no_gravity(),
            SolverConfig::default(),
            &vec![invalid; MAX_BODIES + 1],
            &[]
        ),
        Err(Error::BodyBudget)
    ));
    assert!(matches!(
        World::new(
            no_gravity(),
            SolverConfig::default(),
            &[invalid],
            &vec![rod(1, 1, 1, 0.0); MAX_LINKS + 1]
        ),
        Err(Error::LinkBudget)
    ));
    let mut world = World::new(
        no_gravity(),
        SolverConfig::default(),
        &[body(1, 0.0, 0.0, 1.0)],
        &[],
    )
    .unwrap();
    world.step(&[]).unwrap();
    let snapshot = world.snapshot();
    assert_eq!(
        world.step(&vec![force(99, 1, f64::NAN, 0.0); MAX_FORCES + 1]),
        Err(Error::ForceBudget)
    );
    assert_eq!(world.snapshot(), snapshot);
    for forces in [
        vec![force(1, 1, 1.0, 0.0), force(1, 1, 2.0, 0.0)],
        vec![force(99, 1, 1.0, 0.0)],
        vec![force(1, 1, f64::NAN, 0.0)],
        vec![force(1, 1, f64::MAX, 0.0)],
        vec![force(1, 1, f64::MAX, 0.0), force(1, 2, f64::MAX, 0.0)],
    ] {
        assert!(world.step(&forces).is_err());
        assert_eq!(world.snapshot(), snapshot);
    }
}

#[test]
fn fixed_force_and_absorbed_state_progress_reject_atomically() {
    let mut world = World::new(
        no_gravity(),
        SolverConfig::default(),
        &[fixed(1, 0.0, 0.0)],
        &[],
    )
    .unwrap();
    let snapshot = world.snapshot();
    assert_eq!(
        world.step(&[force(1, 1, 1.0, 0.0)]),
        Err(Error::FixedForce(id(1)))
    );
    assert_eq!(world.snapshot(), snapshot);
    let initial = BodyDesc {
        velocity_m_s: Vec2::new(1e-12, 0.0),
        ..body(1, 1e8, 0.0, 1.0)
    };
    let mut world = World::new(no_gravity(), SolverConfig::default(), &[initial], &[]).unwrap();
    let snapshot = world.snapshot();
    assert_eq!(
        world.step(&[]),
        Err(Error::PrecisionLoss(NumericStage::Position))
    );
    assert_eq!(world.snapshot(), snapshot);
    let mut world = World::new(
        no_gravity(),
        SolverConfig::default(),
        &[body(1, 0.0, 0.0, 1.0)],
        &[],
    )
    .unwrap();
    let snapshot = world.snapshot();
    assert_eq!(
        world.step(&[force(1, 1, f64::from_bits(1), 0.0)]),
        Err(Error::PrecisionLoss(NumericStage::Velocity))
    );
    assert_eq!(world.snapshot(), snapshot);
}

#[test]
fn invalid_graphs_settings_and_solver_are_rejected() {
    assert_eq!(BodyId::new(0), Err(Error::ZeroId));
    assert_eq!(BodyId::new(u64::MAX).unwrap().get(), u64::MAX);
    let bodies = [fixed(1, 0.0, 0.0), body(2, 1.0, 0.0, 1.0)];
    for links in [
        vec![rod(1, 1, 1, 1.0)],
        vec![rod(1, 1, 3, 1.0)],
        vec![rod(1, 1, 2, 2.0)],
        vec![rod(1, 1, 2, f64::NAN)],
        vec![rod(1, 1, 2, 1.0), rod(2, 2, 1, 1.0)],
        vec![rod(1, 1, 2, 1.0), rod(1, 1, 2, 1.0)],
    ] {
        assert!(World::new(no_gravity(), SolverConfig::default(), &bodies, &links).is_err());
    }
    assert!(matches!(
        World::new(
            no_gravity(),
            SolverConfig::default(),
            &[bodies[0], bodies[0]],
            &[]
        ),
        Err(Error::DuplicateBody(_))
    ));
    assert!(
        World::new(
            no_gravity(),
            SolverConfig::default(),
            &[fixed(1, 0.0, 0.0), fixed(2, 1.0, 0.0)],
            &[rod(1, 1, 2, 1.0)]
        )
        .is_err()
    );
    for dt in [0.0, -1.0, f64::NAN, 1.0, f64::INFINITY] {
        assert!(matches!(
            World::new(
                no_gravity(),
                SolverConfig {
                    fixed_dt_s: dt,
                    ..SolverConfig::default()
                },
                &[],
                &[]
            ),
            Err(Error::InvalidSolver)
        ));
    }
    for mass in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e20] {
        assert!(
            World::new(
                no_gravity(),
                SolverConfig::default(),
                &[body(1, 0.0, 0.0, mass)],
                &[]
            )
            .is_err()
        );
    }
    let moving = BodyDesc {
        velocity_m_s: Vec2::new(1.0, 0.0),
        ..bodies[1]
    };
    assert!(matches!(
        World::new(
            no_gravity(),
            SolverConfig::default(),
            &[bodies[0], moving],
            &[rod(1, 1, 2, 1.0)]
        ),
        Err(Error::InitialConstraint(_))
    ));
}

#[test]
fn spring_stability_budget_is_graph_wide() {
    let settings = SolverConfig {
        fixed_dt_s: 1.0 / 30.0,
        ..SolverConfig::default()
    };
    let bodies = [fixed(1, 0.0, 0.0), body(2, 1.0, 0.0, 1.0)];
    let links: Vec<_> = (1..=4)
        .map(|i| LinkDesc::Spring {
            id: link_id(i),
            a: id(1),
            b: id(2),
            rest_length_m: 1.0,
            stiffness_n_m: 100.0,
        })
        .collect();
    assert!(World::new(no_gravity(), settings, &bodies, &links[..1]).is_ok());
    assert!(matches!(
        World::new(no_gravity(), settings, &bodies, &links),
        Err(Error::SpringStepTooLarge(_))
    ));
}

#[test]
fn nonconvergent_constraint_step_has_no_partial_commit() {
    let mut world = World::new(
        PhysicsSettings::default(),
        SolverConfig {
            constraint_iterations: 1,
            ..SolverConfig::default()
        },
        &[
            fixed(1, 0.0, 0.0),
            body(2, 1.0, 0.0, 1.0),
            body(3, 2.0, 0.0, 1.0),
        ],
        &[rod(1, 1, 2, 1.0), rod(2, 2, 3, 1.0)],
    )
    .unwrap();
    let snapshot = world.snapshot();
    assert!(matches!(
        world.step(&[force(3, 1, 100.0, 10.0)]),
        Err(Error::ConstraintFailure(_))
    ));
    assert_eq!(world.snapshot(), snapshot);
}
