/// Bounded conversion from accepted wall time to a count of fixed domain steps.
#[derive(Clone, Debug)]
pub(super) struct FixedStepClock {
    fixed_step_seconds: f64,
    maximum_steps_per_frame: usize,
    maximum_wall_delta_seconds: f64,
    accumulator_seconds: f64,
}

impl FixedStepClock {
    pub(super) fn new(
        fixed_step_seconds: f64,
        maximum_steps_per_frame: usize,
        maximum_wall_delta_seconds: f64,
    ) -> Result<Self, String> {
        if !fixed_step_seconds.is_finite() || fixed_step_seconds <= 0.0 {
            return Err("fixed-step clock requires positive finite step".to_owned());
        }
        if maximum_steps_per_frame == 0 {
            return Err("fixed-step clock requires a non-zero work budget".to_owned());
        }
        if !maximum_wall_delta_seconds.is_finite() || maximum_wall_delta_seconds <= 0.0 {
            return Err("fixed-step clock requires positive finite wall delta".to_owned());
        }
        Ok(Self {
            fixed_step_seconds,
            maximum_steps_per_frame,
            maximum_wall_delta_seconds,
            accumulator_seconds: 0.0,
        })
    }

    pub(super) fn take_steps(&mut self, wall_seconds: f64) -> usize {
        if !wall_seconds.is_finite() || wall_seconds <= 0.0 {
            return 0;
        }
        let work_budget = self.fixed_step_seconds * self.maximum_steps_per_frame as f64;
        let accepted = wall_seconds.min(self.maximum_wall_delta_seconds);
        self.accumulator_seconds = (self.accumulator_seconds + accepted).min(work_budget);
        let ratio = self.accumulator_seconds / self.fixed_step_seconds;
        let nearest = ratio.round();
        let tolerance = 8.0 * f64::EPSILON * ratio.abs().max(1.0);
        let integral_ratio = if (ratio - nearest).abs() <= tolerance {
            nearest
        } else {
            ratio.floor()
        };
        let available = integral_ratio as usize;
        let steps = available.min(self.maximum_steps_per_frame);
        self.accumulator_seconds -= self.fixed_step_seconds * steps as f64;
        if self.accumulator_seconds.abs() <= self.fixed_step_seconds * 8.0 * f64::EPSILON {
            self.accumulator_seconds = 0.0;
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::FixedStepClock;

    #[test]
    fn wall_time_conversion_is_bounded_and_retains_fractional_time() {
        let mut clock = FixedStepClock::new(0.1, 3, 1.0).expect("clock");
        assert_eq!(clock.take_steps(0.25), 2);
        assert_eq!(clock.take_steps(0.05), 1);
        assert_eq!(clock.take_steps(100.0), 3);
    }
}
