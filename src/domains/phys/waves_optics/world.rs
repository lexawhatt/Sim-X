use crate::foundation::{
    Meters, MetersPerSecond, MetersPerSecondSquared, Seconds, StepIndex, TimeStep, WorldRevision,
    reproducible_sum::reproducible_sum,
    scaled_product::{ScaledProductError, scaled_product_quotient},
};

use super::{
    BoundaryCondition, SampleSpacing, WaveArithmetic, WaveError, WaveEvent, WaveSampleSnapshot,
    WaveSnapshot, WaveSpec, WaveSpeed, WaveStepReport,
};

/// Minimum number of samples needed for two endpoints and one interior value.
pub const MIN_WAVE_SAMPLES: usize = 3;
/// Hard maximum sample count accepted by one canonical wave field.
pub const MAX_WAVE_SAMPLES: usize = 8_192;

/// Canonical bounded one-dimensional transverse wave state.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveWorld {
    spacing: SampleSpacing,
    speed: WaveSpeed,
    boundary: BoundaryCondition,
    x_coordinates: Vec<Meters>,
    displacement: Vec<Meters>,
    velocity: Vec<MetersPerSecond>,
    acceleration: Vec<MetersPerSecondSquared>,
    step: StepIndex,
    revision: WorldRevision,
    elapsed: Seconds,
}

impl WaveWorld {
    /// Validates and creates a uniform fixed-endpoint wave field.
    ///
    /// Sample count, matching vectors, fixed endpoints, and the complete grid
    /// extent are checked before canonical state is accepted.
    pub fn new(spec: WaveSpec) -> Result<Self, WaveError> {
        let count = spec.displacement.len();
        if !(MIN_WAVE_SAMPLES..=MAX_WAVE_SAMPLES).contains(&count) {
            return Err(WaveError::SampleCountOutOfRange {
                count,
                minimum: MIN_WAVE_SAMPLES,
                maximum: MAX_WAVE_SAMPLES,
            });
        }
        if count != spec.velocity.len() {
            return Err(WaveError::SampleCountMismatch {
                displacement: count,
                velocity: spec.velocity.len(),
            });
        }
        for index in [0, count - 1] {
            if spec.displacement[index] != Meters::ZERO
                || spec.velocity[index] != MetersPerSecond::ZERO
            {
                return Err(WaveError::InvalidFixedEndpoint { index });
            }
        }
        let x_coordinates = (0..count)
            .map(|index| {
                let x = checked_product(
                    index as f64,
                    spec.spacing.get(),
                    WaveArithmetic::GridExtent,
                    None,
                )?;
                Meters::new(x).map_err(|_| WaveError::DerivedNonFinite {
                    operation: WaveArithmetic::GridExtent,
                    sample: Some(index),
                })
            })
            .collect::<Result<Vec<_>, WaveError>>()?;

        Ok(Self {
            spacing: spec.spacing,
            speed: spec.speed,
            boundary: spec.boundary,
            x_coordinates,
            displacement: spec.displacement,
            velocity: spec.velocity,
            acceleration: vec![MetersPerSecondSquared::ZERO; count],
            step: StepIndex::ZERO,
            revision: WorldRevision::ZERO,
            elapsed: Seconds::ZERO,
        })
    }

    /// Advances the complete field by one stable fixed step.
    ///
    /// The Courant number must be at most one. Every interior sample is derived
    /// from the same committed pre-step displacement arrays and all candidates
    /// commit together or not at all.
    pub fn step(&mut self, dt: TimeStep) -> Result<WaveStepReport, WaveError> {
        let next_step = self
            .step
            .checked_next()
            .ok_or(WaveError::CounterExhausted)?;
        let next_revision = self
            .revision
            .checked_next()
            .ok_or(WaveError::CounterExhausted)?;
        let next_elapsed_value = checked_sum(
            self.elapsed.get(),
            dt.get(),
            WaveArithmetic::ElapsedTime,
            None,
        )?;
        if next_elapsed_value == self.elapsed.get() {
            return Err(WaveError::PrecisionLoss {
                operation: WaveArithmetic::ElapsedTime,
                sample: None,
            });
        }
        let next_elapsed =
            Seconds::new(next_elapsed_value).map_err(|_| WaveError::DerivedNonFinite {
                operation: WaveArithmetic::ElapsedTime,
                sample: None,
            })?;

        let courant = scaled_ratio(
            &[self.speed.get(), dt.get()],
            &[self.spacing.get()],
            WaveArithmetic::CourantNumber,
            None,
        )?;
        if courant > 1.0 {
            return Err(WaveError::CourantLimitExceeded {
                courant_number: courant,
            });
        }

        let mut next_displacement = self.displacement.clone();
        let mut next_velocity = self.velocity.clone();
        let mut next_acceleration = self.acceleration.clone();

        for index in 1..self.displacement.len() - 1 {
            let center = self.displacement[index].get();
            let second_difference = reproducible_sum(&[
                self.displacement[index - 1].get(),
                -center,
                -center,
                self.displacement[index + 1].get(),
            ])
            .map_err(|_| WaveError::DerivedNonFinite {
                operation: WaveArithmetic::StencilReduction,
                sample: Some(index),
            })?;
            let acceleration = scaled_ratio(
                &[second_difference, self.speed.get(), self.speed.get()],
                &[self.spacing.get(), self.spacing.get()],
                WaveArithmetic::Acceleration,
                Some(index),
            )?;
            let velocity_increment = checked_product(
                acceleration,
                dt.get(),
                WaveArithmetic::VelocityIncrement,
                Some(index),
            )?;
            let velocity = checked_sum(
                self.velocity[index].get(),
                velocity_increment,
                WaveArithmetic::VelocityUpdate,
                Some(index),
            )?;
            if velocity_increment != 0.0 && velocity == self.velocity[index].get() {
                return Err(WaveError::PrecisionLoss {
                    operation: WaveArithmetic::VelocityUpdate,
                    sample: Some(index),
                });
            }
            let displacement_increment = checked_product(
                velocity,
                dt.get(),
                WaveArithmetic::DisplacementIncrement,
                Some(index),
            )?;
            let displacement = checked_sum(
                self.displacement[index].get(),
                displacement_increment,
                WaveArithmetic::DisplacementUpdate,
                Some(index),
            )?;
            if displacement_increment != 0.0 && displacement == self.displacement[index].get() {
                return Err(WaveError::PrecisionLoss {
                    operation: WaveArithmetic::DisplacementUpdate,
                    sample: Some(index),
                });
            }

            next_acceleration[index] = MetersPerSecondSquared::new(acceleration).map_err(|_| {
                WaveError::DerivedNonFinite {
                    operation: WaveArithmetic::Acceleration,
                    sample: Some(index),
                }
            })?;
            next_velocity[index] =
                MetersPerSecond::new(velocity).map_err(|_| WaveError::DerivedNonFinite {
                    operation: WaveArithmetic::VelocityUpdate,
                    sample: Some(index),
                })?;
            next_displacement[index] =
                Meters::new(displacement).map_err(|_| WaveError::DerivedNonFinite {
                    operation: WaveArithmetic::DisplacementUpdate,
                    sample: Some(index),
                })?;
        }

        self.displacement = next_displacement;
        self.velocity = next_velocity;
        self.acceleration = next_acceleration;
        self.step = next_step;
        self.revision = next_revision;
        self.elapsed = next_elapsed;

        let snapshot = self.snapshot();
        let event = WaveEvent {
            step: self.step,
            revision: self.revision,
            elapsed: self.elapsed,
            sample_count: self.displacement.len(),
        };
        Ok(WaveStepReport { event, snapshot })
    }

