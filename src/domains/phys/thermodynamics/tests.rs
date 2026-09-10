use crate::foundation::{EntityId, TimeStep};

use super::{
    ConductiveLinkSpec, JoulesPerKelvin, Kelvin, MAX_THERMAL_BODIES, MAX_THERMAL_EVENTS,
    MAX_THERMAL_LINKS, ThermalArithmetic, ThermalBodySpec, ThermalEnergyTotal, ThermalError,
    ThermalEventKind, ThermalWorld, WattsPerKelvin,
};

fn body(temperature: f64, capacity: f64) -> ThermalBodySpec {
    ThermalBodySpec::new(
        Kelvin::new(temperature).expect("valid test temperature"),
        JoulesPerKelvin::new(capacity).expect("valid test capacity"),
    )
}

fn conductance(value: f64) -> WattsPerKelvin {
    WattsPerKelvin::new(value).expect("valid test conductance")
}

#[test]
fn quantities_reject_non_positive_and_non_finite_values() {
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Kelvin::new(invalid).is_err());
        assert!(JoulesPerKelvin::new(invalid).is_err());
        assert!(WattsPerKelvin::new(invalid).is_err());
    }
}

#[test]
fn two_bodies_exchange_energy_toward_equilibrium() {
    let mut world = ThermalWorld::new();
    let hot = world.create_body(body(400.0, 100.0)).expect("hot body");
    let cold = world.create_body(body(300.0, 100.0)).expect("cold body");
    world
        .create_link(ConductiveLinkSpec::new(hot, cold, conductance(10.0)))
        .expect("thermal link");

    let report = world
        .step(TimeStep::new(1.0).expect("valid dt"))
        .expect("stable conduction");

    assert_eq!(report.snapshot.bodies[0].temperature.get(), 390.0);
    assert_eq!(report.snapshot.bodies[1].temperature.get(), 310.0);
    assert_eq!(report.snapshot.bodies[0].net_heat_power.get(), -1_000.0);
    assert_eq!(report.snapshot.bodies[1].net_heat_power.get(), 1_000.0);
    assert!(report.events.len() <= MAX_THERMAL_EVENTS);
    assert!(matches!(
        report.events.last().map(|event| &event.kind),
        Some(ThermalEventKind::StepCommitted {
            bodies: 2,
            links: 1
        })
    ));
}

#[test]
fn equal_temperatures_remain_equal() {
    let mut world = ThermalWorld::new();
    let first = world.create_body(body(300.0, 20.0)).expect("first");
    let second = world.create_body(body(300.0, 30.0)).expect("second");
    world
        .create_link(ConductiveLinkSpec::new(first, second, conductance(5.0)))
        .expect("link");
    let before_energy: f64 = world
        .snapshot()
        .expect("snapshot")
        .bodies
        .iter()
        .map(|item| item.energy.get())
        .sum();

    let after = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("step")
        .snapshot;
    let after_energy: f64 = after.bodies.iter().map(|item| item.energy.get()).sum();
    assert_eq!(before_energy, after_energy);
    assert!(
        after
            .bodies
            .iter()
            .all(|item| item.temperature.get() == 300.0)
    );
}

#[test]
fn unstable_time_step_rejects_atomically() {
    let mut world = ThermalWorld::new();
    let first = world.create_body(body(400.0, 1.0)).expect("first");
    let second = world.create_body(body(300.0, 1.0)).expect("second");
    world
        .create_link(ConductiveLinkSpec::new(first, second, conductance(2.0)))
        .expect("link");
    let before = world.clone();

    let error = world.step(TimeStep::new(1.0).expect("dt"));
    assert!(matches!(
        error,
        Err(ThermalError::UnstableLinkTimeStep { .. }) | Err(ThermalError::UnstableTimeStep { .. })
    ));
    assert_eq!(world, before);
}

#[test]
fn pair_stability_rejects_temperature_crossing_atomically() {
    let mut world = ThermalWorld::new();
    let hot = world.create_body(body(400.0, 1.0)).expect("hot");
    let cold = world.create_body(body(300.0, 1.0)).expect("cold");
    world
        .create_link(ConductiveLinkSpec::new(hot, cold, conductance(0.75)))
        .expect("link");
    let before = world.clone();

    let error = world.step(TimeStep::new(1.0).expect("dt"));

    assert!(matches!(
        error,
        Err(ThermalError::UnstableLinkTimeStep { ratio, .. }) if ratio == 1.5
    ));
    assert_eq!(world, before);
}

