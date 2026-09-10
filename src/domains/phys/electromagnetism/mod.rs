//! Bounded stationary point-charge electric-field evaluation.

mod error;
mod model;
mod quantities;
mod snapshot;
mod world;

pub use error::{ElectrostaticArithmetic, ElectrostaticError};
pub use model::{FieldProbeId, PointChargeSpec, Position2};
pub use quantities::{Coulombs, ElectricField2, ElectrostaticQuantityError, NewtonsPerCoulomb};
pub use snapshot::{ElectrostaticSnapshot, FieldProbeSnapshot, PointChargeSnapshot};
pub use world::{
    ElectrostaticWorld, MAX_CHARGES, MAX_ELECTROSTATIC_INTERACTIONS, MAX_FIELD_PROBES,
};

#[cfg(test)]
mod tests;
