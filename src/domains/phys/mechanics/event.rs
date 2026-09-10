use crate::foundation::{EntityId, Seconds, StepIndex, WorldRevision};

use super::{Acceleration2, Force2, ForceSourceId, MechanicsSnapshot, Position2, Velocity2};

/// Typed payload of one post-commit Mechanics observation.
#[derive(Clone, Debug, PartialEq)]
pub enum MechanicsEventKind {
    /// One validated contribution participated in the committed step.
    ForceAccepted {
        /// Dynamic target that received the contribution.
        target: EntityId,
        /// Stable semantic source of the contribution.
        source: ForceSourceId,
        /// Accepted force vector in newtons.
        force: Force2,
    },
    /// One dynamic body was advanced by the fixed-step integrator.
    BodyIntegrated {
        /// Stable body identity.
        entity: EntityId,
        /// Newly committed position.
        position: Position2,
        /// Newly committed velocity.
        velocity: Velocity2,
        /// Acceleration used for this step.
        acceleration: Acceleration2,
        /// Reduced net force used for this step.
        net_force: Force2,
    },
    /// Summary emitted after all body events for one committed step.
    StepCommitted {
        /// Number of dynamic bodies integrated.
        dynamic_bodies: usize,
        /// Number of distinct accepted force contributions.
        accepted_forces: usize,
    },
}

/// One immutable post-commit Mechanics event with canonical time metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicsEvent {
    /// Step that produced this event.
    pub step: StepIndex,
    /// World revision committed by the step.
    pub revision: WorldRevision,
    /// Elapsed simulation time after the step.
    pub elapsed: Seconds,
    /// Typed event payload.
    pub kind: MechanicsEventKind,
}

/// Complete successful result of one fixed Mechanics step.
#[derive(Clone, Debug, PartialEq)]
pub struct StepReport {
    /// Immutable read model built from the committed world.
    pub snapshot: MechanicsSnapshot,
    /// Canonically ordered post-commit observations.
    pub events: Vec<MechanicsEvent>,
}
