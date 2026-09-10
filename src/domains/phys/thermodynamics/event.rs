use crate::foundation::{EntityId, Seconds, StepIndex, WorldRevision};

use super::{Kelvin, ThermalLinkId, ThermalSnapshot, Watts};

/// Typed thermodynamic fact emitted only after a successful commit.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalEvent {
    /// Step that produced this fact.
    pub step: StepIndex,
    /// Canonical world revision committed by the step.
    pub revision: WorldRevision,
    /// Elapsed simulation time after the commit, in seconds.
    pub elapsed: Seconds,
    /// Typed post-commit payload.
    pub kind: ThermalEventKind,
}

/// Post-commit facts produced by one thermal step.
#[derive(Clone, Debug, PartialEq)]
pub enum ThermalEventKind {
    /// One conductive link's observable transfer for the committed step.
    HeatTransferred {
        /// Stable conductive-link identity.
        link: ThermalLinkId,
        /// Signed watts from the canonical first endpoint to the second.
        first_to_second_power: Watts,
    },
    /// One body's committed temperature and net heat-power observation.
    BodyTemperatureChanged {
        /// Stable thermal-body identity.
        entity: EntityId,
        /// Committed absolute temperature in kelvins.
        temperature: Kelvin,
        /// Signed net heat power applied to the body, in watts.
        net_heat_power: Watts,
    },
    /// Summary emitted after all link and body facts.
    StepCommitted {
        /// Number of thermal bodies present in the committed world.
        bodies: usize,
        /// Number of conductive links present in the committed world.
        links: usize,
    },
}

/// Successful atomic step output containing events and its exact snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalStepReport {
    /// Canonically ordered facts emitted by the successful commit.
    pub events: Vec<ThermalEvent>,
    /// Immutable state after the same commit.
    pub snapshot: ThermalSnapshot,
}