    /// Returns a bounded ordered immutable field snapshot without stepping.
    pub fn snapshot(&self) -> WaveSnapshot {
        let samples = self
            .x_coordinates
            .iter()
            .zip(&self.displacement)
            .zip(&self.velocity)
            .zip(&self.acceleration)
            .enumerate()
            .map(
                |(index, (((x, displacement), velocity), acceleration))| WaveSampleSnapshot {
                    index,
                    x: *x,
                    displacement: *displacement,
                    velocity: *velocity,
                    acceleration: *acceleration,
                },
            )
            .collect();
        WaveSnapshot {
            step: self.step,
            revision: self.revision,
            elapsed: self.elapsed,
            spacing: self.spacing,
            speed: self.speed,
            boundary: self.boundary,
            samples,
        }
    }
}

fn checked_sum(
    left: f64,
    right: f64,
    operation: WaveArithmetic,
    sample: Option<usize>,
) -> Result<f64, WaveError> {
    let value = left + right;
    if value.is_finite() {
        Ok(if value == 0.0 { 0.0 } else { value })
    } else {
        Err(WaveError::DerivedNonFinite { operation, sample })
    }
}

fn checked_product(
    left: f64,
    right: f64,
    operation: WaveArithmetic,
    sample: Option<usize>,
) -> Result<f64, WaveError> {
    let value = left * right;
    if !value.is_finite() {
        Err(WaveError::DerivedNonFinite { operation, sample })
    } else if left != 0.0 && right != 0.0 && value == 0.0 {
        Err(WaveError::PrecisionLoss { operation, sample })
    } else {
        Ok(if value == 0.0 { 0.0 } else { value })
    }
}

fn scaled_ratio(
    numerators: &[f64],
    denominators: &[f64],
    operation: WaveArithmetic,
    sample: Option<usize>,
) -> Result<f64, WaveError> {
    match scaled_product_quotient(numerators, denominators) {
        Ok(value) => Ok(value),
        Err(ScaledProductError::Underflow) => Err(WaveError::PrecisionLoss { operation, sample }),
        Err(_) => Err(WaveError::DerivedNonFinite { operation, sample }),
    }
}

#[cfg(test)]
mod tests {
    use crate::foundation::{Meters, MetersPerSecond, StepIndex, TimeStep, WorldRevision};

    use super::{SampleSpacing, WaveError, WaveSpec, WaveSpeed, WaveWorld};

    fn zero_world() -> WaveWorld {
        WaveWorld::new(WaveSpec::fixed_zero(
            SampleSpacing::new(1.0).expect("spacing"),
            WaveSpeed::new(1.0).expect("speed"),
            vec![Meters::ZERO; 3],
            vec![MetersPerSecond::ZERO; 3],
        ))
        .expect("zero field")
    }

    #[test]
    fn exhausted_step_counter_rejects_atomically() {
        let mut world = zero_world();
        world.step = StepIndex::new(u64::MAX);
        let before = world.clone();

        assert_eq!(
            world.step(TimeStep::new(0.5).expect("dt")),
            Err(WaveError::CounterExhausted)
        );
        assert_eq!(world, before);
    }

    #[test]
    fn exhausted_revision_counter_rejects_atomically() {
        let mut world = zero_world();
        world.revision = WorldRevision::new(u64::MAX);
        let before = world.clone();

        assert_eq!(
            world.step(TimeStep::new(0.5).expect("dt")),
            Err(WaveError::CounterExhausted)
        );
        assert_eq!(world, before);
    }
}
