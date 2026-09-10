use crate::foundation::{EntityId, Kilograms, Seconds, StepIndex, WorldRevision};

use super::{Acceleration2, Force2, Mobility, Position2, Velocity2, body::Body};

/// Immutable renderer-neutral body state after a committed operation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodySnapshot {
    /// Stable body identity.
    pub id: EntityId,
    /// Committed world position in meters.
    pub position: Position2,
    /// Committed velocity in meters per second.
    pub velocity: Velocity2,
    /// Finite positive mass in kilograms.
    pub mass: Kilograms,
    /// Whether ordinary force integration can move this body.
    pub mobility: Mobility,
    /// Acceleration used by the most recently committed step.
    pub acceleration: Acceleration2,
    /// Net force reduced for the most recently committed step.
    pub net_force: Force2,
}

impl From<&Body> for BodySnapshot {
    fn from(body: &Body) -> Self {
        Self {
            id: body.id,
            position: body.position,
            velocity: body.velocity,
            mass: body.mass,
            mobility: body.mobility,
            acceleration: body.last_acceleration,
            net_force: body.last_net_force,
        }
    }
}

/// Bounded immutable read model of committed Mechanics state.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicsSnapshot {
    /// Number of successfully committed physics steps.
    pub step: StepIndex,
    /// Revision of all committed canonical world state.
    pub revision: WorldRevision,
    /// Elapsed fixed-step simulation time.
    pub elapsed: Seconds,
    /// Body values sorted by stable `EntityId`.
    pub bodies: Vec<BodySnapshot>,
}
