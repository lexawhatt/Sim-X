use std::{error::Error, fmt};

/// Error returned when an opaque stable identifier violates its invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdError {
    /// Entity identifiers reserve zero as an invalid sentinel.
    ZeroEntityId,
}

impl fmt::Display for IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroEntityId => formatter.write_str("entity identifiers must be non-zero"),
        }
    }
}

impl Error for IdError {}

/// Opaque stable identity of one domain entity within a scene.
///
/// The value is non-zero and carries no physical or presentation meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(u64);

impl EntityId {
    /// Constructs an entity identity from its stable non-zero representation.
    ///
    /// This constructor exists for domain allocators and future persistence
    /// boundaries. Ordinary callers should obtain IDs from the owning world.
    pub const fn new(raw: u64) -> Result<Self, IdError> {
        if raw == 0 {
            Err(IdError::ZeroEntityId)
        } else {
            Ok(Self(raw))
        }
    }

    /// Returns the stable integer representation without assigning it meaning.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Monotonic identity of a successfully committed simulation step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepIndex(u64);

impl StepIndex {
    /// Initial state before any simulation step has committed.
    pub const ZERO: Self = Self(0);

    /// Constructs a step index from a persisted or tested representation.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the integer representation.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next index, or `None` if the counter is exhausted.
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

/// Monotonic revision of committed canonical world state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldRevision(u64);

impl WorldRevision {
    /// Initial empty-world revision.
    pub const ZERO: Self = Self(0);

    /// Constructs a revision from a persisted or tested representation.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the integer representation.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next revision, or `None` if the counter is exhausted.
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityId, StepIndex, WorldRevision};

    #[test]
    fn entity_id_rejects_zero() {
        assert!(EntityId::new(0).is_err());
        assert_eq!(EntityId::new(7).expect("valid ID").get(), 7);
    }

    #[test]
    fn counters_detect_exhaustion() {
        assert_eq!(StepIndex::ZERO.checked_next(), Some(StepIndex::new(1)));
        assert_eq!(
            WorldRevision::ZERO.checked_next(),
            Some(WorldRevision::new(1))
        );
        assert_eq!(StepIndex::new(u64::MAX).checked_next(), None);
        assert_eq!(WorldRevision::new(u64::MAX).checked_next(), None);
    }
}
