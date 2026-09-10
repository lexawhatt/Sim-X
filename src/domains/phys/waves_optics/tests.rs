use crate::foundation::{Meters, MetersPerSecond, TimeStep};

use super::{
    BoundaryCondition, MAX_WAVE_SAMPLES, SampleSpacing, WaveArithmetic, WaveError, WaveSpec,
    WaveSpeed, WaveWorld,
};

fn field(displacement: &[f64], speed: f64, spacing: f64) -> WaveSpec {
    field_with_velocity(displacement, &vec![0.0; displacement.len()], speed, spacing)
}

fn field_with_velocity(
    displacement: &[f64],
    velocity: &[f64],
    speed: f64,
    spacing: f64,
) -> WaveSpec {
    WaveSpec::fixed_zero(
        SampleSpacing::new(spacing).expect("spacing"),
        WaveSpeed::new(speed).expect("speed"),
        displacement
            .iter()
            .copied()
            .map(|value| Meters::new(value).expect("displacement"))
            .collect(),
        velocity
            .iter()
            .copied()
            .map(|value| MetersPerSecond::new(value).expect("velocity"))
            .collect(),
    )
}

#[test]
fn positive_wave_quantities_reject_invalid_values() {
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(SampleSpacing::new(invalid).is_err());
        assert!(WaveSpeed::new(invalid).is_err());
    }
}

#[test]
fn non_finite_samples_are_rejected_before_a_spec_can_be_formed() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Meters::new(invalid).is_err());
        assert!(MetersPerSecond::new(invalid).is_err());
    }
}

#[test]
fn construction_rejects_bad_counts_and_fixed_endpoints() {
    assert!(matches!(
        WaveWorld::new(field(&[0.0, 0.0], 1.0, 1.0)),
        Err(WaveError::SampleCountOutOfRange { .. })
    ));
    let oversized = vec![0.0; MAX_WAVE_SAMPLES + 1];
    assert!(matches!(
        WaveWorld::new(field(&oversized, 1.0, 1.0)),
        Err(WaveError::SampleCountOutOfRange { .. })
    ));
    assert!(matches!(
        WaveWorld::new(field(&[1.0, 0.0, 0.0], 1.0, 1.0)),
        Err(WaveError::InvalidFixedEndpoint { index: 0 })
    ));

    let mismatched = WaveSpec::fixed_zero(
        SampleSpacing::new(1.0).expect("spacing"),
        WaveSpeed::new(1.0).expect("speed"),
        vec![Meters::ZERO; 4],
        vec![MetersPerSecond::ZERO; 3],
    );
    assert_eq!(
        WaveWorld::new(mismatched),
        Err(WaveError::SampleCountMismatch {
            displacement: 4,
            velocity: 3,
        })
    );

    assert!(matches!(
        WaveWorld::new(field_with_velocity(
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, 1.0],
            1.0,
            1.0,
        )),
        Err(WaveError::InvalidFixedEndpoint { index: 2 })
    ));
}

#[test]
fn zero_field_remains_exactly_zero() {
    let mut world = WaveWorld::new(field(&[0.0; 9], 2.0, 1.0)).expect("field");
    let report = world
        .step(TimeStep::new(0.25).expect("dt"))
        .expect("stable step");
    assert!(report.snapshot.samples.iter().all(|sample| {
        sample.displacement == Meters::ZERO
            && sample.velocity == MetersPerSecond::ZERO
            && sample.acceleration.get() == 0.0
    }));
}

#[test]
fn centered_pulse_evolves_symmetrically_with_fixed_endpoints() {
    let mut world = WaveWorld::new(field(&[0.0, 0.0, 1.0, 0.0, 0.0], 1.0, 1.0)).expect("field");
    let snapshot = world
        .step(TimeStep::new(0.5).expect("dt"))
        .expect("step")
        .snapshot;
    assert_eq!(snapshot.samples[0].displacement.get(), 0.0);
    assert_eq!(snapshot.samples[4].displacement.get(), 0.0);
    assert_eq!(
        snapshot.samples[1].displacement.get().to_bits(),
        snapshot.samples[3].displacement.get().to_bits()
    );
    assert_eq!(snapshot.samples[1].displacement.get(), 0.25);
    assert_eq!(snapshot.samples[2].displacement.get(), 0.5);
}

#[test]
fn courant_limit_is_enforced_atomically() {
    let mut world = WaveWorld::new(field(&[0.0, 0.0, 1.0, 0.0, 0.0], 2.0, 1.0)).expect("field");
    let before = world.clone();
    assert!(matches!(
        world.step(TimeStep::new(0.500_000_1).expect("dt")),
        Err(WaveError::CourantLimitExceeded { .. })
    ));
    assert_eq!(world, before);

    world
        .step(TimeStep::new(0.5).expect("limit dt"))
        .expect("Courant one is valid");
}

