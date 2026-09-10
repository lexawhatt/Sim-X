//! Deterministic translational mechanics for the first Sim;X vertical slice.

mod body;
mod capability;
mod error;
mod event;
mod force;
mod integration;
mod numeric;
mod quantities;
mod snapshot;
mod summation;
mod world;

pub use body::{BodySpec, Mobility};
pub use error::{ArithmeticOperation, CounterKind, MechanicsError};
pub use event::{MechanicsEvent, MechanicsEventKind, StepReport};
pub use force::{ForceContribution, ForceSourceId, ForceSourceIdError};
pub use quantities::{Acceleration2, Force2, Position2, Velocity2};
pub use snapshot::{BodySnapshot, MechanicsSnapshot};
pub use world::{
    MAX_BODIES, MAX_EVENTS_PER_STEP, MAX_FORCE_CONTRIBUTIONS_PER_STEP, MechanicsWorld,
};

#[cfg(test)]
mod tests;
