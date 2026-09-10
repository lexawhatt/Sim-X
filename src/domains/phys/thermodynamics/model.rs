use crate::foundation::EntityId;

use super::{JoulesPerKelvin, Kelvin, WattsPerKelvin};

/// Opaque stable identity of a conductive relationship in one thermal world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThermalLinkId(u64);

impl ThermalLinkId {
    /// Constructs a non-zero identity for persistence and domain allocators.
    pub const fn new(raw: u64) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    /// Returns the stable non-zero representation.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Validated creation data for one internally isothermal body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalBodySpec {
    /// Initial absolute temperature in kelvins.
    pub temperature: Kelvin,
    /// Lumped heat capacity in joules per kelvin.
    pub heat_capacity: JoulesPerKelvin,
}

impl ThermalBodySpec {
    /// Creates thermal body data from validated SI quantities.
    pub const fn new(temperature: Kelvin, heat_capacity: JoulesPerKelvin) -> Self {
        Self {
            temperature,
            heat_capacity,
        }
    }
}

/// Validated endpoint and conductance data proposed for a conductive link.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConductiveLinkSpec {
    /// First body endpoint. Endpoint order carries no physical direction.
    pub first: EntityId,
    /// Second distinct body endpoint.
    pub second: EntityId,
    /// Positive conductance in watts per kelvin.
    pub conductance: WattsPerKelvin,
}

impl ConductiveLinkSpec {
    /// Creates a conductive-link proposal from validated values.
    pub const fn new(first: EntityId, second: EntityId, conductance: WattsPerKelvin) -> Self {
        Self {
            first,
            second,
            conductance,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ThermalBody {
    pub(super) energy_joules: f64,
    pub(super) heat_capacity: JoulesPerKelvin,
    pub(super) last_net_power: super::Watts,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ConductiveLink {
    pub(super) first: EntityId,
    pub(super) second: EntityId,
    pub(super) conductance: WattsPerKelvin,
    pub(super) last_first_to_second_power: super::Watts,
}
