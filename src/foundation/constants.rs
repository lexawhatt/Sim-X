use std::{error::Error, fmt};

/// Schema version of the typed Sim;X physical-constants registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstantsVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl ConstantsVersion {
    /// Initial registry schema containing vacuum electric permittivity.
    pub const V1_0_0: Self = Self::new(1, 0, 0);

    /// Constructs an explicit semantic registry schema version.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Returns the breaking-change component.
    pub const fn major(self) -> u32 {
        self.major
    }

    /// Returns the compatible-addition component.
    pub const fn minor(self) -> u32 {
        self.minor
    }

    /// Returns the compatible-fix component.
    pub const fn patch(self) -> u32 {
        self.patch
    }
}

/// Provenance of the values stored in a physical-constants registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConstantsSource {
    /// NIST publication of the 2022 CODATA recommended values.
    Codata2022,
    /// A user- or project-supplied scientifically unusual value set.
    Custom,
}

/// Error returned by invalid fundamental-constant construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstantsError {
    /// NaN and either infinity are computationally invalid.
    NonFiniteVacuumPermittivity,
    /// Vacuum permittivity must be strictly positive for this registry schema.
    NonPositiveVacuumPermittivity,
}

impl fmt::Display for ConstantsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteVacuumPermittivity => {
                formatter.write_str("vacuum permittivity must be finite")
            }
            Self::NonPositiveVacuumPermittivity => {
                formatter.write_str("vacuum permittivity must be strictly positive")
            }
        }
    }
}

impl Error for ConstantsError {}

/// Positive finite vacuum electric permittivity in farads per meter.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct VacuumPermittivity(f64);

impl VacuumPermittivity {
    /// Validates a vacuum electric permittivity in farads per meter.
    pub fn new(farads_per_meter: f64) -> Result<Self, ConstantsError> {
        if !farads_per_meter.is_finite() {
            Err(ConstantsError::NonFiniteVacuumPermittivity)
        } else if farads_per_meter <= 0.0 {
            Err(ConstantsError::NonPositiveVacuumPermittivity)
        } else {
            Ok(Self(farads_per_meter))
        }
    }

    /// Returns the value in farads per meter.
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Minimal versioned scene-wide physical-constants registry.
///
/// The registry currently contains only the constant consumed by an active
/// scientific slice. Custom values are permitted but retain the same schema
/// version and explicit custom provenance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalConstants {
    version: ConstantsVersion,
    source: ConstantsSource,
    vacuum_permittivity: VacuumPermittivity,
}

impl PhysicalConstants {
    /// Returns the real-world 2022 CODATA preset.
    ///
    /// Vacuum permittivity is `8.8541878188e-12 F/m`, as published by NIST for
    /// the 2022 CODATA adjustment. Formula code must read this registry rather
    /// than duplicate the value.
    pub const fn codata_2022() -> Self {
        Self {
            version: ConstantsVersion::V1_0_0,
            source: ConstantsSource::Codata2022,
            vacuum_permittivity: VacuumPermittivity(8.854_187_818_8e-12),
        }
    }

    /// Creates registry schema 1.0.0 with a validated custom scene-wide value.
    pub const fn custom(vacuum_permittivity: VacuumPermittivity) -> Self {
        Self {
            version: ConstantsVersion::V1_0_0,
            source: ConstantsSource::Custom,
            vacuum_permittivity,
        }
    }

    /// Returns the registry schema version.
    pub const fn version(self) -> ConstantsVersion {
        self.version
    }

    /// Returns whether values are the real-world preset or a custom set.
    pub const fn source(self) -> ConstantsSource {
        self.source
    }

    /// Returns vacuum electric permittivity in farads per meter.
    pub const fn vacuum_permittivity(self) -> VacuumPermittivity {
        self.vacuum_permittivity
    }
}

#[cfg(test)]
mod tests {
    use super::{ConstantsSource, ConstantsVersion, PhysicalConstants, VacuumPermittivity};

    #[test]
    fn codata_registry_is_versioned_and_finite() {
        let constants = PhysicalConstants::codata_2022();
        assert_eq!(constants.version(), ConstantsVersion::V1_0_0);
        assert_eq!(constants.source(), ConstantsSource::Codata2022);
        assert_eq!(constants.vacuum_permittivity().get(), 8.854_187_818_8e-12);
    }

    #[test]
    fn custom_registry_rejects_computationally_invalid_values() {
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(VacuumPermittivity::new(invalid).is_err());
        }
        let custom = PhysicalConstants::custom(
            VacuumPermittivity::new(1.0e-9).expect("valid unusual constant"),
        );
        assert_eq!(custom.source(), ConstantsSource::Custom);
    }
}
