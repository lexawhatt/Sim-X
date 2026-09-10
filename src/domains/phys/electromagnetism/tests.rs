use crate::foundation::{
    ConstantsError, ConstantsSource, Meters, PhysicalConstants, VacuumPermittivity,
};

use super::{
    Coulombs, ElectrostaticArithmetic, ElectrostaticError, ElectrostaticWorld, MAX_CHARGES,
    MAX_ELECTROSTATIC_INTERACTIONS, MAX_FIELD_PROBES, PointChargeSpec, Position2,
};

fn position(x: f64, y: f64) -> Position2 {
    Position2::new(
        Meters::new(x).expect("valid test X"),
        Meters::new(y).expect("valid test Y"),
    )
}

fn charge(x: f64, y: f64, coulombs: f64) -> PointChargeSpec {
    PointChargeSpec::new(
        position(x, y),
        Coulombs::new(coulombs).expect("valid test charge"),
    )
}

#[test]
fn charge_quantity_rejects_zero_and_non_finite_values() {
    for invalid in [0.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Coulombs::new(invalid).is_err());
    }
    assert_eq!(Coulombs::new(-2.0).expect("negative charge").get(), -2.0);
}

#[test]
fn positions_and_constants_reject_non_finite_or_non_positive_inputs() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Meters::new(invalid).is_err());
        assert!(matches!(
            VacuumPermittivity::new(invalid),
            Err(ConstantsError::NonFiniteVacuumPermittivity)
        ));
    }
    for invalid in [0.0, -0.0, -1.0] {
        assert!(matches!(
            VacuumPermittivity::new(invalid),
            Err(ConstantsError::NonPositiveVacuumPermittivity)
        ));
    }
}

#[test]
fn positive_and_negative_charges_point_field_in_expected_direction() {
    let constants = PhysicalConstants::codata_2022();
    let mut positive = ElectrostaticWorld::new();
    positive
        .create_charge(charge(0.0, 0.0, 1.0e-9))
        .expect("positive charge");
    positive.create_probe(position(1.0, 0.0)).expect("probe");
    let positive_field = positive.evaluate(&constants).expect("field").probes[0].field;
    assert!(positive_field.x().get() > 0.0);
    assert_eq!(positive_field.y().get(), 0.0);

    let mut negative = ElectrostaticWorld::new();
    negative
        .create_charge(charge(0.0, 0.0, -1.0e-9))
        .expect("negative charge");
    negative.create_probe(position(1.0, 0.0)).expect("probe");
    let negative_field = negative.evaluate(&constants).expect("field").probes[0].field;
    assert!(negative_field.x().get() < 0.0);
    assert_eq!(negative_field.y().get(), 0.0);
}

#[test]
fn field_magnitude_obeys_inverse_square_ratio() {
    let constants = PhysicalConstants::codata_2022();
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(0.0, 0.0, 1.0e-9))
        .expect("charge");
    world.create_probe(position(1.0, 0.0)).expect("near");
    world.create_probe(position(2.0, 0.0)).expect("far");
    let snapshot = world.evaluate(&constants).expect("field");
    let ratio = snapshot.probes[0].magnitude.get() / snapshot.probes[1].magnitude.get();
    assert!((ratio - 4.0).abs() < 1.0e-12);
}

#[test]
fn dipole_field_has_axis_symmetry() {
    let constants = PhysicalConstants::codata_2022();
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(-1.0, 0.0, 1.0e-9))
        .expect("positive");
    world
        .create_charge(charge(1.0, 0.0, -1.0e-9))
        .expect("negative");
    world.create_probe(position(0.0, 2.0)).expect("upper");
    world.create_probe(position(0.0, -2.0)).expect("lower");
    let snapshot = world.evaluate(&constants).expect("dipole field");
    assert_eq!(
        snapshot.probes[0].field.x().get().to_bits(),
        snapshot.probes[1].field.x().get().to_bits()
    );
    assert_eq!(snapshot.probes[0].field.y().get(), 0.0);
    assert_eq!(snapshot.probes[1].field.y().get(), 0.0);
}