#[test]
fn failure_at_a_later_sample_discards_all_candidate_updates() {
    let mut world =
        WaveWorld::new(field(&[0.0, 0.0, 0.0, f64::MAX, 0.0], 1.0, 1.0)).expect("field");
    let before = world.clone();

    assert_eq!(
        world.step(TimeStep::new(1.0).expect("dt")),
        Err(WaveError::DerivedNonFinite {
            operation: WaveArithmetic::StencilReduction,
            sample: Some(3),
        })
    );
    assert_eq!(world, before);
}

#[test]
fn equal_tiny_spacing_and_speed_avoid_independent_square_underflow() {
    let smallest_normal = f64::MIN_POSITIVE;
    let mut world =
        WaveWorld::new(field(&[0.0, 0.0, 0.0], smallest_normal, smallest_normal)).expect("field");

    let report = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("finite speed/spacing ratio");

    assert!(report.snapshot.samples.iter().all(|sample| {
        sample.displacement.get() == 0.0
            && sample.velocity.get() == 0.0
            && sample.acceleration.get() == 0.0
    }));
}

#[test]
fn courant_preserves_multiply_divide_compensation() {
    let tiny = 2.0_f64.powi(-600);
    let mut world = WaveWorld::new(field(&[0.0, 0.0, 0.0], tiny, tiny)).expect("field");

    world
        .step(TimeStep::new(tiny).expect("dt"))
        .expect("finite compensated Courant number");
}

#[test]
fn exact_stencil_cancels_maximum_terms_before_rounding() {
    let mut world =
        WaveWorld::new(field(&[0.0, f64::MAX, f64::MAX, f64::MAX, 0.0], 1.0, 1.0)).expect("field");

    let snapshot = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("exact stencil")
        .snapshot;

    let displacement: Vec<_> = snapshot
        .samples
        .iter()
        .map(|sample| sample.displacement.get())
        .collect();
    assert_eq!(displacement, [0.0, 0.0, f64::MAX, 0.0, 0.0]);
}

#[test]
fn scaled_acceleration_avoids_independent_square_overflow() {
    let scale = 2.0_f64.powi(512);
    let mut world = WaveWorld::new(field(&[0.0, 1.0, 0.0], scale, scale)).expect("field");

    let center = world
        .step(TimeStep::new(1.0).expect("dt"))
        .expect("finite wave coefficient")
        .snapshot
        .samples[1]
        .clone();

    assert_eq!(center.acceleration.get(), -2.0);
    assert_eq!(center.velocity.get(), -2.0);
    assert_eq!(center.displacement.get(), -1.0);
}

#[test]
fn scaled_acceleration_preserves_compensated_underflow_with_large_time_step() {
    let initial = -2.0_f64.powi(999);
    let expected_final = 2.0_f64.powi(999);
    let cases = [
        (
            2.0_f64.powi(-600),
            1.0,
            2.0_f64.powi(600),
            2.0_f64.powi(-200),
            2.0_f64.powi(400),
        ),
        (
            2.0_f64.powi(-100),
            2.0_f64.powi(600),
            2.0_f64.powi(700),
            2.0_f64.powi(-400),
            2.0_f64.powi(300),
        ),
    ];

    for (speed, spacing, dt, expected_acceleration, expected_velocity) in cases {
        let mut world = WaveWorld::new(field(&[0.0, initial, 0.0], speed, spacing)).expect("field");

        let center = world
            .step(TimeStep::new(dt).expect("dt"))
            .expect("compensated finite step")
            .snapshot
            .samples[1]
            .clone();

        assert_eq!(center.acceleration.get(), expected_acceleration);
        assert_eq!(center.velocity.get(), expected_velocity);
        assert_eq!(center.displacement.get(), expected_final);
    }
}

#[test]
fn acceleration_rounds_once_at_the_finite_overflow_boundary() {
    let stencil = 1.269_511_161_302_448_5e308;
    let speed = 1.301_001_203_388_323_8;
    let spacing = 1.093_297_059_508_779_9;
    let mut world =
        WaveWorld::new(field(&[0.0, -stencil / 2.0, 0.0], speed, spacing)).expect("field");

    let center = world
        .step(TimeStep::new(0.75).expect("dt"))
        .expect("exact acceleration rounds to maximum finite")
        .snapshot
        .samples[1]
        .clone();
    assert_eq!(center.acceleration.get(), f64::MAX);
    assert!(center.velocity.get().is_finite());
    assert!(center.displacement.get().is_finite());
}

#[test]
fn acceleration_rejects_exact_overflow_even_if_rounded_factors_look_finite() {
    let stencil = 1.230_095_462_592_292e308;
    let speed = 1.692_033_924_774_457_1;
    let spacing = 1.399_654_015_922_604;
    let mut world =
        WaveWorld::new(field(&[0.0, -stencil / 2.0, 0.0], speed, spacing)).expect("field");
    let before = world.clone();

    assert_eq!(
        world.step(TimeStep::new(0.5).expect("dt")),
        Err(WaveError::DerivedNonFinite {
            operation: WaveArithmetic::Acceleration,
            sample: Some(1),
        })
    );
    assert_eq!(world, before);
}

