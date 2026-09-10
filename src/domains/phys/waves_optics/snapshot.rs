use crate::foundation::{
    Meters, MetersPerSecond, MetersPerSecondSquared, Seconds, StepIndex, WorldRevision,
};

use super::{BoundaryCondition, SampleSpacing, WaveSpeed};

/// Immutable renderer-neutral state of one ordered grid sample.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveSampleSnapshot {
    /// Zero-based canonical sample index.
    pub index: usize,
    /// Grid X coordinate in meters.
    pub x: Meters,
    /// Transverse displacement in meters.
    pub displacement: Meters,
    /// Transverse velocity in meters per second.
    pub velocity: MetersPerSecond,
    /// Last committed transverse acceleration in meters per second squared.
    pub acceleration: MetersPerSecondSquared,
}

/// Bounded immutable state of the complete one-dimensional field.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveSnapshot {
    /// Number of successfully committed wave steps.
    pub step: StepIndex,
    /// Revision of all committed canonical field state.
    pub revision: WorldRevision,
    /// Elapsed fixed-step simulation time in seconds.
    pub elapsed: Seconds,
    /// Uniform distance between samples in meters.
    pub spacing: SampleSpacing,
    /// Positive propagation speed in meters per second.
    pub speed: WaveSpeed,
    /// Boundary rule used by the discrete solver.
    pub boundary: BoundaryCondition,
    /// Complete ordered sample grid, bounded by `MAX_WAVE_SAMPLES`.
    pub samples: Vec<WaveSampleSnapshot>,
}

/// Single bounded typed event emitted after a successful wave step.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveEvent {
    /// Step that produced this post-commit fact.
    pub step: StepIndex,
    /// Canonical world revision committed by the step.
    pub revision: WorldRevision,
    /// Elapsed simulation time after the commit, in seconds.
    pub elapsed: Seconds,
    /// Number of samples advanced by the step.
    pub sample_count: usize,
}

/// Successful atomic wave step output.
#[derive(Clone, Debug, PartialEq)]
pub struct WaveStepReport {
    /// Single bounded post-commit wave event.
    pub event: WaveEvent,
    /// Immutable field state after the same commit.
    pub snapshot: WaveSnapshot,
}