#[test]
fn pair_stability_accepts_tiny_capacities_without_inverse_overflow() {
    let mut world = ThermalWorld::new();
    let hot = world.create_body(body(2.0, 1.0e-308)).expect("hot");
    let cold = world.create_body(body(1.0, 1.0e-308)).expect("cold");
    world
        .create_link(ConductiveLinkSpec::new(
            hot,
            cold,
            conductance(f64::from_bits(1)),
        ))
        .expect("link");

    let report = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("small dimensionless pair ratio is stable");

    assert_eq!(report.snapshot.step.get(), 1);
    assert!(report.snapshot.bodies[0].temperature.get() < 2.0);
    assert!(report.snapshot.bodies[1].temperature.get() > 1.0);
}

#[test]
fn body_stability_accepts_scaled_incident_ratios_when_conductance_sum_overflows() {
    let mut world = ThermalWorld::new();
    let center = world.create_body(body(1.0, f64::MAX)).expect("center");
    let left = world.create_body(body(1.0, f64::MAX)).expect("left");
    let right = world.create_body(body(1.0, f64::MAX)).expect("right");
    let maximum_conductance = conductance(f64::MAX);
    world
        .create_link(ConductiveLinkSpec::new(center, left, maximum_conductance))
        .expect("left link");
    world
        .create_link(ConductiveLinkSpec::new(center, right, maximum_conductance))
        .expect("right link");

    let report = world
        .step(TimeStep::new(f64::from_bits(1)).expect("minimum positive dt"))
        .expect("small incident dimensionless ratios are stable");

    assert_eq!(report.snapshot.step.get(), 1);
    assert_eq!(report.snapshot.elapsed.get().to_bits(), 1);
    assert!(
        report
            .snapshot
            .bodies
            .iter()
            .all(|snapshot| snapshot.temperature.get() == 1.0)
    );
}

#[test]
fn invalid_and_duplicate_links_commit_nothing() {
    let mut world = ThermalWorld::new();
    let first = world.create_body(body(400.0, 10.0)).expect("first");
    let second = world.create_body(body(300.0, 10.0)).expect("second");
    let before = world.clone();
    assert!(matches!(
        world.create_link(ConductiveLinkSpec::new(
            first,
            EntityId::new(999).expect("missing ID"),
            conductance(1.0),
        )),
        Err(ThermalError::MissingBody { .. })
    ));
    assert_eq!(world, before);
    assert!(matches!(
        world.create_link(ConductiveLinkSpec::new(first, first, conductance(1.0))),
        Err(ThermalError::IdenticalEndpoints { .. })
    ));
    world
        .create_link(ConductiveLinkSpec::new(first, second, conductance(1.0)))
        .expect("first link");
    let before = world.clone();
    assert!(matches!(
        world.create_link(ConductiveLinkSpec::new(second, first, conductance(2.0))),
        Err(ThermalError::DuplicateLink { .. })
    ));
    assert_eq!(world, before);
}

#[test]
fn initial_energy_overflow_and_underflow_reject_body_creation_atomically() {
    let mut world = ThermalWorld::new();
    let before = world.clone();

    assert!(matches!(
        world.create_body(body(f64::MAX, 2.0)),
        Err(ThermalError::DerivedNonFinite {
            operation: ThermalArithmetic::InitialEnergy,
            ..
        })
    ));
    assert_eq!(world, before);

    assert!(matches!(
        world.create_body(body(f64::from_bits(1), f64::from_bits(1))),
        Err(ThermalError::PrecisionLoss {
            operation: ThermalArithmetic::InitialEnergy,
            ..
        })
    ));
    assert_eq!(world, before);
    assert!(
        world
            .snapshot()
            .expect("unchanged snapshot")
            .bodies
            .is_empty()
    );
}

