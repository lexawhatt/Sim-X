use crate::foundation::{EntityId, Kilograms};

use super::{Acceleration2, Force2, Position2, Velocity2};

/// Whether ordinary Mechanics integration may move a body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mobility {
    /// The body responds to accepted force contributions according to its mass.
    Dynamic,
    /// The body is not advanced by ordinary force integration.
    Fixed,
}

/// Validated canonical values used to create one Mechanics body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodySpec {
    /// Initial world position in meters.
    pub position: Position2,
    /// Initial linear velocity in meters per second.
    pub velocity: Velocity2,
    /// Finite, strictly positive body mass.
    pub mass: Kilograms,
    /// Ordinary integration policy.
    pub mobility: Mobility,
}

impl BodySpec {
    /// Constructs a body specification from already validated typed values.
    pub const fn new(
        position: Position2,
        velocity: Velocity2,
        mass: Kilograms,
        mobility: Mobility,
    ) -> Self {
        Self {
            position,
            velocity,
            mass,
            mobility,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Body {
    pub(super) id: EntityId,
    pub(super) position: Position2,
    pub(super) velocity: Velocity2,
    pub(super) mass: Kilograms,
    pub(super) mobility: Mobility,
    pub(super) last_acceleration: Acceleration2,
    pub(super) last_net_force: Force2,
}

impl Body {
    pub(super) const fn new(id: EntityId, spec: BodySpec) -> Self {
        Self {
            id,
            position: spec.position,
            velocity: spec.velocity,
            mass: spec.mass,
            mobility: spec.mobility,
            last_acceleration: Acceleration2::ZERO,
            last_net_force: Force2::ZERO,
        }
    }

    pub(super) fn commit_integrated(
        &mut self,
        position: Position2,
        velocity: Velocity2,
        acceleration: Acceleration2,
        net_force: Force2,
    ) {
        self.position = position;
        self.velocity = velocity;
        self.last_acceleration = acceleration;
        self.last_net_force = net_force;
    }

    pub(super) fn clear_step_outputs(&mut self) {
        self.last_acceleration = Acceleration2::ZERO;
        self.last_net_force = Force2::ZERO;
    }
}
