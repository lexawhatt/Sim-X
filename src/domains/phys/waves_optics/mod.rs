//! Bounded fixed-endpoint one-dimensional transverse waves.

mod error;
mod model;
mod quantities;
mod snapshot;
mod world;

pub use error::{WaveArithmetic, WaveError};
pub use model::{BoundaryCondition, WaveSpec};
pub use quantities::{SampleSpacing, WaveQuantityError, WaveSpeed};
pub use snapshot::{WaveEvent, WaveSampleSnapshot, WaveSnapshot, WaveStepReport};
pub use world::{MAX_WAVE_SAMPLES, MIN_WAVE_SAMPLES, WaveWorld};

#[cfg(test)]
mod tests;
