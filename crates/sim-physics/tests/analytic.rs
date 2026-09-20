//! Analytic trajectories, physical invariants and measured convergence.

mod support;
use sim_physics::*;
use support::*;

fn pendulum(mass: f64, gravity: f64, dt: f64, angle: f64) -> World {
    let mut definition = scenarios::pendulum(
        1.0,
        mass,
        angle,
        PhysicsSettings {
            gravity_m_s2: Vec2::new(0.0, -gravity),
            ..no_gravity()
        },
    )
    .unwrap();
    definition.solver.fixed_dt_s = dt;
    definition.build().unwrap()
}

#[test]
fn free_fall_matches_analytic_trajectory_and_energy() {
    let initial = BodyDesc {
        velocity_m_s: Vec2::new(2.0, 3.0),
        ..body(1, 4.0, 8.0, 2.0)
    };
    let mut world = World::new(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &[initial],
        &[],
    )
    .unwrap();
    for _ in 0..480 {
        world.step(&[]).unwrap();
    }
    let t = world.elapsed_s();
    let state = world.body(id(1)).unwrap();
    close(state.position_m.x, 4.0 + 2.0 * t, 1e-11);
    close(
        state.position_m.y,
        8.0 + 3.0 * t - 0.5 * 9.80665 * t * t,
        1e-10,
    );
    close(state.velocity_m_s.y, 3.0 - 9.80665 * t, 1e-11);
    close(
        world.last_report().unwrap().energy.total_j,
        13.0 + 2.0 * 9.80665 * 8.0,
        1e-9,
    );
}

#[test]
fn external_fma_is_mass_sensitive_and_only_lasts_one_step() {
    let mut world = World::new(
        no_gravity(),
        SolverConfig::default(),
        &[body(1, 0.0, 0.0, 1.0), body(2, 0.0, 0.0, 2.0)],
        &[],
    )
    .unwrap();
    let report = world
        .step(&[force(1, 1, 6.0, -2.0), force(2, 1, 6.0, -2.0)])
        .unwrap();
    close(report.bodies[0].acceleration_m_s2.x, 6.0, 1e-12);
    close(report.bodies[1].acceleration_m_s2.x, 3.0, 1e-12);
    let velocity = world.bodies()[0].velocity_m_s;
    let report = world.step(&[]).unwrap();
    assert_eq!(report.bodies[0].external_force_n, Vec2::ZERO);
    assert_eq!(world.bodies()[0].velocity_m_s, velocity);
}

#[test]
fn rod_preserves_distance_tangency_and_reports_support_force() {
    let mut world = pendulum(2.0, 9.80665, 1.0 / 240.0, 0.0);
    let report = world.step(&[]).unwrap();
    assert_eq!(world.bodies()[0].position_m, Vec2::ZERO);
    close(report.links[0].distance_m, 1.0, 1e-10);
    close(report.links[0].radial_velocity_m_s, 0.0, 1e-10);
    close(report.bodies[1].constraint_force_n.y, 19.6133, 1e-8);
    close(report.bodies[1].net_force_n.y, 0.0, 1e-8);
    close(report.links[0].force_on_a_n.y, -19.6133, 1e-8);
}

fn measure_period(gravity: f64, mass: f64) -> f64 {
    let mut world = pendulum(mass, gravity, 1.0 / 480.0, 0.02);
    let mut previous = world.bodies()[1].position_m.x;
    let mut crossings = Vec::new();
    for _ in 0..15000 {
        world.step(&[]).unwrap();
        let current = world.bodies()[1].position_m.x;
        if previous > 0.0 && current <= 0.0 {
            let fraction = previous / (previous - current);
            crossings.push(world.elapsed_s() - (1.0 - fraction) * world.solver().fixed_dt_s);
            if crossings.len() == 2 {
                return crossings[1] - crossings[0];
            }
        }
        previous = current;
    }
    panic!("pendulum did not complete period")
}

#[test]
fn pendulum_period_gravity_scaling_and_mass_independence() {
    let earth = measure_period(9.80665, 1.0);
    let heavy = measure_period(9.80665, 1000.0);
    let low = measure_period(9.80665 / 4.0, 1.0);
    let analytic = std::f64::consts::TAU / (9.80665_f64).sqrt();
    close(earth, analytic, 8e-5);
    close(heavy, earth, 1e-9);
    close(low / earth, 2.0, 2e-5);
}

fn pendulum_energy_error(dt: f64) -> f64 {
    let angle = 0.7;
    let mut world = pendulum(2.0, 9.80665, dt, angle);
    let initial = -2.0 * 9.80665 * angle.cos();
    let mut maximum: f64 = 0.0;
    for _ in 0..(40.0 / dt) as usize {
        let report = world.step(&[]).unwrap();
        maximum = maximum.max((report.energy.total_j - initial).abs());
        assert!(report.links[0].extension_m.abs() < 2e-9);
        assert!(report.links[0].radial_velocity_m_s.abs() < 2e-9);
    }
    maximum
}

#[test]
fn long_pendulum_run_has_bounded_energy_and_second_order_convergence() {
    let coarse = pendulum_energy_error(1.0 / 120.0);
    let fine = pendulum_energy_error(1.0 / 240.0);
    assert!(coarse < 0.01, "coarse energy error {coarse}");
    assert!(fine < coarse * 0.3, "coarse={coarse},fine={fine}");
}

