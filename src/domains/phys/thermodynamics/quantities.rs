use std::{error::Error, fmt};

/// Reason a thermodynamic SI scalar could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalQuantityError {
    /// NaN and either infinity are not canonical values.
    NonFinite {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
    /// This quantity must be strictly positive.
    NonPositive {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
}

impl fmt::Display for ThermalQuantityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { quantity } => write!(formatter, "{quantity} must be finite"),
            Self::NonPositive { quantity } => {
                write!(formatter, "{quantity} must be strictly positive")
            }
        }
    }
}

impl Error for ThermalQuantityError {}

macro_rules! positive_quantity {
    ($name:ident, $label:literal, $documentation:literal) => {
        #[doc = $documentation]
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
        pub struct $name(f64);

        impl $name {
            /// Constructs a finite, strictly positive value in the documented SI unit.
            pub fn new(value: f64) -> Result<Self, ThermalQuantityError> {
                if !value.is_finite() {
                    Err(ThermalQuantityError::NonFinite { quantity: $label })
                } else if value <= 0.0 {
                    Err(ThermalQuantityError::NonPositive { quantity: $label })
                } else {
                    Ok(Self(value))
                }
            }

            /// Returns the scalar value in the documented SI unit.
            pub const fn get(self) -> f64 {
                self.0
            }
        }
    };
}

positive_quantity!(Kelvin, "kelvin", "An absolute temperature in kelvins.");
positive_quantity!(
    Joules,
    "joules",
    "A strictly positive thermal energy in joules."
);
positive_quantity!(
    JoulesPerKelvin,
    "joules per kelvin",
    "A lumped heat capacity in joules per kelvin."
);
positive_quantity!(
    WattsPerKelvin,
    "watts per kelvin",
    "A conductive thermal link conductance in watts per kelvin."
);

/// A finite signed heat power in watts.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Watts(f64);

impl Watts {
    /// Exact zero heat power.
    pub const ZERO: Self = Self(0.0);

    /// Constructs a finite signed heat power in watts.
    pub fn new(value: f64) -> Result<Self, ThermalQuantityError> {
        if value.is_finite() {
            Ok(Self(if value == 0.0 { 0.0 } else { value }))
        } else {
            Err(ThermalQuantityError::NonFinite { quantity: "watts" })
        }
    }

    /// Returns the signed heat power in watts.
    pub const fn get(self) -> f64 {
        self.0
    }
}