#[test]
fn acceleration_rounds_once_to_the_minimum_subnormal() {
    let stencil = 9.477_621_891_619_394e300;
    let speed = 1.132_244_946_103e-312;
    let spacing = 2.217_750_825_257_271_5;
    let mut world =
        WaveWorld::new(field(&[0.0, -stencil / 2.0, 0.0], speed, spacing)).expect("field");

    let center = world
        .step(TimeStep::new(1.0e308).expect("dt"))
        .expect("exact acceleration rounds to minimum subnormal")
        .snapshot
        .samples[1]
        .clone();
    assert_eq!(center.acceleration.get(), f64::from_bits(1));
    assert!(center.velocity.get().is_finite() && center.velocity.get() > 0.0);
    assert!(center.displacement.get().is_finite());
}

#[test]
fn absorbed_velocity_progress_rejects_atomically() {
    let large_velocity = 2_f64.powi(53);
    let mut world = WaveWorld::new(field_with_velocity(
        &[0.0, -0.5, 0.0],
        &[0.0, large_velocity, 0.0],
        1.0,
        1.0,
    ))
    .expect("field");
    let before = world.clone();

    assert_eq!(
        world.step(TimeStep::new(1.0).expect("dt")),
        Err(WaveError::PrecisionLoss {
            operation: WaveArithmetic::VelocityUpdate,
            sample: Some(1),
        })
    );
    assert_eq!(world, before);
}

#[test]
fn absorbed_displacement_progress_rejects_atomically() {
    let large_displacement = 2_f64.powi(53);
    let mut world = WaveWorld::new(field_with_velocity(
        &[
            0.0,
            large_displacement,
            large_displacement,
            large_displacement,
            0.0,
        ],
        &[0.0, 0.0, 1.0, 0.0, 0.0],
        0.5,
        1.0,
    ))
    .expect("field");
    let before = world.clone();

    assert_eq!(
        world.step(TimeStep::new(1.0).expect("dt")),
        Err(WaveError::PrecisionLoss {
            operation: WaveArithmetic::DisplacementUpdate,
            sample: Some(2),
        })
    );
    assert_eq!(world, before);
}

#[test]
fn elapsed_time_must_advance_representably() {
    let speed = 2_f64.powi(-500);
    let mut world = WaveWorld::new(field(&[0.0, 0.0, 0.0], speed, 1.0)).expect("field");
    world
        .step(TimeStep::new(2_f64.powi(53)).expect("large dt"))
        .expect("first representable step");
    let before = world.clone();

    assert_eq!(
        world.step(TimeStep::new(1.0).expect("absorbed dt")),
        Err(WaveError::PrecisionLoss {
            operation: WaveArithmetic::ElapsedTime,
            sample: None,
        })
    );
    assert_eq!(world, before);
}

#[test]
fn snapshot_is_read_only_and_has_explicit_grid_coordinates() {
    let world = WaveWorld::new(field(&[0.0, 0.0, 0.0, 0.0], 1.0, 0.25)).expect("field");
    let first = world.snapshot();
    let second = world.snapshot();
    assert_eq!(first, second);
    assert_eq!(first.samples[3].x.get(), 0.75);
    assert_eq!(first.samples.len(), 4);
}

#[test]
fn maximum_field_is_bounded_ordered_and_emits_one_summary() {
    let mut world =
        WaveWorld::new(field(&vec![0.0; MAX_WAVE_SAMPLES], 1.0, 1.0)).expect("maximum field");
    let initial = world.snapshot();
    assert_eq!(initial.samples.len(), MAX_WAVE_SAMPLES);
    assert!(
        initial
            .samples
            .iter()
            .enumerate()
            .all(|(index, sample)| sample.index == index && sample.x.get() == index as f64)
    );

    let report = world
        .step(TimeStep::new(0.5).expect("dt"))
        .expect("bounded maximum step");
    assert_eq!(report.event.sample_count, MAX_WAVE_SAMPLES);
    assert_eq!(report.snapshot.samples.len(), MAX_WAVE_SAMPLES);
    assert_eq!(report.snapshot.boundary, BoundaryCondition::FixedZero);
}

#[test]
fn identical_inputs_produce_identical_reports_and_snapshots() {
    let spec = field_with_velocity(
        &[0.0, 0.25, 1.0, -0.5, 0.0],
        &[0.0, -0.1, 0.2, 0.3, 0.0],
        1.0,
        1.0,
    );
    let mut first = WaveWorld::new(spec.clone()).expect("first field");
    let mut second = WaveWorld::new(spec).expect("second field");

    for seconds in [0.125, 0.25, 0.5] {
        let dt = TimeStep::new(seconds).expect("dt");
        assert_eq!(first.step(dt), second.step(dt));
        assert_eq!(first.snapshot(), second.snapshot());
    }
}
