use crate::foundation::{Meters, MetersPerSecond};

use super::{SampleSpacing, WaveSpeed};

/// Boundary semantics for the canonical one-dimensional field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryCondition {
    /// Both endpoint displacement and velocity remain exact zero.
    FixedZero,
}

/// Complete construction data for a uniform bounded wave field.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveSpec {
    /// Uniform distance between samples in meters.
    pub spacing: SampleSpacing,
    /// Propagation speed in meters per second.
    pub speed: WaveSpeed,
    /// Initial transverse displacement samples in meters.
    pub displacement: Vec<Meters>,
    /// Initial transverse velocity samples in meters per second.
    pub velocity: Vec<MetersPerSecond>,
    /// Explicit endpoint behavior.
    pub boundary: BoundaryCondition,
}

impl WaveSpec {
    /// Creates a fixed-boundary field proposal from validated SI samples.
    pub fn fixed_zero(
        spacing: SampleSpacing,
        speed: WaveSpeed,
        displacement: Vec<Meters>,
        velocity: Vec<MetersPerSecond>,
    ) -> Self {
        Self {
            spacing,
            speed,
            displacement,
            velocity,
            boundary: BoundaryCondition::FixedZero,
        }
    }
}
