use crate::foundation::{EntityId, Meters};

use super::Coulombs;

/// Finite two-dimensional world position in meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position2 {
    x: Meters,
    y: Meters,
}

impl Position2 {
    /// World origin in meters.
    pub const ZERO: Self = Self {
        x: Meters::ZERO,
        y: Meters::ZERO,
    };

    /// Constructs a position from validated meter components.
    pub const fn new(x: Meters, y: Meters) -> Self {
        Self { x, y }
    }

    /// Returns the world X coordinate in meters.
    pub const fn x(self) -> Meters {
        self.x
    }

    /// Returns the world Y coordinate in meters.
    pub const fn y(self) -> Meters {
        self.y
    }
}

/// Stable identity of an electric-field probe in one electrostatic world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldProbeId(u64);

impl FieldProbeId {
    /// Constructs a non-zero identity for allocators and persistence boundaries.
    pub const fn new(raw: u64) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    /// Returns the stable non-zero representation.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Complete canonical creation data for one stationary point charge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointChargeSpec {
    /// Charge location in world meters.
    pub position: Position2,
    /// Signed non-zero charge in coulombs.
    pub charge: Coulombs,
}

impl PointChargeSpec {
    /// Creates a point-charge proposal from validated SI values.
    pub const fn new(position: Position2, charge: Coulombs) -> Self {
        Self { position, charge }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PointCharge {
    pub(super) entity: EntityId,
    pub(super) position: Position2,
    pub(super) charge: Coulombs,
}
