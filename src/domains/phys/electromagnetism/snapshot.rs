use crate::foundation::{
    ConstantsSource, ConstantsVersion, EntityId, VacuumPermittivity, WorldRevision,
};

use super::{Coulombs, ElectricField2, FieldProbeId, NewtonsPerCoulomb, Position2};

/// Immutable renderer-neutral state of one stationary point charge.
#[derive(Clone, Debug, PartialEq)]
pub struct PointChargeSnapshot {
    /// Stable point-charge identity.
    pub entity: EntityId,
    /// Stationary charge position in world meters.
    pub position: Position2,
    /// Finite signed non-zero charge in coulombs.
    pub charge: Coulombs,
}

/// Immutable evaluated electric field at one probe position.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldProbeSnapshot {
    /// Stable field-probe identity.
    pub probe: FieldProbeId,
    /// Probe position in world meters.
    pub position: Position2,
    /// Reduced electric-field vector in newtons per coulomb.
    pub field: ElectricField2,
    /// Overflow-safe magnitude of `field`, in newtons per coulomb.
    pub magnitude: NewtonsPerCoulomb,
}

/// Complete bounded output of one pure electrostatic evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct ElectrostaticSnapshot {
    /// Revision of the stationary canonical charge/probe world.
    pub revision: WorldRevision,
    /// Schema version of the constants registry used for evaluation.
    pub constants_version: ConstantsVersion,
    /// Provenance of the constants registry used for evaluation.
    pub constants_source: ConstantsSource,
    /// Exact scene-wide constant used to derive every field in this snapshot.
    pub vacuum_permittivity: VacuumPermittivity,
    /// Point charges ordered by stable `EntityId`.
    pub charges: Vec<PointChargeSnapshot>,
    /// Evaluated probes ordered by stable `FieldProbeId`.
    pub probes: Vec<FieldProbeSnapshot>,
}
