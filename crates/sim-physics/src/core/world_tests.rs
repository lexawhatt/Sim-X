use super::*;
use crate::Vec2;

#[test]
fn elapsed_progress_and_step_counter_failure_leave_everything_unchanged() {
    let mut world = World::new(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &[],
        &[],
    )
    .unwrap();
    world.elapsed_s = 2_f64.powi(53);
    let before = world.snapshot();
    assert_eq!(
        world.step(&[]),
        Err(Error::PrecisionLoss(NumericStage::Time))
    );
    assert_eq!(world.snapshot(), before);
    world.elapsed_s = 0.0;
    world.step_index = u64::MAX;
    let before = world.snapshot();
    assert_eq!(world.step(&[]), Err(Error::CounterExhausted));
    assert_eq!(world.snapshot(), before);
}

#[test]
fn exhausted_settings_revision_rejects_changes_but_allows_noop() {
    let mut world = World::new(
        PhysicsSettings::default(),
        SolverConfig::default(),
        &[],
        &[],
    )
    .unwrap();
    world.settings_revision = u64::MAX;
    let before = world.snapshot();
    assert_eq!(world.set_settings(world.settings()), Ok(false));
    assert_eq!(
        world.set_settings(PhysicsSettings {
            gravity_m_s2: Vec2::ZERO,
            ..world.settings()
        }),
        Err(Error::CounterExhausted)
    );
    assert_eq!(world.snapshot(), before);
}