#[test]
fn step_arithmetic_overflow_and_underflow_are_structured_and_atomic() {
    let mut power_overflow = ThermalWorld::new();
    let capacity = f64::MAX / 4.0;
    let hot = power_overflow
        .create_body(body(3.0, capacity))
        .expect("hot");
    let cold = power_overflow
        .create_body(body(1.0, capacity))
        .expect("cold");
    power_overflow
        .create_link(ConductiveLinkSpec::new(hot, cold, conductance(f64::MAX)))
        .expect("link");
    let before = power_overflow.clone();
    assert!(matches!(
        power_overflow.step(TimeStep::new(f64::from_bits(1)).expect("minimum dt")),
        Err(ThermalError::DerivedNonFinite {
            operation: ThermalArithmetic::ConductivePower,
            ..
        })
    ));
    assert_eq!(power_overflow, before);

    let mut power_underflow = ThermalWorld::new();
    let hot = power_underflow.create_body(body(1.5, 1.0)).expect("hot");
    let cold = power_underflow.create_body(body(1.0, 1.0)).expect("cold");
    power_underflow
        .create_link(ConductiveLinkSpec::new(
            hot,
            cold,
            conductance(f64::from_bits(1)),
        ))
        .expect("link");
    let before = power_underflow.clone();
    assert!(matches!(
        power_underflow.step(TimeStep::new(1.0).expect("dt")),
        Err(ThermalError::PrecisionLoss {
            operation: ThermalArithmetic::ConductivePower,
            ..
        })
    ));
    assert_eq!(power_underflow, before);

    let mut heat_underflow = ThermalWorld::new();
    let hot = heat_underflow
        .create_body(body(f64::from_bits(2), 1.0))
        .expect("hot");
    let cold = heat_underflow
        .create_body(body(f64::from_bits(1), 1.0))
        .expect("cold");
    heat_underflow
        .create_link(ConductiveLinkSpec::new(hot, cold, conductance(1.0)))
        .expect("link");
    let before = heat_underflow.clone();
    assert!(matches!(
        heat_underflow.step(TimeStep::new(0.5).expect("dt")),
        Err(ThermalError::PrecisionLoss {
            operation: ThermalArithmetic::TransferredHeat,
            ..
        })
    ));
    assert_eq!(heat_underflow, before);
}

#[test]
fn elapsed_overflow_and_absorbed_progress_reject_atomically() {
    let mut overflow = ThermalWorld::new();
    overflow
        .step(TimeStep::new(f64::MAX).expect("maximum finite dt"))
        .expect("first empty-world step");
    let before = overflow.clone();
    assert!(matches!(
        overflow.step(TimeStep::new(f64::MAX).expect("maximum finite dt")),
        Err(ThermalError::DerivedNonFinite {
            operation: ThermalArithmetic::ElapsedTime,
            ..
        })
    ));
    assert_eq!(overflow, before);

    let mut absorbed = ThermalWorld::new();
    absorbed
        .step(TimeStep::new(2.0_f64.powi(53)).expect("large representable dt"))
        .expect("first empty-world step");
    let before = absorbed.clone();
    assert!(matches!(
        absorbed.step(TimeStep::new(1.0).expect("positive dt")),
        Err(ThermalError::PrecisionLoss {
            operation: ThermalArithmetic::ElapsedTime,
            ..
        })
    ));
    assert_eq!(absorbed, before);
}

#[test]
fn conservative_pair_absorption_commits_as_explicit_numerical_settlement() {
    let mut world = ThermalWorld::new();
    let hot = world.create_body(body(1.0e16, 1.0)).expect("hot body");
    let cold = world
        .create_body(body(1.0e16 - 2.0, 1.0))
        .expect("cold body");
    world
        .create_link(ConductiveLinkSpec::new(hot, cold, conductance(0.5)))
        .expect("link");
    let before = world.snapshot().expect("before");

    let report = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("absorbed pair transfer settles");

    assert_eq!(report.snapshot.bodies[0].energy, before.bodies[0].energy);
    assert_eq!(report.snapshot.bodies[1].energy, before.bodies[1].energy);
    assert_eq!(report.snapshot.links[0].first_to_second_power.get(), 0.0);
    assert!(
        report
            .snapshot
            .bodies
            .iter()
            .all(|body| body.net_heat_power.get() == 0.0)
    );
}

#[test]
fn exact_body_reduction_cancels_large_incoming_and_outgoing_heat_before_update() {
    let mut world = ThermalWorld::new();
    let hot = world
        .create_body(body(3.0, f64::MAX / 4.0))
        .expect("hot body");
    let center = world
        .create_body(body(2.0, f64::MAX / 2.0))
        .expect("center body");
    let cold = world
        .create_body(body(1.0, f64::MAX / 4.0))
        .expect("cold body");
    let large_conductance = conductance(f64::MAX / 7.0);
    world
        .create_link(ConductiveLinkSpec::new(hot, center, large_conductance))
        .expect("incoming link");
    world
        .create_link(ConductiveLinkSpec::new(center, cold, large_conductance))
        .expect("outgoing link");
    let before = world.snapshot().expect("before");

    let report = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("exact center cancellation avoids an intermediate overflow");

    assert!(report.snapshot.bodies[0].energy.get() < before.bodies[0].energy.get());
    assert_eq!(report.snapshot.bodies[1].energy, before.bodies[1].energy);
    assert!(report.snapshot.bodies[2].energy.get() > before.bodies[2].energy.get());
    assert!(
        report
            .snapshot
            .links
            .iter()
            .all(|link| link.first_to_second_power.get() > 0.0)
    );
    assert_eq!(report.snapshot.total_energy, before.total_energy);
}

