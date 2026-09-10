use crate::foundation::{EntityId, Kilograms};

use super::{Mobility, Position2, Velocity2, body::Body};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CapabilityRejection {
    FixedBody { entity: EntityId },
}

/// Ephemeral read-only proof that a body may receive ordinary force.
pub(super) struct ForceReceiver<'body> {
    body: &'body Body,
}

impl<'body> ForceReceiver<'body> {
    pub(super) fn project(body: &'body Body) -> Result<Self, CapabilityRejection> {
        match body.mobility {
            Mobility::Dynamic => Ok(Self { body }),
            Mobility::Fixed => Err(CapabilityRejection::FixedBody { entity: body.id }),
        }
    }

    pub(super) const fn entity(&self) -> EntityId {
        self.body.id
    }

    pub(super) const fn position(&self) -> Position2 {
        self.body.position
    }

    pub(super) const fn velocity(&self) -> Velocity2 {
        self.body.velocity
    }

    pub(super) const fn mass(&self) -> Kilograms {
        self.body.mass
    }
}
