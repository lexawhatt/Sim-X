use crate::{
    domains::phys::thermodynamics::{
        ConductiveLinkSpec, JoulesPerKelvin, Kelvin, ThermalBodySpec, ThermalSnapshot,
        ThermalWorld, WattsPerKelvin,
    },
    foundation::TimeStep,
    presentation::ui::ProjectTemplate,
};

use super::fixed_step_clock::FixedStepClock;

const FIXED_STEP_SECONDS: f64 = 1.0 / 120.0;
const MAX_FIXED_STEPS_PER_FRAME: usize = 8;
const MAX_WALL_DELTA_SECONDS: f64 = 0.1;

/// App-owned fixed-step adapter for one open Thermodynamics project.
#[derive(Clone, Debug)]
pub(super) struct ThermalSession {
    template: ProjectTemplate,
    world: ThermalWorld,
    snapshot: ThermalSnapshot,
    fixed_step: TimeStep,
    clock: FixedStepClock,
    faulted: bool,
}

impl ThermalSession {
    pub(super) fn new(template: ProjectTemplate) -> Result<Self, String> {
        let mut world = ThermalWorld::new();
        match template {
            ProjectTemplate::Blank => {
                world
                    .create_body(body(293.15, 20.0)?)
                    .map_err(|error| error.to_string())?;
            }
            ProjectTemplate::PrimaryLab => {
                let hot = world
                    .create_body(body(420.0, 20.0)?)
                    .map_err(|error| error.to_string())?;
                let cold = world
                    .create_body(body(280.0, 20.0)?)
                    .map_err(|error| error.to_string())?;
                world
                    .create_link(ConductiveLinkSpec::new(
                        hot,
                        cold,
                        WattsPerKelvin::new(5.0).map_err(|error| error.to_string())?,
                    ))
                    .map_err(|error| error.to_string())?;
            }
            ProjectTemplate::SecondaryLab => {
                let hot = world
                    .create_body(body(500.0, 100.0)?)
                    .map_err(|error| error.to_string())?;
                let middle = world
                    .create_body(body(350.0, 80.0)?)
                    .map_err(|error| error.to_string())?;
                let cold = world
                    .create_body(body(250.0, 120.0)?)
                    .map_err(|error| error.to_string())?;
                for (first, second, conductance) in [(hot, middle, 4.0), (middle, cold, 7.0)] {
                    world
                        .create_link(ConductiveLinkSpec::new(
                            first,
                            second,
                            WattsPerKelvin::new(conductance).map_err(|error| error.to_string())?,
                        ))
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        let snapshot = world.snapshot().map_err(|error| error.to_string())?;
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

    pub(super) const fn snapshot(&self) -> &ThermalSnapshot {
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
                    return Err(format!(
                        "thermodynamics session paused after domain error: {error}"
                    ));
                }
            }
        }
        Ok(())
    }
}

fn body(
    temperature_kelvin: f64,
    heat_capacity_joules_per_kelvin: f64,
) -> Result<ThermalBodySpec, String> {
    Ok(ThermalBodySpec::new(
        Kelvin::new(temperature_kelvin).map_err(|error| error.to_string())?,
        JoulesPerKelvin::new(heat_capacity_joules_per_kelvin).map_err(|error| error.to_string())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::ThermalSession;
    use crate::presentation::ui::ProjectTemplate;

    #[test]
    fn heat_exchange_project_advances_toward_equilibrium() {
        let mut session = ThermalSession::new(ProjectTemplate::PrimaryLab).expect("session");
        let before = session.snapshot().bodies[0].temperature.get();
        session.advance(0.05).expect("steps");
        assert!(session.snapshot().bodies[0].temperature.get() < before);
        assert!(session.snapshot().step.get() > 0);
    }

    #[test]
    fn thermal_network_project_composes_multiple_links() {
        let mut session = ThermalSession::new(ProjectTemplate::SecondaryLab).expect("session");
        assert_eq!(session.snapshot().bodies.len(), 3);
        assert_eq!(session.snapshot().links.len(), 2);
        session.advance(0.05).expect("network steps");
        assert!(session.snapshot().step.get() > 0);
    }
}
