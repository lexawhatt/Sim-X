use crate::foundation::{EntityId, Seconds, StepIndex, WorldRevision};

use super::{Joules, JoulesPerKelvin, Kelvin, ThermalLinkId, Watts, WattsPerKelvin};

/// Representability of the exact scene-wide sum of positive body energies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThermalEnergyTotal {
    /// The world has no thermal bodies and therefore no stored thermal energy.
    Empty,
    /// The exact total rounds to a finite positive binary64 joule value.
    Representable(Joules),
    /// The exact positive total is larger than finite binary64 can represent.
    AboveBinary64Range,
}

/// Immutable renderer-neutral state of one thermal body.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalBodySnapshot {
    /// Stable thermal-body identity.
    pub entity: EntityId,
    /// Derived absolute temperature in kelvins.
    pub temperature: Kelvin,
    /// Canonical stored thermal energy in joules.
    pub energy: Joules,
    /// Positive lumped heat capacity in joules per kelvin.
    pub heat_capacity: JoulesPerKelvin,
    /// Net heat power applied by the most recent step, in watts.
    pub net_heat_power: Watts,
}

/// Immutable renderer-neutral state of one conductive link.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalLinkSnapshot {
    /// Stable conductive-link identity.
    pub link: ThermalLinkId,
    /// Canonically ordered first body endpoint.
    pub first: EntityId,
    /// Canonically ordered second body endpoint.
    pub second: EntityId,
    /// Positive conductance in watts per kelvin.
    pub conductance: WattsPerKelvin,
    /// Most recent signed heat power from `first` to `second`, in watts.
    pub first_to_second_power: Watts,
}

/// Bounded committed state exposed to app and presentation adapters.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalSnapshot {
    /// Number of successfully committed thermal steps.
    pub step: StepIndex,
    /// Revision of all committed canonical world state.
    pub revision: WorldRevision,
    /// Elapsed fixed-step simulation time in seconds.
    pub elapsed: Seconds,
    /// Domain-owned exact-sum outcome used by diagnostics and presentation.
    pub total_energy: ThermalEnergyTotal,
    /// Body snapshots ordered by stable `EntityId`.
    pub bodies: Vec<ThermalBodySnapshot>,
    /// Conductive-link snapshots ordered by stable `ThermalLinkId`.
    pub links: Vec<ThermalLinkSnapshot>,
}