#[test]
fn coincident_probe_rejects_pure_evaluation_without_mutation() {
    let constants = PhysicalConstants::codata_2022();
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(1.0, 2.0, 1.0e-9))
        .expect("charge");
    world.create_probe(position(1.0, 2.0)).expect("probe");
    let before = world.clone();
    assert!(matches!(
        world.evaluate(&constants),
        Err(ElectrostaticError::SingularProbe { .. })
    ));
    assert_eq!(world, before);
}

#[test]
fn charge_identity_order_does_not_change_field_result() {
    fn build(reverse: bool) -> ElectrostaticWorld {
        let mut world = ElectrostaticWorld::new();
        let charges = [charge(-1.0, 0.5, 2.0e-9), charge(2.0, -0.5, -0.8e-9)];
        for index in if reverse { [1, 0] } else { [0, 1] } {
            world.create_charge(charges[index]).expect("charge");
        }
        world.create_probe(position(0.25, 2.0)).expect("probe");
        world
    }
    let constants = PhysicalConstants::codata_2022();
    let first = build(false).evaluate(&constants).expect("first");
    let second = build(true).evaluate(&constants).expect("second");
    assert_eq!(
        first.probes[0].field.x().get().to_bits(),
        second.probes[0].field.x().get().to_bits()
    );
    assert_eq!(
        first.probes[0].field.y().get().to_bits(),
        second.probes[0].field.y().get().to_bits()
    );
}

#[test]
fn evaluation_budget_is_checked_before_pair_work() {
    let constants =
        PhysicalConstants::custom(VacuumPermittivity::new(1.0e-11).expect("custom constant"));
    assert_eq!(constants.source(), ConstantsSource::Custom);
    let mut world = ElectrostaticWorld::new();
    for index in 0..513 {
        world
            .create_charge(charge(index as f64 + 1.0, 0.0, 1.0e-12))
            .expect("bounded charge");
    }
    for index in 0..512 {
        world
            .create_probe(position(0.0, index as f64 + 1.0))
            .expect("bounded probe");
    }
    let interactions = 513 * 512;
    assert!(interactions > MAX_ELECTROSTATIC_INTERACTIONS);
    assert!(matches!(
        world.evaluate(&constants),
        Err(ElectrostaticError::InteractionBudgetExceeded { .. })
    ));
}

#[test]
fn charge_and_probe_capacity_failures_are_atomic() {
    let constants = PhysicalConstants::codata_2022();
    let mut charge_world = ElectrostaticWorld::new();
    for index in 0..MAX_CHARGES {
        charge_world
            .create_charge(charge(index as f64 + 1.0, 0.0, 1.0e-20))
            .expect("charge within capacity");
    }
    let charge_world_before = charge_world.clone();
    let charge_snapshot_before = charge_world.evaluate(&constants).expect("bounded snapshot");
    assert!(matches!(
        charge_world.create_charge(charge(-1.0, 0.0, 1.0e-20)),
        Err(ElectrostaticError::ChargeCapacityReached {
            maximum: MAX_CHARGES
        })
    ));
    assert_eq!(charge_world, charge_world_before);
    assert_eq!(
        charge_world
            .evaluate(&constants)
            .expect("unchanged snapshot"),
        charge_snapshot_before
    );
    assert_eq!(charge_snapshot_before.charges.len(), MAX_CHARGES);
    assert!(
        charge_snapshot_before
            .charges
            .windows(2)
            .all(|pair| pair[0].entity < pair[1].entity)
    );

    let mut probe_world = ElectrostaticWorld::new();
    for index in 0..MAX_FIELD_PROBES {
        probe_world
            .create_probe(position(index as f64 + 1.0, 0.0))
            .expect("probe within capacity");
    }
    let probe_world_before = probe_world.clone();
    let probe_snapshot_before = probe_world.evaluate(&constants).expect("bounded snapshot");
    assert!(matches!(
        probe_world.create_probe(position(-1.0, 0.0)),
        Err(ElectrostaticError::ProbeCapacityReached {
            maximum: MAX_FIELD_PROBES
        })
    ));
    assert_eq!(probe_world, probe_world_before);
    assert_eq!(
        probe_world
            .evaluate(&constants)
            .expect("unchanged snapshot"),
        probe_snapshot_before
    );
    assert_eq!(probe_snapshot_before.probes.len(), MAX_FIELD_PROBES);
    assert!(
        probe_snapshot_before
            .probes
            .windows(2)
            .all(|pair| pair[0].probe < pair[1].probe)
    );
}

