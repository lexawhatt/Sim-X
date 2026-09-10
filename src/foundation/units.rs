use std::{error::Error, fmt};

/// Reason a scalar SI quantity could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuantityError {
    /// NaN and either infinity are invalid canonical values.
    NonFinite {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
    /// A duration may be zero, but never negative.
    Negative {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
    /// Mass and fixed simulation steps must be strictly positive.
    NonPositive {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
}

impl fmt::Display for QuantityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { quantity } => write!(formatter, "{quantity} must be finite"),
            Self::Negative { quantity } => write!(formatter, "{quantity} must not be negative"),
            Self::NonPositive { quantity } => {
                write!(formatter, "{quantity} must be strictly positive")
            }
        }
    }
}

impl Error for QuantityError {}

macro_rules! finite_quantity {
    ($name:ident, $label:literal, $documentation:literal) => {
        #[doc = $documentation]
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
        pub struct $name(f64);

        impl $name {
            /// Exact zero in this quantity's SI unit.
            pub const ZERO: Self = Self(0.0);

            /// Constructs a finite scalar value in the documented SI unit.
            pub fn new(value: f64) -> Result<Self, QuantityError> {
                if value.is_finite() {
                    Ok(Self(if value == 0.0 { 0.0 } else { value }))
                } else {
                    Err(QuantityError::NonFinite { quantity: $label })
                }
            }

            /// Returns the scalar value in the documented SI unit.
            pub const fn get(self) -> f64 {
                self.0
            }
        }
    };
}

finite_quantity!(
    Meters,
    "meters",
    "A finite signed distance component in meters."
);
finite_quantity!(
    MetersPerSecond,
    "meters per second",
    "A finite signed velocity component in meters per second."
);
finite_quantity!(
    MetersPerSecondSquared,
    "meters per second squared",
    "A finite signed acceleration component in meters per second squared."
);
finite_quantity!(
    Newtons,
    "newtons",
    "A finite signed force component in newtons."
);

/// A finite, non-negative duration in seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Seconds(f64);

impl Seconds {
    /// Zero elapsed simulation time.
    pub const ZERO: Self = Self(0.0);

    /// Constructs a finite, non-negative duration in seconds.
    pub fn new(value: f64) -> Result<Self, QuantityError> {
        if !value.is_finite() {
            Err(QuantityError::NonFinite {
                quantity: "seconds",
            })
        } else if value < 0.0 {
            Err(QuantityError::Negative {
                quantity: "seconds",
            })
        } else {
            Ok(Self(if value == 0.0 { 0.0 } else { value }))
        }
    }

    /// Returns this duration in seconds.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// A validated finite, strictly positive fixed simulation step.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct TimeStep(Seconds);

impl TimeStep {
    /// Constructs a fixed step from a number of seconds.
    pub fn new(seconds: f64) -> Result<Self, QuantityError> {
        if !seconds.is_finite() {
            Err(QuantityError::NonFinite {
                quantity: "time step",
            })
        } else if seconds <= 0.0 {
            Err(QuantityError::NonPositive {
                quantity: "time step",
            })
        } else {
            Ok(Self(Seconds(seconds)))
        }
    }

    /// Returns the step as a non-negative duration.
    pub const fn as_seconds(self) -> Seconds {
        self.0
    }

    /// Returns the fixed step in seconds.
    pub const fn get(self) -> f64 {
        self.0.get()
    }
}

/// A finite, strictly positive mass in kilograms.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Kilograms(f64);

impl Kilograms {
    /// Constructs a finite, strictly positive mass in kilograms.
    pub fn new(value: f64) -> Result<Self, QuantityError> {
        if !value.is_finite() {
            Err(QuantityError::NonFinite {
                quantity: "kilograms",
            })
        } else if value <= 0.0 {
            Err(QuantityError::NonPositive {
                quantity: "kilograms",
            })
        } else {
            Ok(Self(value))
        }
    }

    /// Returns this mass in kilograms.
    pub const fn get(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Kilograms, Meters, Seconds, TimeStep};

    #[test]
    fn finite_quantity_rejects_non_finite_values() {
        assert!(Meters::new(f64::NAN).is_err());
        assert!(Meters::new(f64::INFINITY).is_err());
        assert!(Meters::new(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn signed_zero_is_canonicalized() {
        assert_eq!(Meters::new(-0.0).expect("signed zero").get().to_bits(), 0);
        assert_eq!(Seconds::new(-0.0).expect("signed zero").get().to_bits(), 0);
    }

    #[test]
    fn elapsed_seconds_are_non_negative() {
        assert_eq!(Seconds::new(0.0).expect("zero is valid"), Seconds::ZERO);
        assert!(Seconds::new(-0.001).is_err());
    }

    #[test]
    fn mass_and_time_step_are_strictly_positive() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0] {
            assert!(Kilograms::new(invalid).is_err());
            assert!(TimeStep::new(invalid).is_err());
        }
    }
}
