//! Energy diagnostics must not lose representable results in tiny intermediates.

use sim_physics::*;

fn tiny_particle(velocity: f64, drag: f64) -> World {
    World::new(
        PhysicsSettings {
            gravity_m_s2: Vec2::ZERO,
            linear_drag_per_s: drag,
        },
        SolverConfig::default(),
        &[BodyDesc {
            id: BodyId::new(1).unwrap(),
            position_m: Vec2::ZERO,
            velocity_m_s: Vec2::new(velocity, 0.0),
            mass_kg: 1e12,
            mobility: Mobility::Dynamic,
        }],
        &[],
    )
    .unwrap()
}

#[test]
fn kinetic_energy_survives_an_underflowing_velocity_square() {
    let mut world = tiny_particle(1e-162, 0.0);
    assert_eq!(world.bodies()[0].velocity_m_s.x.powi(2), 0.0);
    let report = world.step(&[]).unwrap();
    assert!(report.bodies[0].position_m.x > 0.0);
    assert!(report.energy.kinetic_j > 0.0);
    assert!((report.energy.kinetic_j / 5e-313 - 1.0).abs() < 1e-10);
    assert_eq!(report.energy.kinetic_j, report.bodies[0].kinetic_energy_j);
    assert_eq!(report.energy.total_j, report.energy.kinetic_j);
}

#[test]
fn drag_work_and_remaining_energy_stay_representable_and_balance() {
    let mut world = tiny_particle(1e-162, 1.0);
    let report = world.step(&[]).unwrap();
    let velocity = world.bodies()[0].velocity_m_s.x;
    let expected_energy = ((0.5 * 1e12) * velocity) * velocity;
    let expected_work = 5e-313 - expected_energy;
    assert!(report.energy.drag_dissipated_j > 0.0);
    assert!((report.energy.kinetic_j / expected_energy - 1.0).abs() < 1e-10);
    assert!((report.energy.drag_dissipated_j / expected_work - 1.0).abs() < 1e-7);
    assert!(
        ((report.energy.kinetic_j + report.energy.drag_dissipated_j) / 5e-313 - 1.0).abs() < 1e-10
    );
}

#[test]
fn truly_unrepresentable_energy_rejects_the_entire_step() {
    let mut world = tiny_particle(1e-180, 0.0);
    let before = world.snapshot();
    assert_eq!(
        world.step(&[]),
        Err(Error::PrecisionLoss(NumericStage::Telemetry))
    );
    assert_eq!(world.snapshot(), before);
}

#[test]
fn unrepresentable_drag_work_is_not_silently_reported_as_zero() {
    let mut world = tiny_particle(1e-162, 1e-12);
    let before = world.snapshot();
    assert_eq!(
        world.step(&[]),
        Err(Error::PrecisionLoss(NumericStage::Telemetry))
    );
    assert_eq!(world.snapshot(), before);
}
