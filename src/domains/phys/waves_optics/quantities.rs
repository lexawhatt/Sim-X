use std::{error::Error, fmt};

/// Reason a positive wave-model scalar could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveQuantityError {
    /// NaN and either infinity are invalid canonical values.
    NonFinite {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
    /// Grid spacing and propagation speed must be strictly positive.
    NonPositive {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
}

impl fmt::Display for WaveQuantityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { quantity } => write!(formatter, "{quantity} must be finite"),
            Self::NonPositive { quantity } => {
                write!(formatter, "{quantity} must be strictly positive")
            }
        }
    }
}

impl Error for WaveQuantityError {}

macro_rules! positive_quantity {
    ($name:ident, $label:literal, $documentation:literal) => {
        #[doc = $documentation]
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
        pub struct $name(f64);

        impl $name {
            /// Constructs a finite, strictly positive SI value.
            pub fn new(value: f64) -> Result<Self, WaveQuantityError> {
                if !value.is_finite() {
                    Err(WaveQuantityError::NonFinite { quantity: $label })
                } else if value <= 0.0 {
                    Err(WaveQuantityError::NonPositive { quantity: $label })
                } else {
                    Ok(Self(value))
                }
            }

            /// Returns the scalar value in its documented SI unit.
            pub const fn get(self) -> f64 {
                self.0
            }
        }
    };
}

positive_quantity!(
    SampleSpacing,
    "wave sample spacing",
    "Uniform distance between adjacent samples in meters."
);
positive_quantity!(
    WaveSpeed,
    "wave propagation speed",
    "Positive propagation speed in meters per second."
);