#[test]
fn hooke_oscillator_matches_period_and_bounded_energy() {
    let mut world = World::new(
        no_gravity(),
        SolverConfig::default(),
        &[fixed(1, 0.0, 0.0), body(2, 1.2, 0.0, 1.0)],
        &[LinkDesc::Spring {
            id: link_id(1),
            a: id(1),
            b: id(2),
            rest_length_m: 1.0,
            stiffness_n_m: 4.0,
        }],
    )
    .unwrap();
    let mut max_energy: f64 = 0.0;
    let mut max_position: f64 = 0.0;
    for _ in 0..10000 {
        let report = world.step(&[]).unwrap();
        max_energy = max_energy.max((report.energy.total_j - 0.08).abs());
        max_position = max_position.max(
            (world.bodies()[1].position_m.x - (1.0 + 0.2 * (2.0 * world.elapsed_s()).cos())).abs(),
        );
    }
    assert!(max_energy < 2e-6, "energy {max_energy}");
    assert!(max_position < 5e-5, "position {max_position}");
}

#[test]
fn internal_spring_and_rod_forces_preserve_linear_momentum() {
    let initial = [
        BodyDesc {
            velocity_m_s: Vec2::new(1.0, 0.0),
            ..body(1, -0.5, 0.0, 2.0)
        },
        BodyDesc {
            velocity_m_s: Vec2::new(-2.0, 0.0),
            ..body(2, 0.5, 0.0, 1.0)
        },
    ];
    let spring = LinkDesc::Spring {
        id: link_id(1),
        a: id(1),
        b: id(2),
        rest_length_m: 2.0,
        stiffness_n_m: 3.0,
    };
    let mut world = World::new(no_gravity(), SolverConfig::default(), &initial, &[spring]).unwrap();
    for _ in 0..1000 {
        let r = world.step(&[]).unwrap();
        close(
            r.bodies[0].velocity_m_s.x * 2.0 + r.bodies[1].velocity_m_s.x,
            0.0,
            1e-12,
        );
    }
    let initial = [
        BodyDesc {
            velocity_m_s: Vec2::new(0.0, 1.0),
            ..body(1, -0.5, 0.0, 2.0)
        },
        BodyDesc {
            velocity_m_s: Vec2::new(0.0, -2.0),
            ..body(2, 0.5, 0.0, 1.0)
        },
    ];
    let mut world = World::new(
        no_gravity(),
        SolverConfig::default(),
        &initial,
        &[rod(1, 1, 2, 1.0)],
    )
    .unwrap();
    for _ in 0..1000 {
        let r = world.step(&[]).unwrap();
        close(
            (r.bodies[0].velocity_m_s * 2.0 + r.bodies[1].velocity_m_s).length(),
            0.0,
            1e-11,
        );
    }
}

#[test]
fn three_link_chain_converges_with_documented_sweeps() {
    let mut world = World::new(
        PhysicsSettings::default(),
        SolverConfig {
            constraint_iterations: 96,
            ..SolverConfig::default()
        },
        &[
            fixed(1, 0.0, 0.0),
            body(2, 1.0, 0.0, 1.0),
            body(3, 2.0, 0.0, 1.0),
            body(4, 3.0, 0.0, 1.0),
        ],
        &[rod(1, 1, 2, 1.0), rod(2, 2, 3, 1.0), rod(3, 3, 4, 1.0)],
    )
    .unwrap();
    for _ in 0..1000 {
        let report = world.step(&[]).unwrap();
        for link in report.links {
            assert!(link.extension_m.abs() < 2e-9);
        }
    }
}

#[test]
fn exact_drag_decay_and_energy_accounting() {
    let initial = BodyDesc {
        velocity_m_s: Vec2::new(2.0, -3.0),
        ..body(1, 0.0, 0.0, 2.0)
    };
    let mut world = World::new(
        PhysicsSettings {
            linear_drag_per_s: 0.3,
            ..no_gravity()
        },
        SolverConfig::default(),
        &[initial],
        &[],
    )
    .unwrap();
    let mut dissipated = 0.0;
    for _ in 0..480 {
        let r = world.step(&[]).unwrap();
        dissipated += r.energy.drag_dissipated_j;
    }
    close(
        world.bodies()[0].velocity_m_s.x,
        2.0 * (-0.6_f64).exp(),
        1e-12,
    );
    close(
        world.bodies()[0].velocity_m_s.y,
        -3.0 * (-0.6_f64).exp(),
        1e-12,
    );
    close(
        world.last_report().unwrap().energy.kinetic_j + dissipated,
        13.0,
        1e-11,
    );
}

#[test]
fn telemetry_force_balance_matches_momentum_change_with_drag_and_springs() {
    let mut world = World::new(
        PhysicsSettings {
            linear_drag_per_s: 0.2,
            ..PhysicsSettings::default()
        },
        SolverConfig::default(),
        &[fixed(1, 0.0, 0.0), body(2, 1.1, -1.0, 2.0)],
        &[LinkDesc::Spring {
            id: link_id(1),
            a: id(1),
            b: id(2),
            rest_length_m: 1.0,
            stiffness_n_m: 10.0,
        }],
    )
    .unwrap();
    for _ in 0..1000 {
        let report = world.step(&[force(2, 1, 0.2, 0.0)]).unwrap();
        let body = &report.bodies[1];
        close(
            (body.net_force_n - body.acceleration_m_s2 * 2.0).length(),
            0.0,
            1e-10,
        );
    }
}
