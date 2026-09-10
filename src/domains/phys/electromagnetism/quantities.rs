use std::{error::Error, fmt};

/// Reason an electrostatic SI scalar could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElectrostaticQuantityError {
    /// NaN and either infinity are invalid canonical values.
    NonFinite {
        /// Stable SI quantity name used by diagnostics.
        quantity: &'static str,
    },
    /// A canonical point charge must carry non-zero charge.
    ZeroCharge,
}

impl fmt::Display for ElectrostaticQuantityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { quantity } => write!(formatter, "{quantity} must be finite"),
            Self::ZeroCharge => formatter.write_str("point charge must be non-zero"),
        }
    }
}

impl Error for ElectrostaticQuantityError {}

/// A finite signed non-zero electric charge in coulombs.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Coulombs(f64);

impl Coulombs {
    /// Validates a signed non-zero charge in coulombs.
    pub fn new(value: f64) -> Result<Self, ElectrostaticQuantityError> {
        if !value.is_finite() {
            Err(ElectrostaticQuantityError::NonFinite {
                quantity: "coulombs",
            })
        } else if value == 0.0 {
            Err(ElectrostaticQuantityError::ZeroCharge)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the signed charge in coulombs.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// A finite signed electric-field component in newtons per coulomb.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct NewtonsPerCoulomb(f64);

impl NewtonsPerCoulomb {
    /// Exact zero electric field.
    pub const ZERO: Self = Self(0.0);

    /// Validates a signed field component in newtons per coulomb.
    pub fn new(value: f64) -> Result<Self, ElectrostaticQuantityError> {
        if value.is_finite() {
            Ok(Self(if value == 0.0 { 0.0 } else { value }))
        } else {
            Err(ElectrostaticQuantityError::NonFinite {
                quantity: "newtons per coulomb",
            })
        }
    }

    /// Returns the component in newtons per coulomb.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Two-dimensional electric field in world X/Y coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElectricField2 {
    x: NewtonsPerCoulomb,
    y: NewtonsPerCoulomb,
}

impl ElectricField2 {
    /// Exact zero field.
    pub const ZERO: Self = Self {
        x: NewtonsPerCoulomb::ZERO,
        y: NewtonsPerCoulomb::ZERO,
    };

    /// Constructs a field from validated X/Y components.
    pub const fn new(x: NewtonsPerCoulomb, y: NewtonsPerCoulomb) -> Self {
        Self { x, y }
    }

    /// Returns the X component in newtons per coulomb.
    pub const fn x(self) -> NewtonsPerCoulomb {
        self.x
    }

    /// Returns the Y component in newtons per coulomb.
    pub const fn y(self) -> NewtonsPerCoulomb {
        self.y
    }
}
