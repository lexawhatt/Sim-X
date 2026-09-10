use crate::{
    domains::phys::waves_optics::{SampleSpacing, WaveSnapshot, WaveSpec, WaveSpeed, WaveWorld},
    foundation::{Meters, MetersPerSecond, TimeStep},
    presentation::ui::ProjectTemplate,
};

use super::fixed_step_clock::FixedStepClock;

const FIXED_STEP_SECONDS: f64 = 1.0 / 120.0;
const MAX_FIXED_STEPS_PER_FRAME: usize = 8;
const MAX_WALL_DELTA_SECONDS: f64 = 0.1;
const SAMPLE_COUNT: usize = 129;
const SAMPLE_SPACING_METERS: f64 = 0.05;

/// App-owned fixed-step adapter for one open Waves and Optics project.
#[derive(Clone, Debug)]
pub(super) struct WaveSession {
    template: ProjectTemplate,
    world: WaveWorld,
    snapshot: WaveSnapshot,
    fixed_step: TimeStep,
    clock: FixedStepClock,
    faulted: bool,
}

impl WaveSession {
    pub(super) fn new(template: ProjectTemplate) -> Result<Self, String> {
        let displacement = match template {
            ProjectTemplate::Blank => vec![Meters::ZERO; SAMPLE_COUNT],
            ProjectTemplate::PrimaryLab => pulse_samples()?,
            ProjectTemplate::SecondaryLab => standing_wave_samples()?,
        };
        let spec = WaveSpec::fixed_zero(
            SampleSpacing::new(SAMPLE_SPACING_METERS).map_err(|error| error.to_string())?,
            WaveSpeed::new(1.0).map_err(|error| error.to_string())?,
            displacement,
            vec![MetersPerSecond::ZERO; SAMPLE_COUNT],
        );
        let world = WaveWorld::new(spec).map_err(|error| error.to_string())?;
        let snapshot = world.snapshot();
        Ok(Self {
            template,
            world,
            snapshot,
            fixed_step: TimeStep::new(FIXED_STEP_SECONDS).map_err(|error| error.to_string())?,
            clock: FixedStepClock::new(
                FIXED_STEP_SECONDS,
                MAX_FIXED_STEPS_PER_FRAME,
                MAX_WALL_DELTA_SECONDS,
            )?,
            faulted: false,
        })
    }

    pub(super) const fn template(&self) -> ProjectTemplate {
        self.template
    }

    pub(super) const fn snapshot(&self) -> &WaveSnapshot {
        &self.snapshot
    }

    pub(super) fn reset_transient_runtime(&mut self) -> Result<(), String> {
        self.clock = FixedStepClock::new(
            FIXED_STEP_SECONDS,
            MAX_FIXED_STEPS_PER_FRAME,
            MAX_WALL_DELTA_SECONDS,
        )?;
        self.faulted = false;
        Ok(())
    }

    pub(super) fn advance(&mut self, wall_seconds: f64) -> Result<(), String> {
        if self.faulted {
            return Ok(());
        }
        for _ in 0..self.clock.take_steps(wall_seconds) {
            match self.world.step(self.fixed_step) {
                Ok(report) => self.snapshot = report.snapshot,
                Err(error) => {
                    self.faulted = true;
                    return Err(format!("wave session paused after domain error: {error}"));
                }
            }
        }
        Ok(())
    }
}

fn pulse_samples() -> Result<Vec<Meters>, String> {
    let center = (SAMPLE_COUNT - 1) as f64 * 0.5;
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for index in 0..SAMPLE_COUNT {
        let value = if index == 0 || index == SAMPLE_COUNT - 1 {
            0.0
        } else {
            let normalized = (index as f64 - center) / 9.0;
            0.7 * (-normalized * normalized).exp()
        };
        samples.push(Meters::new(value).map_err(|error| error.to_string())?);
    }
    Ok(samples)
}

fn standing_wave_samples() -> Result<Vec<Meters>, String> {
    let last = SAMPLE_COUNT - 1;
    (0..SAMPLE_COUNT)
        .map(|index| {
            let value = if index == 0 || index == last {
                0.0
            } else {
                let phase = 2.0 * std::f64::consts::PI * index as f64 / last as f64;
                0.55 * phase.sin()
            };
            Meters::new(value).map_err(|error| error.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{SAMPLE_COUNT, WaveSession};
    use crate::presentation::ui::ProjectTemplate;

    #[test]
    fn pulse_project_advances_with_bounded_fixed_steps() {
        let mut session = WaveSession::new(ProjectTemplate::PrimaryLab).expect("session");
        session.advance(10.0).expect("bounded steps");
        assert_eq!(session.snapshot().step.get(), 8);
        assert_eq!(session.snapshot().samples.len(), 129);
    }

    #[test]
    fn standing_wave_project_has_two_opposite_antinodes() {
        let session = WaveSession::new(ProjectTemplate::SecondaryLab).expect("session");
        let samples = &session.snapshot().samples;
        assert!(samples[SAMPLE_COUNT / 4].displacement.get() > 0.5);
        assert!(samples[SAMPLE_COUNT * 3 / 4].displacement.get() < -0.5);
        assert_eq!(samples.first().unwrap().displacement.get(), 0.0);
        assert_eq!(samples.last().unwrap().displacement.get(), 0.0);
    }
}
