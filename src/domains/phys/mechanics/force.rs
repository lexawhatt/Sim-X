use std::{error::Error, fmt};

use crate::foundation::EntityId;

use super::Force2;

/// Error returned when a force-source identity is invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForceSourceIdError {
    /// Zero is reserved as an invalid source sentinel.
    Zero,
}

impl fmt::Display for ForceSourceIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("force source identifiers must be non-zero")
    }
}

impl Error for ForceSourceIdError {}

/// Opaque stable identity of one semantic force source.
///
/// A source ID must not be derived from contribution slice position, render
/// order, hash iteration, or a transient widget handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ForceSourceId(u64);

impl ForceSourceId {
    /// Constructs a non-zero source identity.
    pub const fn new(raw: u64) -> Result<Self, ForceSourceIdError> {
        if raw == 0 {
            Err(ForceSourceIdError::Zero)
        } else {
            Ok(Self(raw))
        }
    }

    /// Returns the stable integer representation.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One typed force input bound to the step call that receives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceContribution {
    /// Dynamic body that should receive the force.
    pub target: EntityId,
    /// Stable semantic source used for ordering and diagnostics.
    pub source: ForceSourceId,
    /// Finite center-of-mass force in newtons.
    pub force: Force2,
}

impl ForceContribution {
    /// Constructs a one-step force contribution from validated values.
    pub const fn new(target: EntityId, source: ForceSourceId, force: Force2) -> Self {
        Self {
            target,
            source,
            force,
        }
    }
}