#[test]
fn absorbed_endpoint_settles_link_before_other_endpoint_overflow_is_fatal() {
    let mut world = ThermalWorld::new();
    let cold = world.create_body(body(1.0, f64::MAX)).expect("cold body");
    let predecessor = f64::from_bits(f64::MAX.to_bits() - 1);
    let hot = world
        .create_body(body(2.0, predecessor / 2.0))
        .expect("hot body");
    world
        .create_link(ConductiveLinkSpec::new(
            cold,
            hot,
            conductance(2.0_f64.powi(970)),
        ))
        .expect("link");
    let before = world.snapshot().expect("before");

    let report = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("absorbed hot endpoint settles the complete link");

    assert_eq!(report.snapshot.bodies, before.bodies);
    assert_eq!(report.snapshot.links[0].first_to_second_power.get(), 0.0);
    assert_eq!(report.snapshot.total_energy, before.total_energy);
}

#[test]
fn link_identity_order_does_not_change_body_results() {
    fn build(reverse: bool) -> ThermalWorld {
        let mut world = ThermalWorld::new();
        let hot = world.create_body(body(500.0, 100.0)).expect("hot");
        let middle = world.create_body(body(350.0, 80.0)).expect("middle");
        let cold = world.create_body(body(250.0, 120.0)).expect("cold");
        let links = [
            ConductiveLinkSpec::new(hot, middle, conductance(4.0)),
            ConductiveLinkSpec::new(middle, cold, conductance(7.0)),
        ];
        for index in if reverse { [1, 0] } else { [0, 1] } {
            world.create_link(links[index]).expect("link");
        }
        world
    }

    let dt = TimeStep::new(0.5).expect("dt");
    let first = build(false).step(dt).expect("first step").snapshot;
    let second = build(true).step(dt).expect("second step").snapshot;
    let first_temperatures: Vec<u64> = first
        .bodies
        .iter()
        .map(|item| item.temperature.get().to_bits())
        .collect();
    let second_temperatures: Vec<u64> = second
        .bodies
        .iter()
        .map(|item| item.temperature.get().to_bits())
        .collect();
    assert_eq!(first_temperatures, second_temperatures);
}

#[test]
fn snapshot_is_read_only_and_ordered() {
    let mut world = ThermalWorld::new();
    world.create_body(body(320.0, 10.0)).expect("body one");
    world.create_body(body(280.0, 10.0)).expect("body two");
    let first = world.snapshot().expect("first snapshot");
    let second = world.snapshot().expect("second snapshot");
    assert_eq!(first, second);
    assert!(
        first
            .bodies
            .windows(2)
            .all(|pair| pair[0].entity < pair[1].entity)
    );
}

#[test]
fn snapshot_reports_a_positive_total_above_binary64_without_faking_infinity() {
    let mut world = ThermalWorld::new();
    world
        .create_body(body(1.0, f64::MAX))
        .expect("first maximum-energy body");
    world
        .create_body(body(1.0, f64::MAX))
        .expect("second maximum-energy body");

    let snapshot = world.snapshot().expect("bounded exact-sum outcome");

    assert_eq!(
        snapshot.total_energy,
        ThermalEnergyTotal::AboveBinary64Range
    );
}

