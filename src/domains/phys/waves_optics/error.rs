use std::{error::Error, fmt};

use super::WaveQuantityError;

/// Named arithmetic stage used by structured wave failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveArithmetic {
    /// Multiplication of sample spacing by the final grid index.
    GridExtent,
    /// Dimensionless `speed * dt / spacing` stability ratio.
    CourantNumber,
    /// Exact reduction of the four-term centered displacement stencil.
    StencilReduction,
    /// Scaled evaluation of `stencil * wave_speed^2 / sample_spacing^2`.
    Acceleration,
    /// Multiplication of acceleration by the fixed time step.
    VelocityIncrement,
    /// Addition of the velocity increment to canonical velocity.
    VelocityUpdate,
    /// Multiplication of next velocity by the fixed time step.
    DisplacementIncrement,
    /// Addition of the displacement increment to canonical displacement.
    DisplacementUpdate,
    /// Addition of the fixed step to elapsed simulation time.
    ElapsedTime,
}

/// Structured failure from the one-dimensional wave domain.
#[derive(Clone, Debug, PartialEq)]
pub enum WaveError {
    /// A typed wave-model scalar failed boundary validation.
    Quantity(WaveQuantityError),
    /// The proposed grid size lies outside the bounded supported interval.
    SampleCountOutOfRange {
        /// Submitted displacement sample count.
        count: usize,
        /// Smallest accepted grid size.
        minimum: usize,
        /// Largest accepted grid size.
        maximum: usize,
    },
    /// Displacement and velocity arrays describe different grids.
    SampleCountMismatch {
        /// Number of displacement samples supplied.
        displacement: usize,
        /// Number of velocity samples supplied.
        velocity: usize,
    },
    /// A fixed-zero boundary endpoint contains non-zero initial state.
    InvalidFixedEndpoint {
        /// Zero-based endpoint sample index.
        index: usize,
    },
    /// The explicit solver's Courant number exceeds its stability limit.
    CourantLimitExceeded {
        /// Computed dimensionless Courant number.
        courant_number: f64,
    },
    /// A step or revision counter cannot advance without overflow.
    CounterExhausted,
    /// Checked derived arithmetic produced NaN or infinity.
    DerivedNonFinite {
        /// Formula stage that failed.
        operation: WaveArithmetic,
        /// Affected sample index, or `None` for world-level arithmetic.
        sample: Option<usize>,
    },
    /// A non-zero derived result or update became unrepresentable.
    PrecisionLoss {
        /// Formula stage that lost representability.
        operation: WaveArithmetic,
        /// Affected sample index, or `None` for world-level arithmetic.
        sample: Option<usize>,
    },
}

impl From<WaveQuantityError> for WaveError {
    fn from(value: WaveQuantityError) -> Self {
        Self::Quantity(value)
    }
}

impl fmt::Display for WaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quantity(error) => error.fmt(formatter),
            Self::SampleCountOutOfRange {
                count,
                minimum,
                maximum,
            } => write!(
                formatter,
                "wave sample count {count} must be in {minimum}..={maximum}"
            ),
            Self::SampleCountMismatch {
                displacement,
                velocity,
            } => write!(
                formatter,
                "wave displacement count {displacement} differs from velocity count {velocity}"
            ),
            Self::InvalidFixedEndpoint { index } => write!(
                formatter,
                "fixed wave endpoint {index} must have zero displacement and velocity"
            ),
            Self::CourantLimitExceeded { courant_number } => write!(
                formatter,
                "wave Courant number {courant_number} exceeds one"
            ),
            Self::CounterExhausted => formatter.write_str("wave world counter exhausted"),
            Self::DerivedNonFinite { operation, sample } => write!(
                formatter,
                "non-finite wave result during {operation:?} at {sample:?}"
            ),
            Self::PrecisionLoss { operation, sample } => write!(
                formatter,
                "wave progress lost during {operation:?} at {sample:?}"
            ),
        }
    }
}

impl Error for WaveError {}