#[test]
fn exact_interaction_budget_produces_a_complete_bounded_snapshot() {
    const SIDE: usize = 512;
    assert_eq!(SIDE * SIDE, MAX_ELECTROSTATIC_INTERACTIONS);

    let constants = PhysicalConstants::codata_2022();
    let mut world = ElectrostaticWorld::new();
    for index in 0..SIDE {
        world
            .create_charge(charge(10_000.0 + index as f64, 0.0, 1.0e-20))
            .expect("charge within work budget");
        world
            .create_probe(position(-10_000.0, index as f64 + 1.0))
            .expect("probe within work budget");
    }

    let before = world.clone();
    let first = world
        .evaluate(&constants)
        .expect("exact budget is accepted");
    let second = world.evaluate(&constants).expect("repeat evaluation");

    assert_eq!(world, before);
    assert_eq!(first, second);
    assert_eq!(first.charges.len(), SIDE);
    assert_eq!(first.probes.len(), SIDE);
}

#[test]
fn snapshots_are_pure_ordered_and_repeatable() {
    let constants = PhysicalConstants::codata_2022();
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(-1.0, 0.0, 1.0e-9))
        .expect("charge");
    world.create_probe(position(1.0, 1.0)).expect("probe");
    let first = world.evaluate(&constants).expect("first");
    let second = world.evaluate(&constants).expect("second");
    assert_eq!(first, second);
}

#[test]
fn finite_large_field_uses_an_overflow_safe_magnitude() {
    let constants = PhysicalConstants::codata_2022();
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(0.0, 0.0, 1.0e298))
        .expect("large finite charge");
    world.create_probe(position(1.0, 0.0)).expect("probe");

    let probe = &world.evaluate(&constants).expect("finite field").probes[0];
    assert!(probe.field.x().get().is_finite());
    assert!(probe.field.x().get().abs() > f64::MAX.sqrt());
    assert_eq!(
        probe.magnitude.get().to_bits(),
        probe.field.x().get().abs().to_bits()
    );
}

#[test]
fn snapshot_audits_the_exact_custom_constant() {
    let permittivity = VacuumPermittivity::new(2.5e-11).expect("custom constant");
    let constants = PhysicalConstants::custom(permittivity);
    let snapshot = ElectrostaticWorld::new()
        .evaluate(&constants)
        .expect("empty snapshot");

    assert_eq!(snapshot.constants_source, ConstantsSource::Custom);
    assert_eq!(snapshot.vacuum_permittivity, permittivity);
}

#[test]
fn empty_evaluation_skips_unused_coulomb_arithmetic() {
    let smallest_positive =
        VacuumPermittivity::new(f64::from_bits(1)).expect("positive subnormal permittivity");
    let constants = PhysicalConstants::custom(smallest_positive);

    let snapshot = ElectrostaticWorld::new()
        .evaluate(&constants)
        .expect("no interactions need a Coulomb constant");

    assert!(snapshot.charges.is_empty());
    assert!(snapshot.probes.is_empty());
    assert_eq!(snapshot.vacuum_permittivity, smallest_positive);
}

#[test]
fn symmetric_maximum_fields_cancel_without_intermediate_overflow() {
    let unit_coulomb_constant =
        VacuumPermittivity::new(1.0 / (4.0 * std::f64::consts::PI)).expect("finite permittivity");
    let constants = PhysicalConstants::custom(unit_coulomb_constant);
    let mut world = ElectrostaticWorld::new();
    for value in [
        f64::MAX / 2.0,
        f64::MAX / 2.0,
        f64::MAX / 2.0,
        -f64::MAX / 2.0,
        -f64::MAX / 2.0,
        -f64::MAX / 2.0,
    ] {
        world
            .create_charge(charge(0.0, 0.0, value))
            .expect("finite charge");
    }
    world.create_probe(position(1.0, 0.0)).expect("probe");

    let snapshot = world.evaluate(&constants).expect("exact cancellation");

    assert_eq!(snapshot.probes[0].field, super::ElectricField2::ZERO);
    assert_eq!(snapshot.probes[0].magnitude.get(), 0.0);
}

