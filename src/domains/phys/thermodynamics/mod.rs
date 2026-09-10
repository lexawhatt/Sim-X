//! Bounded lumped-capacitance heat conduction.

mod error;
mod event;
mod model;
mod quantities;
mod snapshot;
mod world;

pub use error::{ThermalArithmetic, ThermalError};
pub use event::{ThermalEvent, ThermalEventKind, ThermalStepReport};
pub use model::{ConductiveLinkSpec, ThermalBodySpec, ThermalLinkId};
pub use quantities::{
    Joules, JoulesPerKelvin, Kelvin, ThermalQuantityError, Watts, WattsPerKelvin,
};
pub use snapshot::{ThermalBodySnapshot, ThermalEnergyTotal, ThermalLinkSnapshot, ThermalSnapshot};
pub use world::{MAX_THERMAL_BODIES, MAX_THERMAL_EVENTS, MAX_THERMAL_LINKS, ThermalWorld};

#[cfg(test)]
mod tests;