#[test]
fn exact_capacity_world_has_bounded_ordered_snapshot_and_event_stream() {
    let mut world = ThermalWorld::new();
    let mut bodies = Vec::with_capacity(MAX_THERMAL_BODIES);
    for _ in 0..MAX_THERMAL_BODIES {
        bodies.push(
            world
                .create_body(body(300.0, 1.0))
                .expect("body within capacity"),
        );
    }
    let before_body_failure = world.clone();
    assert!(matches!(
        world.create_body(body(300.0, 1.0)),
        Err(ThermalError::BodyCapacityReached {
            maximum: MAX_THERMAL_BODIES
        })
    ));
    assert_eq!(world, before_body_failure);

    let mut created_links = 0;
    'pairs: for first in 0..bodies.len() {
        for second in (first + 1)..bodies.len() {
            world
                .create_link(ConductiveLinkSpec::new(
                    bodies[first],
                    bodies[second],
                    conductance(1.0e-6),
                ))
                .expect("link within capacity");
            created_links += 1;
            if created_links == MAX_THERMAL_LINKS {
                break 'pairs;
            }
        }
    }
    assert_eq!(created_links, MAX_THERMAL_LINKS);
    let before_link_failure = world.clone();
    assert!(matches!(
        world.create_link(ConductiveLinkSpec::new(
            bodies[0],
            bodies[1],
            conductance(1.0e-6),
        )),
        Err(ThermalError::LinkCapacityReached {
            maximum: MAX_THERMAL_LINKS
        })
    ));
    assert_eq!(world, before_link_failure);

    let snapshot = world.snapshot().expect("maximum bounded snapshot");
    assert_eq!(snapshot.bodies.len(), MAX_THERMAL_BODIES);
    assert_eq!(snapshot.links.len(), MAX_THERMAL_LINKS);
    assert!(
        snapshot
            .bodies
            .windows(2)
            .all(|pair| pair[0].entity < pair[1].entity)
    );
    assert!(
        snapshot
            .links
            .windows(2)
            .all(|pair| pair[0].link < pair[1].link)
    );

    let report = world
        .step(TimeStep::new(1.0).expect("stable dt"))
        .expect("maximum bounded step");
    assert_eq!(report.events.len(), MAX_THERMAL_EVENTS);
    assert_eq!(report.snapshot.bodies.len(), MAX_THERMAL_BODIES);
    assert_eq!(report.snapshot.links.len(), MAX_THERMAL_LINKS);
    assert!(
        report.events[..MAX_THERMAL_LINKS]
            .iter()
            .all(|event| matches!(event.kind, ThermalEventKind::HeatTransferred { .. }))
    );
    assert!(
        report.events[MAX_THERMAL_LINKS..MAX_THERMAL_LINKS + MAX_THERMAL_BODIES]
            .iter()
            .all(|event| matches!(event.kind, ThermalEventKind::BodyTemperatureChanged { .. }))
    );
    assert!(matches!(
        report.events.last().map(|event| &event.kind),
        Some(ThermalEventKind::StepCommitted {
            bodies: MAX_THERMAL_BODIES,
            links: MAX_THERMAL_LINKS,
        })
    ));
}

#[test]
fn repeated_worlds_produce_identical_reports_and_canonical_state() {
    fn build() -> ThermalWorld {
        let mut world = ThermalWorld::new();
        let hot = world.create_body(body(450.0, 30.0)).expect("hot");
        let middle = world.create_body(body(330.0, 20.0)).expect("middle");
        let cold = world.create_body(body(270.0, 40.0)).expect("cold");
        world
            .create_link(ConductiveLinkSpec::new(hot, middle, conductance(2.0)))
            .expect("first link");
        world
            .create_link(ConductiveLinkSpec::new(middle, cold, conductance(1.5)))
            .expect("second link");
        world
    }

    let mut first = build();
    let mut second = build();
    let dt = TimeStep::new(0.25).expect("dt");
    for _ in 0..128 {
        assert_eq!(first.step(dt), second.step(dt));
    }
    assert_eq!(first, second);
    assert_eq!(first.snapshot(), second.snapshot());
}

#[test]
fn primary_project_settles_without_a_precision_fault() {
    let mut world = ThermalWorld::new();
    let hot = world.create_body(body(420.0, 20.0)).expect("hot body");
    let cold = world.create_body(body(280.0, 20.0)).expect("cold body");
    world
        .create_link(ConductiveLinkSpec::new(hot, cold, conductance(5.0)))
        .expect("thermal link");
    let dt = TimeStep::new(1.0 / 120.0).expect("fixed step");

    let mut last = None;
    for _ in 0..8_000 {
        last = Some(world.step(dt).expect("standard project must keep running"));
    }
    let snapshot = last.expect("at least one step").snapshot;

    assert_eq!(snapshot.links[0].first_to_second_power.get(), 0.0);
    assert_eq!(snapshot.bodies[0].net_heat_power.get(), 0.0);
    assert_eq!(snapshot.bodies[1].net_heat_power.get(), 0.0);
    assert!(snapshot.bodies[0].temperature.get() >= snapshot.bodies[1].temperature.get());
}