#[test]
fn derived_overflow_and_underflow_are_structured_and_pure() {
    let regular_constants = PhysicalConstants::codata_2022();

    let mut extended_distance_underflow = ElectrostaticWorld::new();
    extended_distance_underflow
        .create_charge(charge(-f64::MAX, 0.0, 1.0))
        .expect("charge");
    extended_distance_underflow
        .create_probe(position(f64::MAX, 0.0))
        .expect("probe");
    let before = extended_distance_underflow.clone();
    assert!(matches!(
        extended_distance_underflow.evaluate(&regular_constants),
        Err(ElectrostaticError::PrecisionLoss {
            operation: ElectrostaticArithmetic::FieldScale,
            ..
        })
    ));
    assert_eq!(extended_distance_underflow, before);

    let mut near_field_overflow = ElectrostaticWorld::new();
    near_field_overflow
        .create_charge(charge(0.0, 0.0, 1.0))
        .expect("charge");
    near_field_overflow
        .create_probe(position(f64::from_bits(1), 0.0))
        .expect("probe");
    let before = near_field_overflow.clone();
    assert!(matches!(
        near_field_overflow.evaluate(&regular_constants),
        Err(ElectrostaticError::DerivedNonFinite {
            operation: ElectrostaticArithmetic::FieldScale,
            ..
        })
    ));
    assert_eq!(near_field_overflow, before);

    let mut field_scale_underflow = ElectrostaticWorld::new();
    field_scale_underflow
        .create_charge(charge(0.0, 0.0, f64::from_bits(1)))
        .expect("charge");
    field_scale_underflow
        .create_probe(position(1.0e100, 0.0))
        .expect("probe");
    let before = field_scale_underflow.clone();
    assert!(matches!(
        field_scale_underflow.evaluate(&regular_constants),
        Err(ElectrostaticError::PrecisionLoss {
            operation: ElectrostaticArithmetic::FieldScale,
            ..
        })
    ));
    assert_eq!(field_scale_underflow, before);
}

#[test]
fn extreme_constants_and_vector_reductions_preserve_final_range_semantics() {
    let mut one_pair = ElectrostaticWorld::new();
    one_pair
        .create_charge(charge(0.0, 0.0, 1.0))
        .expect("charge");
    one_pair.create_probe(position(1.0, 0.0)).expect("probe");
    let maximum_permittivity =
        PhysicalConstants::custom(VacuumPermittivity::new(f64::MAX).expect("finite permittivity"));
    let maximum_field = one_pair
        .evaluate(&maximum_permittivity)
        .expect("finite field despite overflowing associated denominator")
        .probes[0]
        .field
        .x()
        .get();
    assert!(maximum_field.is_finite() && maximum_field > 0.0);
    let minimum_permittivity = PhysicalConstants::custom(
        VacuumPermittivity::new(f64::from_bits(1)).expect("positive permittivity"),
    );
    assert!(matches!(
        one_pair.evaluate(&minimum_permittivity),
        Err(ElectrostaticError::DerivedNonFinite {
            operation: ElectrostaticArithmetic::FieldScale,
            ..
        })
    ));

    let unit_constants = PhysicalConstants::custom(
        VacuumPermittivity::new(1.0 / (4.0 * std::f64::consts::PI)).expect("unit Coulomb constant"),
    );
    let mut reduction_overflow = ElectrostaticWorld::new();
    for _ in 0..3 {
        reduction_overflow
            .create_charge(charge(0.0, 0.0, f64::MAX / 2.0))
            .expect("charge");
    }
    reduction_overflow
        .create_probe(position(1.0, 0.0))
        .expect("probe");
    assert!(matches!(
        reduction_overflow.evaluate(&unit_constants),
        Err(ElectrostaticError::DerivedNonFinite {
            operation: ElectrostaticArithmetic::FieldReduction,
            ..
        })
    ));

    let mut magnitude_overflow = ElectrostaticWorld::new();
    magnitude_overflow
        .create_charge(charge(-1.0, 0.0, 1.3e308))
        .expect("X charge");
    magnitude_overflow
        .create_charge(charge(0.0, -1.0, 1.3e308))
        .expect("Y charge");
    magnitude_overflow
        .create_probe(position(0.0, 0.0))
        .expect("probe");
    assert!(matches!(
        magnitude_overflow.evaluate(&unit_constants),
        Err(ElectrostaticError::DerivedNonFinite {
            operation: ElectrostaticArithmetic::FieldMagnitude,
            ..
        })
    ));
}

