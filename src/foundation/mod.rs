//! Small domain-neutral contracts shared across Sim;X boundaries.

mod constants;
mod ids;
pub(crate) mod reproducible_sum;
pub(crate) mod scaled_product;
mod units;

pub use constants::{
    ConstantsError, ConstantsSource, ConstantsVersion, PhysicalConstants, VacuumPermittivity,
};
pub use ids::{EntityId, IdError, StepIndex, WorldRevision};
pub use units::{
    Kilograms, Meters, MetersPerSecond, MetersPerSecondSquared, Newtons, QuantityError, Seconds,
    TimeStep,
};