#[test]
fn scaled_inverse_square_accepts_huge_charge_over_huge_distance() {
    let constants = PhysicalConstants::custom(
        VacuumPermittivity::new(1.0 / (4.0 * std::f64::consts::PI)).expect("unit field factor"),
    );
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(0.0, 0.0, 2.0_f64.powi(1_023)))
        .expect("charge");
    world
        .create_probe(position(2.0_f64.powi(511), 0.0))
        .expect("probe");

    let probe = &world.evaluate(&constants).expect("finite field").probes[0];
    assert!((probe.field.x().get() - 2.0).abs() <= 2.0 * f64::EPSILON);
    assert_eq!(probe.field.y().get(), 0.0);
    assert_eq!(probe.magnitude.get(), probe.field.x().get());
}

#[test]
fn normalized_direction_does_not_require_euclidean_distance_to_fit() {
    let constants = PhysicalConstants::custom(
        VacuumPermittivity::new(1.0 / (4.0 * std::f64::consts::PI)).expect("unit field factor"),
    );
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(0.0, 0.0, f64::MAX))
        .expect("charge");
    world
        .create_probe(position(f64::MAX, f64::MAX))
        .expect("probe");

    let probe = &world
        .evaluate(&constants)
        .expect("finite field without a representable distance")
        .probes[0];
    assert!(probe.field.x().get().is_finite() && probe.field.x().get() > 0.0);
    assert_eq!(
        probe.field.x().get().to_bits(),
        probe.field.y().get().to_bits()
    );
    assert!(probe.magnitude.get().is_finite() && probe.magnitude.get() > 0.0);
}

#[test]
fn scaled_separation_accepts_opposite_maximum_coordinates() {
    let constants = PhysicalConstants::custom(
        VacuumPermittivity::new(1.0 / (4.0 * std::f64::consts::PI)).expect("unit field factor"),
    );
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(-f64::MAX, 0.0, f64::MAX))
        .expect("charge");
    world.create_probe(position(f64::MAX, 0.0)).expect("probe");

    let probe = &world
        .evaluate(&constants)
        .expect("finite field across an extended separation")
        .probes[0];
    let expected = 1.390_671_161_567e-309;
    assert!((probe.field.x().get() - expected).abs() <= 2.0 * f64::from_bits(1));
    assert_eq!(probe.field.y().get(), 0.0);
    assert_eq!(probe.magnitude.get(), probe.field.x().get());
}

#[test]
fn scaled_field_accepts_compensating_minimum_charge_and_permittivity() {
    let smallest = f64::from_bits(1);
    let constants = PhysicalConstants::custom(
        VacuumPermittivity::new(smallest).expect("minimum positive permittivity"),
    );
    let mut world = ElectrostaticWorld::new();
    world
        .create_charge(charge(0.0, 0.0, smallest))
        .expect("minimum charge");
    world.create_probe(position(1.0, 0.0)).expect("probe");

    let probe = &world.evaluate(&constants).expect("finite field").probes[0];
    let expected = 1.0 / (4.0 * std::f64::consts::PI);
    assert!((probe.field.x().get() - expected).abs() <= expected * f64::EPSILON);
    assert_eq!(probe.field.y().get(), 0.0);
    assert_eq!(probe.magnitude.get(), probe.field.x().get());
}
